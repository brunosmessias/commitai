/// Output style requested by the user. The system prompt is tailored to each
/// so the model emits a predictable, parseable response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// `<gitmoji> <description>`, e.g. "✨ add login screen".
    Gitmoji,
    /// `<type>: <description>` (conventional commits), e.g. "feat: add login".
    Conventional,
    /// Use whatever style the user's custom template asks for — we don't add
    /// a style section of our own, the user's template is authoritative.
    Custom,
}

impl Style {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "gitmoji" => Some(Self::Gitmoji),
            "conventional" | "conv" => Some(Self::Conventional),
            "custom" | "auto" => Some(Self::Custom),
            _ => None,
        }
    }
}

fn gitmoji_rules() -> &'static str {
    r#"OUTPUT STYLE: gitmoji
- Each line starts with a single gitmoji followed by a short imperative description.
- Example: "✨ add login screen"
- Use the most specific gitmoji that fits (🐛 for bugs, not 🔧; 💄 for UI, not ✨).
- Hard rules: English only, max 150 chars per line, no body, no co-author/agent attribution, no "Generated with" or similar."#
}

fn conventional_rules() -> &'static str {
    r#"OUTPUT STYLE: conventional commits
- Each line is `<type>(<optional scope>): <description>` where type is one of:
  feat, fix, docs, style, refactor, perf, test, chore, build, ci, revert.
- Example: "feat(auth): add login screen"
- Hard rules: English only, max 150 chars per line, no body, no co-author/agent attribution, no "Generated with" or similar."#
}

fn custom_rules() -> &'static str {
    r#"OUTPUT STYLE: follow the user's request in the user message exactly.
- Hard rules: English only, max 150 chars per line, no body, no co-author/agent attribution, no "Generated with" or similar."#
}

/// Build the system prompt for the chat-completion call.
///
/// The contract here is the foundation of the whole tool: the model is told
/// that its ONLY valid response is exactly N numbered lines, period. No
/// preambles, no <think>, no "here you go:", no apologies, no follow-up
/// offers. The post-processor in `output.rs` enforces this on the parsing side
/// as a safety net, but a tight system prompt dramatically reduces the
/// amount of junk we have to clean up — and saves tokens.
pub fn build_system_prompt(n: usize, style: Style) -> String {
    let style_rules = match style {
        Style::Gitmoji => gitmoji_rules(),
        Style::Conventional => conventional_rules(),
        Style::Custom => custom_rules(),
    };

    format!(
        r#"You generate commit messages. You obey these rules STRICTLY:

1. You will receive a git diff in the user message.
2. You MUST reply with exactly {n} lines, no more, no fewer.
3. Line `i` (for i in 1..={n}) MUST be in the form: `<i>. <commit message>`
4. NOTHING else in your reply. No preambles, no explanations, no "Here are the commits:", no follow-up offers, no apologies, no closing remarks.
5. No markdown code fences, no bullet points other than the numbered prefix, no empty lines.
6. Do NOT include a <think> block. Do NOT include any chain-of-thought. Output the final lines only.

{style_rules}
"#
    )
}

/// Build the user message. We render the user's template if one is configured,
/// then we APPEND a hard "respond with exactly N lines" reminder — because in
/// practice models pay more attention to instructions in the last user turn
/// than to system prompts.
pub fn build_user_prompt(template_body: &str, diff: &str, n: usize) -> String {
    let body = template_body.replace("{{diff}}", diff);
    // The trailing reminder is a no-op for compliant models and a life-saver
    // for chatty ones.
    format!(
        "{body}\n\n---\nREMINDER: Reply with EXACTLY {n} lines, each in the form `<i>. <commit message>`. Nothing else. No preambles, no closings, no markdown.\n"
    )
}

/// Legacy template body — kept only as a seed for first-run wizard and the
/// `default.txt` that gets written on install. The runtime no longer uses
/// this directly: see `build_user_prompt` for the actual user message.
pub const DEFAULT_TEMPLATE: &str = r#"Please suggest 3 commit messages, given the following diff:

```diff
{{diff}}
```

**Criteria:**

1. **Format:** Each commit message must follow the conventional commits format, which is `<type>: <description>`.
2. **Enumeration:** List the commit messages from 1 to 3.
3. **Clarity and Conciseness:** Each message should clearly and concisely convey the change made.

Write your 3 commit messages below, in the format `1. <message>`, `2. <message>`, etc.:
"#;
