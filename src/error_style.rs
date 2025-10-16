const COLOR_RESET: &str = "\x1b[0m";
const COLOR_HEADER: &str = "\x1b[38;5;220m";
const COLOR_DETAIL: &str = "\x1b[38;5;203m";
const COLOR_LOCATION: &str = "\x1b[38;5;81m";
const COLOR_HINT: &str = "\x1b[38;5;111m";
const COLOR_SNIPPET: &str = "\x1b[38;5;250m";
const COLOR_POINTER: &str = "\x1b[38;5;214m";

const GLYPH_DETAIL: &str = "❌";
const GLYPH_LOCATION: &str = "📍";
const GLYPH_HINT: &str = "💡";
const GLYPH_PREFIX: &str = "✍️  ";
const GLYPH_POINTER: &str = "↑";

pub fn render_error_box(
    line_no: usize,
    col: usize,
    snippet: &str,
    hint: &str,
    detail: &str,
) -> String {
    let prefix_width = content_width(GLYPH_PREFIX);
    let text_line = format!("{GLYPH_PREFIX}{snippet}");
    let pointer_line = format!(
        "{}{}{}",
        " ".repeat(prefix_width),
        " ".repeat(col.saturating_sub(1)),
        GLYPH_POINTER
    );

    let lines = [
        (format!("{GLYPH_DETAIL}  {detail}"), COLOR_DETAIL),
        (
            format!("{GLYPH_LOCATION} Line {line_no}, Column {col}"),
            COLOR_LOCATION,
        ),
        (String::new(), COLOR_RESET),
        (text_line, COLOR_SNIPPET),
        (pointer_line, COLOR_POINTER),
        (format!("{GLYPH_HINT}  {hint}"), COLOR_HINT),
    ];

    let inner_width = lines
        .iter()
        .map(|(line, _)| content_width(line))
        .max()
        .unwrap_or(0);

    let title = format!(" {} ", "TagSpeak Error");
    let header = format!("╭{:─^width$}╮", title, width = inner_width);
    let footer = format!("╰{:─^width$}╯", "", width = inner_width);

    let mut out = String::new();
    out.push_str(&colorize(&header, COLOR_HEADER));
    out.push('\n');
    for (line, color) in &lines {
        out.push_str(&colorize(&pad_line_no_border(line, inner_width), color));
        out.push('\n');
    }
    out.push_str(&colorize(&footer, COLOR_HEADER));
    out
}

pub fn friendly_hint(detail: &str) -> String {
    let (category, message) = classify_detail(detail);
    format!("{} - {}", category.label(), message)
}

pub fn unexpected_hint(ch: char, where_: &str) -> String {
    let (category, message) = classify_unexpected(ch, where_);
    format!("{} - {}", category.label(), message)
}

fn content_width(s: &str) -> usize {
    s.chars().count()
}

fn pad_line_no_border(content: &str, width: usize) -> String {
    let mut line = String::from(content);
    let used = content_width(content);
    if used < width {
        line.push_str(&" ".repeat(width - used));
    }
    line
}

fn colorize(line: &str, color: &str) -> String {
    format!("{color}{line}{COLOR_RESET}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HintCategory {
    Delimiter,
    Packet,
    Variable,
    Syntax,
}

impl HintCategory {
    fn label(self) -> &'static str {
        match self {
            HintCategory::Delimiter => "Delimiter",
            HintCategory::Packet => "Packet",
            HintCategory::Variable => "Variable",
            HintCategory::Syntax => "Syntax",
        }
    }
}

fn classify_detail(detail: &str) -> (HintCategory, String) {
    let lower = detail.to_ascii_lowercase();
    if let Some(msg) = delimiter_hint(detail, &lower) {
        return (HintCategory::Delimiter, msg);
    }
    if let Some(msg) = packet_hint(&lower) {
        return (HintCategory::Packet, msg);
    }
    if let Some(msg) = variable_hint(&lower) {
        return (HintCategory::Variable, msg);
    }
    (
        HintCategory::Syntax,
        "Something’s a little out of place. Deep breath. Let’s sort out the syntax together."
            .to_string(),
    )
}

