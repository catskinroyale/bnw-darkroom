//! Built-in and user styles. User styles persist as JSON in the platform config
//! directory (`~/Library/Application Support/bw-darkroom/styles.json` on macOS).

use std::path::PathBuf;

use bw_profiles::{Style, builtin_styles};

pub struct StyleEntry {
    pub style: Style,
    pub builtin: bool,
}

pub struct StyleStore {
    pub entries: Vec<StyleEntry>,
    path: Option<PathBuf>,
}

impl StyleStore {
    pub fn load() -> Self {
        let path = dirs::config_dir().map(|d| d.join("bw-darkroom").join("styles.json"));
        let mut entries: Vec<StyleEntry> = builtin_styles()
            .into_iter()
            .map(|style| StyleEntry {
                style,
                builtin: true,
            })
            .collect();
        if let Some(user) = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Vec<Style>>(&s).ok())
        {
            entries.extend(user.into_iter().map(|style| StyleEntry {
                style,
                builtin: false,
            }));
        }
        Self { entries, path }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let user: Vec<&Style> = self
            .entries
            .iter()
            .filter(|e| !e.builtin)
            .map(|e| &e.style)
            .collect();
        std::fs::write(path, serde_json::to_string_pretty(&user)?)
    }

    pub fn styles(&self) -> Vec<Style> {
        self.entries.iter().map(|e| e.style.clone()).collect()
    }

    /// A name not used by any existing style.
    pub fn unique_name(&self, base: &str) -> String {
        let taken = |n: &str| self.entries.iter().any(|e| e.style.name == n);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|i| format!("{base} ({i})"))
            .find(|n| !taken(n))
            .expect("unbounded search")
    }

    /// Copy style `index` as a new user style named "<name> copy", placed after it
    /// (after the built-ins when the original is built in). Returns the new index.
    pub fn duplicate(&mut self, index: usize) -> usize {
        let mut style = self.entries[index].style.clone();
        style.name = self.unique_name(&format!("{} copy", style.name));
        let first_user = self
            .entries
            .iter()
            .position(|e| !e.builtin)
            .unwrap_or(self.entries.len());
        let at = (index + 1).max(first_user);
        self.entries.insert(
            at,
            StyleEntry {
                style,
                builtin: false,
            },
        );
        at
    }

    /// Rename a user style. Names must be non-empty and unique.
    pub fn rename(&mut self, index: usize, name: &str) -> Result<(), String> {
        let name = name.trim();
        if self.entries[index].builtin {
            return Err("Built-in styles can't be renamed: duplicate it first.".into());
        }
        if name.is_empty() {
            return Err("Enter a name.".into());
        }
        if self
            .entries
            .iter()
            .enumerate()
            .any(|(i, e)| i != index && e.style.name == name)
        {
            return Err("Another style already has that name.".into());
        }
        self.entries[index].style.name = name.to_string();
        Ok(())
    }

    /// Delete a user style.
    pub fn delete(&mut self, index: usize) -> Result<StyleEntry, String> {
        if self.entries[index].builtin {
            return Err("Built-in styles can't be deleted.".into());
        }
        Ok(self.entries.remove(index))
    }

    /// Whether user style `index` can move one place by `delta` (−1 up, +1 down)
    /// without crossing into the built-ins or off the end.
    pub fn can_move(&self, index: usize, delta: isize) -> bool {
        let target = index as isize + delta;
        !self.entries[index].builtin
            && target >= 0
            && (target as usize) < self.entries.len()
            && !self.entries[target as usize].builtin
    }

    /// Move a user style one place up or down. Returns its new index.
    pub fn move_by(&mut self, index: usize, delta: isize) -> Option<usize> {
        if !self.can_move(index, delta) {
            return None;
        }
        let target = (index as isize + delta) as usize;
        self.entries.swap(index, target);
        Some(target)
    }

    /// Add styles read from a file as user styles, renaming any that clash.
    /// Returns the index of the first one added.
    pub fn import(&mut self, styles: Vec<Style>) -> Option<usize> {
        let first = self.entries.len();
        for mut style in styles {
            style.name = self.unique_name(&style.name);
            self.entries.push(StyleEntry {
                style,
                builtin: false,
            });
        }
        (self.entries.len() > first).then_some(first)
    }
}

/// Parse a style file: one style or a list of styles (JSON).
pub fn parse_style_file(text: &str) -> Result<Vec<Style>, String> {
    serde_json::from_str::<Vec<Style>>(text)
        .or_else(|_| serde_json::from_str::<Style>(text).map(|s| vec![s]))
        .map_err(|e| format!("Not a style file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> StyleStore {
        let mut entries: Vec<StyleEntry> = builtin_styles()
            .into_iter()
            .take(2)
            .map(|style| StyleEntry {
                style,
                builtin: true,
            })
            .collect();
        let mut mine = entries[0].style.clone();
        mine.name = "Mine".into();
        entries.push(StyleEntry {
            style: mine,
            builtin: false,
        });
        StyleStore {
            entries,
            path: None,
        }
    }

    #[test]
    fn duplicate_goes_after_builtins_with_unique_name() {
        let mut s = store();
        let name = s.entries[0].style.name.clone();
        let i = s.duplicate(0);
        assert_eq!(i, 2, "after the built-ins");
        assert_eq!(s.entries[i].style.name, format!("{name} copy"));
        assert!(!s.entries[i].builtin);
        let j = s.duplicate(0);
        assert_eq!(s.entries[j].style.name, format!("{name} copy (2)"));
        let k = s.duplicate(3);
        assert_eq!(k, 4, "user styles duplicate in place");
    }

    #[test]
    fn rename_and_delete_rules() {
        let mut s = store();
        assert!(s.rename(0, "x").is_err(), "built-in");
        assert!(s.rename(2, "  ").is_err(), "empty");
        let taken = s.entries[1].style.name.clone();
        assert!(s.rename(2, &taken).is_err(), "duplicate");
        s.rename(2, "  Darker ").unwrap();
        assert_eq!(s.entries[2].style.name, "Darker");
        assert!(s.delete(0).is_err());
        assert_eq!(s.delete(2).unwrap().style.name, "Darker");
        assert_eq!(s.entries.len(), 2);
    }

    #[test]
    fn user_styles_reorder_without_crossing_builtins() {
        let mut s = store();
        s.duplicate(2);
        assert!(!s.can_move(2, -1), "cannot move above the built-ins");
        assert_eq!(s.move_by(2, 1), Some(3));
        assert_eq!(s.entries[3].style.name, "Mine");
        assert_eq!(s.move_by(3, 1), None, "already last");
        assert_eq!(s.move_by(0, 1), None, "built-ins don't move");
    }

    #[test]
    fn import_accepts_one_or_many_and_renames_clashes() {
        let mut s = store();
        let one = serde_json::to_string(&s.entries[2].style).unwrap();
        let first = s.import(parse_style_file(&one).unwrap()).unwrap();
        assert_eq!(s.entries[first].style.name, "Mine (2)");
        let many = serde_json::to_string(&vec![
            s.entries[0].style.clone(),
            s.entries[1].style.clone(),
        ])
        .unwrap();
        assert_eq!(parse_style_file(&many).unwrap().len(), 2);
        assert!(parse_style_file("{\"nope\": 1}").is_err());
    }
}
