pub fn run(input: &str) -> Option<String> {
    let input = input.replace(" ", ""); // strip spaces
    let mut chars = input.chars();

    let mut lhs = String::new();
    let mut op = None;

    while let Some(c) = chars.next() {
        if c.is_digit(10) || c == '.' {
            lhs.push(c);
        } else {
            op = Some(c);
            break;
        }
    }

    let rhs: String = chars.collect();

    let lhs_val = lhs.parse::<f64>().ok()?;
    let rhs_val = rhs.parse::<f64>().ok()?;

    let result = match op? {
        '+' => lhs_val + rhs_val,
        '-' => lhs_val - rhs_val,
        '*' => lhs_val * rhs_val,
        '/' => {
            if rhs_val == 0.0 {
                return Some("err: div by zero".to_string());
            }
            lhs_val / rhs_val
        }
        _ => return Some("err: unknown op".to_string()),
    };

    Some(result.to_string())
}
