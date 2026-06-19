//! Clean and validate the raw model output before it reaches lazygit.
//!
//! The model is told to emit exactly `N` lines in the form `1. <message>`. In
//! practice models add preambles, postambles, <think> blocks, repeat lines, or
//! sometimes only return 2 of the 3 requested items. Lazygit's menuFromCommand
//! does a regex line match and creates one entry per match — so any extra line
//! with `<digit>. <text>` becomes a phantom menu option.
//!
//! `extract_commits` enforces the contract: the output is *always* at most `n`
//! clean, deduplicated, non-empty lines of the form `<i>. <message>`. Any
//! remaining slack (model returned fewer than `n`) is the caller's problem to
//! surface — but at least the lazygit menu will never have stale/empty/junk
//! entries.

/// Strip everything the model might have added that isn't a numbered line,
/// keep only the first `n` unique non-empty matches, and re-number them 1..=k.
///
/// Returns the lines WITHOUT a trailing newline. Callers add the final `\n` if
/// they want one per line.
pub fn extract_commits(raw: &str, n: usize) -> Vec<String> {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::with_capacity(n);

    for line in raw.lines() {
        if let Some(msg) = parse_numbered_line(line) {
            let trimmed = msg.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Case-insensitive dedup — "✨ Add foo" and "✨ add foo" are the same
            // intent from the menu's perspective.
            let key = trimmed.to_lowercase();
            if seen.insert(key) {
                out.push(trimmed.to_string());
                if out.len() == n {
                    break;
                }
            }
        }
    }
    out
}

/// If `line` starts with `<digits>. ` (or `<digits>)<space>`), return the
/// message portion with the prefix removed. Otherwise None.
///
/// Anchored to the very start of the (trimmed) line and tolerates only
/// whitespace before the number. Crucially it does NOT accept the
/// `digits.digits.` pattern you see in dates (`2024. 01. 15`) — that pattern
/// would otherwise sneak into the lazygit menu as a phantom commit.
///
/// Permissive on whitespace and bullet form so the function is robust to
/// model quirks like " 1. foo" or "1) foo" — but anchored to a digit prefix
/// because that's exactly what lazygit's `menuFromCommand` filter matches.
fn parse_numbered_line(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let bytes = line.as_bytes();

    // Walk over leading digits.
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= bytes.len() {
        return None;
    }
    let sep = bytes[i];
    if sep == b')' {
        i += 1;
    } else if sep == b'.' {
        i += 1;
    } else {
        return None;
    }
    // The separator must be followed by a space OR end-of-line. A digit
    // (`2024.01`) or another `.` (`1.foo`) right after means this is not a
    // list item.
    if i < bytes.len() && bytes[i] != b' ' {
        return None;
    }
    if i < bytes.len() {
        i += 1; // consume the space
    }
    // Reject "2024. 01. 15" — the first token after the prefix must NOT
    // start with a digit. If it does, we are still inside a date/decimal
    // sequence, not at the start of a commit message.
    if i < bytes.len() && bytes[i].is_ascii_digit() {
        return None;
    }
    Some(&line[i..])
}

/// Format the cleaned list as the lazygit menu contract: one `N. <msg>` per
/// line, no trailing newline (lazygit handles line boundaries).
pub fn format_for_lazygit(commits: &[String]) -> String {
    commits
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{}. {}", i + 1, m))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let raw = "1. feat: add foo\n2. fix: bar\n3. refactor: baz\n";
        assert_eq!(
            extract_commits(raw, 3),
            vec![
                "feat: add foo".to_string(),
                "fix: bar".to_string(),
                "refactor: baz".to_string(),
            ]
        );
    }

    #[test]
    fn strips_think_blocks_and_preamble() {
        let raw = r#"<think>The user wants 3 commit messages. Let me think...</think>

Here are 3 suggestions:

1. ✨ add foo
2. 🐛 fix bar
3. 📝 update docs

Let me know if you need more!"#;
        assert_eq!(
            extract_commits(raw, 3),
            vec![
                "✨ add foo".to_string(),
                "🐛 fix bar".to_string(),
                "📝 update docs".to_string(),
            ]
        );
    }

    #[test]
    fn dedups_repeats() {
        let raw = "1. ✨ add foo\n2. ✨ add foo\n3. ✨ ADD FOO\n4. 🐛 fix bar\n";
        assert_eq!(
            extract_commits(raw, 3),
            vec!["✨ add foo".to_string(), "🐛 fix bar".to_string()]
        );
    }

    #[test]
    fn respects_n_cap() {
        let raw = "1. a\n2. b\n3. c\n4. d\n5. e\n";
        assert_eq!(extract_commits(raw, 3).len(), 3);
    }

    #[test]
    fn fewer_than_n_is_ok() {
        let raw = "1. only one\n";
        assert_eq!(extract_commits(raw, 3), vec!["only one".to_string()]);
    }

    #[test]
    fn ignores_non_numbered_lines() {
        // Lines that LOOK numbered but don't match (e.g. dates "2024. 01. 15")
        // are exactly the kind of thing that would otherwise sneak into the
        // lazygit menu — make sure we filter them out.
        let raw = "2024. 01. 15 — release notes\n1. ✨ add foo\n";
        assert_eq!(extract_commits(raw, 3), vec!["✨ add foo".to_string()]);
        // Decimals like "1.5 cups" are also rejected.
        let raw2 = "1.5 cups of flour\n1. ✨ add foo\n";
        assert_eq!(extract_commits(raw2, 3), vec!["✨ add foo".to_string()]);
        // Trailing-dot with no space ("1.foo") is also rejected — looks like
        // a list item but isn't a real commit message format.
        let raw3 = "1.foo bar\n1. ✨ add foo\n";
        assert_eq!(extract_commits(raw3, 3), vec!["✨ add foo".to_string()]);
    }

    #[test]
    fn handles_paren_separator() {
        let raw = "1) feat: a\n2) feat: b\n3) feat: c\n";
        assert_eq!(extract_commits(raw, 3).len(), 3);
    }

    #[test]
    fn format_for_lazygit_renumbers() {
        let commits = vec!["✨ add foo".to_string(), "🐛 fix bar".to_string()];
        assert_eq!(format_for_lazygit(&commits), "1. ✨ add foo\n2. 🐛 fix bar");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(extract_commits("", 3).is_empty());
        assert!(extract_commits("<think>only thoughts</think>", 3).is_empty());
    }

    #[test]
    fn leading_whitespace_tolerated() {
        let raw = "  1. ✨ add foo\n   2. 🐛 fix bar\n";
        assert_eq!(extract_commits(raw, 3).len(), 2);
    }
}
