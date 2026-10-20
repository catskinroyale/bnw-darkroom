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
}
