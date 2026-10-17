//! Strict line-hunk application for diagnostic overlays. Context mismatches
//! stop compilation; instrumentation must never silently drift to another site.
use std::collections::BTreeMap;

#[derive(Default)]
struct Hunk {
    start: usize,
    lines: Vec<(char, String)>,
}

pub(crate) fn apply(path: &str, source: &str, patches: &[&str]) -> Result<String, String> {
    let mut files: BTreeMap<String, Vec<Hunk>> = BTreeMap::new();
    for patch in patches {
        let mut file = None;
        for line in patch.lines() {
            if let Some(name) = line.strip_prefix("+++ b/") {
                file = Some(name.to_owned());
            } else if line.starts_with("@@ ") {
                let name = file.as_ref().ok_or("hunk without file")?;
                let range = line.split_whitespace().nth(1).ok_or("missing old range")?;
                let start = range
                    .trim_start_matches('-')
                    .split(',')
                    .next()
                    .ok_or("missing start")?
                    .parse::<usize>()
                    .map_err(|e| e.to_string())?;
                files.entry(name.clone()).or_default().push(Hunk {
                    start: start.saturating_sub(1),
                    lines: Vec::new(),
                });
            } else if !line.starts_with("--- ")
                && let Some(name) = &file
                && let Some(hunk) = files.get_mut(name).and_then(|h| h.last_mut())
            {
                // Empty context lines omit their one-space prefix, keeping
                // tracked overlay files free of trailing whitespace.
                match line.chars().next() {
                    Some(kind @ (' ' | '+' | '-')) => hunk.lines.push((kind, line[1..].to_owned())),
                    None => hunk.lines.push((' ', String::new())),
                    _ => {}
                }
            }
        }
    }
    let Some(hunks) = files.get(path) else {
        return Ok(source.to_owned());
    };
    let old: Vec<_> = source.lines().collect();
    let mut out = Vec::new();
    let mut cursor = 0;
    for hunk in hunks {
        if hunk.start < cursor || hunk.start > old.len() {
            return Err(format!("{path}: invalid hunk offset {}", hunk.start));
        }
        out.extend(
            old[cursor..hunk.start]
                .iter()
                .map(|line| (*line).to_owned()),
        );
        cursor = hunk.start;
        for (kind, line) in &hunk.lines {
            if *kind != '+' {
                if old.get(cursor).copied() != Some(line.as_str()) {
                    return Err(format!(
                        "{path}: stale diagnostic context at line {}",
                        cursor + 1
                    ));
                }
                cursor += 1;
            }
            if *kind != '-' {
                out.push(line.clone());
            }
        }
    }
    out.extend(old[cursor..].iter().map(|line| (*line).to_owned()));
    Ok(out.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::apply;
    #[test]
    fn applies_multiple_files_and_hunks_without_losing_context() {
        let patch = "--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,3 @@\n a\n+x\n b\n@@ -4,1 +5,1 @@\n-d\n+D\n--- a/src/b.rs\n+++ b/src/b.rs\n@@ -1 +1 @@\n-q\n+Q\n";
        assert_eq!(
            apply("src/a.rs", "a\nb\nc\nd\n", &[patch]).unwrap(),
            "a\nx\nb\nc\nD\n"
        );
        assert_eq!(apply("src/b.rs", "q\n", &[patch]).unwrap(), "Q\n");
        assert!(apply("src/a.rs", "WRONG\nb\nc\nd\n", &[patch]).is_err());
        assert_eq!(
            apply(
                "src/blank.rs",
                "a\n\nb\n",
                &["--- a/src/blank.rs\n+++ b/src/blank.rs\n@@ -1,3 +1,3 @@\n a\n\n-b\n+B\n"]
            )
            .unwrap(),
            "a\n\nB\n"
        );
        assert_eq!(
            apply("src/other.rs", "untouched\n", &[patch]).unwrap(),
            "untouched\n"
        );
    }
}
