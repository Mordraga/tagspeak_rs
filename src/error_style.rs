const COLOR_RESET: &str = "\x1b[0m";
const COLOR_HEADER: &str = "\x1b[38;5;220m";
const COLOR_DETAIL: &str = "\x1b[38;5;203m";
const COLOR_LOCATION: &str = "\x1b[38;5;81m";
const COLOR_HINT: &str = "\x1b[38;5;111m";
const COLOR_SNIPPET: &str = "\x1b[38;5;250m";
const COLOR_POINTER: &str = "\x1b[38;5;214m";

pub fn render_error_box(
    line_no: usize,
    col: usize,
    snippet: &str,
    hint: &str,
    detail: &str,
) -> String {
    let prefix = "✍️  ";
    let prefix_width = content_width(prefix);
    let text_line = format!("{prefix}{snippet}");
    let pointer_line = format!(
        "{}{}↑",
        " ".repeat(prefix_width),
        " ".repeat(col.saturating_sub(1))
    );

    let lines = [
        (format!("❌  {detail}"), COLOR_DETAIL),
        (format!("📍 Line {line_no}, Column {col}"), COLOR_LOCATION),
        (String::new(), COLOR_RESET),
        (text_line, COLOR_SNIPPET),
        (pointer_line, COLOR_POINTER),
        (format!("💡  {hint}"), COLOR_HINT),
    ];

    let inner_width = lines
        .iter()
        .map(|(line, _)| content_width(line))
        .max()
        .unwrap_or(0);

    let header = format!("╭─{:─^width$}─╮", " TagSpeak Error ", width = inner_width);
    let footer = format!("╰{:─^width$}╯", "", width = inner_width + 2);

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
    if detail.contains("unterminated string") {
        "Your [\"] is looking a little less... stringy than it should. Want your other [\"]? "
            .to_string()
    } else if detail.contains("unbalanced [ ... ]") {
        "A matching ']' would make this perfect. <3".to_string()
    } else if detail.contains("empty packet op") {
        "Did you forget to add in an argument? Arguing with [] is half the fun. :D".to_string()
    } else if detail.contains("expected opener") {
        "Looks like we need an opening bracket friend here!".to_string()
    } else {
        "Something feels off—let's peek at those brackets together.".to_string()
    }
}

pub fn unexpected_hint(ch: char, where_: &str) -> String {
    match (where_, ch) {
        ("top-level", _) => {
            "Packets begin with '[' — let's tuck one in right before this darling.".to_string()
        }
        (_, ']') => "Found a ']' without its partner. Maybe invite a '[' to the party?".to_string(),
        (_, '}') => {
            "This '}' is closing early. Peek around to see if a '{' wandered off.".to_string()
        }
        _ => format!("{ch} feels unexpected here—let's give it some company."),
    }
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
