//! Published development times, looked up by film, developer, dilution and EI.
//!
//! The table lives in `data/development/times.csv` (whole-stop EIs only) and is
//! compiled in. Between table rows, times are interpolated linearly in
//! log(time) against stops; beyond the table, a documented rule of thumb extends the
//! nearest row and the result is flagged as estimated.

use std::sync::OnceLock;

const TIMES_CSV: &str = include_str!("../../../data/development/times.csv");

/// Rule-of-thumb time factor per stop of push, used only outside the table.
pub const PUSH_FACTOR_PER_STOP: f64 = 1.28;
/// Rule-of-thumb time factor per stop of pull, used only outside the table.
pub const PULL_FACTOR_PER_STOP: f64 = 0.80;

#[derive(Clone, Debug, PartialEq)]
pub struct TimeRow {
    pub film: String,
    /// Developer combination id, e.g. `hc110-b`.
    pub developer: String,
    pub ei: f64,
    pub time_min: f64,
    pub temp_c: f64,
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeOrigin {
    /// A published time for exactly this EI.
    Published,
    /// Interpolated between two published times.
    Interpolated,
    /// Extended beyond the published range by the push/pull rule of thumb.
    Estimated,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeLookup {
    pub time_min: f64,
    /// Temperature the time applies at.
    pub temp_c: f64,
    pub origin: TimeOrigin,
}

/// Parse the CSV format: `film,developer,dilution,ei,time_min,temp_c,source`.
/// Lines starting with `#` and blank lines are ignored; the developer and dilution
/// columns combine into the developer combination id (`d76` + `1+1` → `d76-1+1`).
pub fn parse(csv: &str) -> Result<Vec<TimeRow>, String> {
    let mut rows = Vec::new();
    let mut lines = csv
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let header = lines.next().ok_or("empty table")?.1;
    if header.trim() != "film,developer,dilution,ei,time_min,temp_c,source" {
        return Err(format!("unexpected header: {header}"));
    }
    for (n, line) in lines {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() != 7 {
            return Err(format!("line {}: expected 7 fields", n + 1));
        }
        let num = |s: &str| {
            s.parse::<f64>()
                .map_err(|e| format!("line {}: {s:?}: {e}", n + 1))
        };
        rows.push(TimeRow {
            film: f[0].into(),
            developer: combination_id(f[1], f[2]),
            ei: num(f[3])?,
            time_min: num(f[4])?,
            temp_c: num(f[5])?,
            source: f[6].into(),
        });
    }
    Ok(rows)
}

/// Developer combination id from developer and dilution columns.
pub fn combination_id(developer: &str, dilution: &str) -> String {
    format!("{developer}-{}", dilution.to_ascii_lowercase())
}

/// The compiled-in table.
pub fn table() -> &'static [TimeRow] {
    static TABLE: OnceLock<Vec<TimeRow>> = OnceLock::new();
    TABLE.get_or_init(|| parse(TIMES_CSV).expect("data/development/times.csv is valid"))
}

/// Published rows for one film and developer combination, sorted by EI.
pub fn rows_for<'a>(rows: &'a [TimeRow], film: &str, developer: &str) -> Vec<&'a TimeRow> {
    let mut v: Vec<&TimeRow> = rows
        .iter()
        .filter(|r| r.film == film && r.developer == developer)
        .collect();
    v.sort_by(|a, b| a.ei.total_cmp(&b.ei));
    v
}

/// Development time for `ei`, or `None` when the combination has no published data.
pub fn lookup_in(rows: &[TimeRow], film: &str, developer: &str, ei: f64) -> Option<TimeLookup> {
    let r = rows_for(rows, film, developer);
    let (first, last) = (r.first()?, r.last()?);
    let stops = |e: f64| e.log2();
    // Exact (within a tenth of a stop: tables list e.g. 3200 or 3000).
    if let Some(row) = r.iter().find(|row| (stops(row.ei) - stops(ei)).abs() < 0.1) {
        return Some(TimeLookup {
            time_min: row.time_min,
            temp_c: row.temp_c,
            origin: TimeOrigin::Published,
        });
    }
    if ei < first.ei {
        let s = stops(first.ei) - stops(ei);
        return Some(TimeLookup {
            time_min: first.time_min * PULL_FACTOR_PER_STOP.powf(s),
            temp_c: first.temp_c,
            origin: TimeOrigin::Estimated,
        });
    }
    if ei > last.ei {
        let s = stops(ei) - stops(last.ei);
        return Some(TimeLookup {
            time_min: last.time_min * PUSH_FACTOR_PER_STOP.powf(s),
            temp_c: last.temp_c,
            origin: TimeOrigin::Estimated,
        });
    }
    let i = r.iter().position(|row| row.ei > ei)?;
    let (a, b) = (r[i - 1], r[i]);
    let t = (stops(ei) - stops(a.ei)) / (stops(b.ei) - stops(a.ei));
    Some(TimeLookup {
        time_min: (a.time_min.ln() + t * (b.time_min.ln() - a.time_min.ln())).exp(),
        temp_c: a.temp_c,
        origin: TimeOrigin::Interpolated,
    })
}

/// [`lookup_in`] against the compiled-in table.
pub fn lookup(film: &str, developer: &str, ei: f64) -> Option<TimeLookup> {
    lookup_in(table(), film, developer, ei)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# comment\nfilm,developer,dilution,ei,time_min,temp_c,source\n\
        f,dev,B,400,6,20,x\nf,dev,B,1600,12,20,x\nf,dev,B,800,8,20,x\n";

    #[test]
    fn parses_and_combines_ids() {
        let rows = parse(SAMPLE).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].developer, "dev-b");
    }

    #[test]
    fn lookup_exact_interpolated_and_estimated() {
        let rows = parse(SAMPLE).unwrap();
        let at = |ei| lookup_in(&rows, "f", "dev-b", ei).unwrap();
        assert_eq!(at(800.0).origin, TimeOrigin::Published);
        assert_eq!(at(800.0).time_min, 8.0);
        let mid = at(1131.0); // half a stop between 800 and 1600
        assert_eq!(mid.origin, TimeOrigin::Interpolated);
        assert!((mid.time_min - (8.0f64 * 12.0).sqrt()).abs() < 0.05);
        let pull = at(200.0);
        assert_eq!(pull.origin, TimeOrigin::Estimated);
        assert!((pull.time_min - 6.0 * PULL_FACTOR_PER_STOP).abs() < 1e-9);
        assert!(lookup_in(&rows, "f", "other", 400.0).is_none());
    }

    #[test]
    fn compiled_table_is_valid() {
        assert!(!table().is_empty());
        for row in table() {
            assert!(row.time_min > 0.0 && row.ei > 0.0, "{row:?}");
        }
    }
}