fn classify_unexpected(ch: char, where_: &str) -> (HintCategory, String) {
    match (where_, ch) {
        (_, ']') => (
            HintCategory::Delimiter,
            "A ']' without a '['? That’s like a hug with no arms — pair it up properly."
                .to_string(),
        ),
        (_, '}') => (
            HintCategory::Delimiter,
            "Ending a block that never started? Try adding a '{' before it to keep things tidy."
                .to_string(),
        ),
        ("top-level", '@') => (
            HintCategory::Syntax,
            "Using '@' on its own? Start with a packet like [print@...] to give it a home."
                .to_string(),
        ),
        ("top-level", c) if c.is_ascii_alphanumeric() => (
            HintCategory::Syntax,
            "Loose literal text? Wrap it in [msg@...] or comment it out to keep things clean."
                .to_string(),
        ),
        _ => (
            HintCategory::Syntax,
            format!("'{ch}' doesn’t belong here. Let’s double-check the structure and try again."),
        ),
    }
}

fn delimiter_hint(detail: &str, lower: &str) -> Option<String> {
    if lower.contains("unterminated string") {
        return Some(
            "Opened a quote but didn’t close it. Pop in the missing '\"' to finish the thought."
                .to_string(),
        );
    }
    if lower.contains("unbalanced [ ... ]") {
        return Some(
            "Started a packet and forgot the ']'. Let’s add the closing bracket to complete the pair."
                .to_string(),
        );
    }
    if lower.contains("unbalanced { ... }") {
        return Some(
            "Looks like a block is missing its '}'. Let’s close it up so nothing leaks."
                .to_string(),
        );
    }
    if lower.contains("expected opener") {
        return Some("Trying to start a packet? Use '[' to open it cleanly.".to_string());
    }
    if lower.contains("extra closing") {
        let culprit = quoted_char(detail).unwrap_or(']');
        return Some(format!(
            "Found an extra '{culprit}'. Remove it or pair it with an opener to balance things out."
        ));
    }
    if lower.contains("unexpected character") && lower.contains("']'") {
        return Some(
            "This ']' doesn’t match anything. Add a '[' before it, or drop it entirely."
                .to_string(),
        );
    }
    None
}

fn packet_hint(lower: &str) -> Option<String> {
    if lower.contains("empty packet op") {
        return Some(
            "Looks like you've got an empty []. Add an op like [print@...] or remove it if it's not needed."
                .to_string(),
        );
    }
    if lower.contains("needs (cond)") {
        return Some(
            "That condition's missing a (cond). Try @(x > 0) or another valid expression."
                .to_string(),
        );
    }
    if lower.contains("expected [then]") {
        return Some(
            "Missing a [then]{...} block after your condition. Let’s add it to complete the flow."
                .to_string(),
        );
    }
    if lower.contains("expected [else]") {
        return Some(
            "Looks like you were going for an [else] branch. Add it or remove the trailing logic."
                .to_string(),
        );
    }
    if lower.contains("unknown operation") {
        return Some(
            "That op isn’t recognized. Maybe it’s a typo or missing its module prefix?".to_string(),
        );
    }
    None
}

fn variable_hint(lower: &str) -> Option<String> {
    if lower.contains("unknown funct") || lower.contains("unknown tag") {
        return Some("Define it with [funct@name]{...} before calling it.".to_string());
    }
    if lower.contains("variable") && lower.contains("not found") {
        return Some(
            "You're using a variable that hasn’t been declared yet. Give it a name and value first."
                .to_string(),
        );
    }
    if lower.contains("invalid variable name") {
        return Some(
            "That variable name doesn’t follow syntax rules. Try something alphanumeric and clean."
                .to_string(),
        );
    }
    if lower.contains("shadowed variable") {
        return Some(
            "This variable name is already taken. Rename it to avoid confusion.".to_string(),
        );
    }
    if lower.contains("undeclared variable") {
        return Some(
            "You're calling a variable before it exists. Pop in a [store@name]{...} first."
                .to_string(),
        );
    }
    None
}

fn quoted_char(detail: &str) -> Option<char> {
    let start = detail.find('\'')?;
    let rest = &detail[start + 1..];
    let end = rest.find('\'')?;
    rest[..end].chars().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_hint_marks_delimiter() {
        let hint = friendly_hint("unbalanced [ ... ] before 40:1");
        assert!(
            hint.starts_with("Delimiter"),
            "expected Delimiter hint, got: {hint}"
        );
    }

    #[test]
    fn friendly_hint_marks_packet() {
        let hint = friendly_hint("if needs (cond) or @(cond)");
        assert!(
            hint.starts_with("Packet"),
            "expected Packet hint, got: {hint}"
        );
    }

    #[test]
    fn unexpected_hint_marks_syntax() {
        let hint = unexpected_hint('p', "top-level");
        assert!(
            hint.starts_with("Syntax"),
            "expected Syntax hint, got: {hint}"
        );
    }
}
