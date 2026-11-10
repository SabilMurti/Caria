//! Caria Code Formatter - Enterprise Indentation & Style Engine
//! Preserves comments, string literals, and normalizes standard layout.

pub fn format_code(input: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut indent_level = 0usize;

    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            if result.last().map_or(false, |l| !l.is_empty()) {
                result.push(String::new());
            }
            continue;
        }

        // Count leading closing braces on this line to dedent before printing
        let leading_rbraces = count_leading_rbraces(trimmed);
        let effective_indent = indent_level.saturating_sub(leading_rbraces);

        let formatted_line = format_line(trimmed);
        let mut line_str = String::with_capacity(effective_indent * 4 + formatted_line.len());
        for _ in 0..effective_indent {
            line_str.push_str("    ");
        }
        line_str.push_str(&formatted_line);
        result.push(line_str);

        // Update ongoing indentation based on net braces outside strings & comments
        let net = count_net_braces(trimmed);
        indent_level = (indent_level as isize + net).max(0) as usize;
    }

    let mut out = result.join("\n");
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn count_leading_rbraces(line: &str) -> usize {
    let mut count = 0;
    for c in line.chars() {
        if c == '}' {
            count += 1;
        } else if c.is_whitespace() {
            continue;
        } else {
            break;
        }
    }
    count
}

fn count_net_braces(line: &str) -> isize {
    let mut net = 0isize;
    let mut in_str = false;
    let mut escape = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if in_str {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            break; // Rest of line is comment
        } else if c == '"' {
            in_str = true;
        } else if c == '{' {
            net += 1;
        } else if c == '}' {
            net -= 1;
        }
        i += 1;
    }
    net
}

fn format_line(line: &str) -> String {
    // If entire line is a comment, preserve as is
    if line.starts_with("//") {
        return line.to_string();
    }

    // Split code and trailing comment if any
    let (code_part, comment_part) = split_trailing_comment(line);
    let mut formatted_code = normalize_code(code_part.trim_end());

    if let Some(comment) = comment_part {
        if !formatted_code.is_empty() {
            formatted_code.push(' ');
        }
        formatted_code.push_str(comment);
    }

    formatted_code
}

fn split_trailing_comment(line: &str) -> (&str, Option<&str>) {
    let mut in_str = false;
    let mut escape = false;
    let chars: Vec<(usize, char)> = line.char_indices().collect();

    for i in 0..chars.len() {
        let (byte_idx, c) = chars[i];
        if in_str {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '/' && i + 1 < chars.len() && chars[i + 1].1 == '/' {
            return (&line[..byte_idx], Some(&line[byte_idx..]));
        } else if c == '"' {
            in_str = true;
        }
    }

    (line, None)
}

fn normalize_code(code: &str) -> String {
    let mut out = String::with_capacity(code.len() + 16);
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    let mut escape = false;

    while i < chars.len() {
        let c = chars[i];

        if in_str {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_str = true;
            out.push(c);
            i += 1;
            continue;
        }

        // Two-character operators: :=, ->, ==, !=, <=, >=, |>
        if i + 1 < chars.len() {
            let next_c = chars[i + 1];
            let op_str = match (c, next_c) {
                (':', '=') => Some(":="),
                ('-', '>') => Some("->"),
                ('=', '=') => Some("=="),
                ('!', '=') => Some("!="),
                ('<', '=') => Some("<="),
                ('>', '=') => Some(">="),
                ('|', '>') => Some("|>"),
                _ => None,
            };

            if let Some(op) = op_str {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(op);
                out.push(' ');
                i += 2;
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                continue;
            }
        }

        // Single character punctuation
        match c {
            '{' => {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
                out.push('{');
            }
            ',' => {
                out.push(',');
                if i + 1 < chars.len() && !chars[i + 1].is_whitespace() && chars[i + 1] != ')' {
                    out.push(' ');
                }
            }
            ':' => {
                out.push(':');
                if i + 1 < chars.len() && !chars[i + 1].is_whitespace() {
                    out.push(' ');
                }
            }
            ' ' | '\t' => {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            }
            _ => {
                out.push(c);
            }
        }
        i += 1;
    }

    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_preserves_runes() {
        let input = "rune abs(n: int) -> int {\nif n < 0 {\nret -n\n}\nret n\n}\n";
        let formatted = format_code(input);
        assert!(formatted.contains("rune abs(n: int) -> int {"));
        assert!(formatted.contains("    if n < 0 {"));
        assert!(formatted.contains("        ret -n"));
        assert!(formatted.contains("    ret n"));
    }

    #[test]
    fn test_format_preserves_comments() {
        let input = "// Header comment\nrune test() -> int {\n    // Nested comment\n    ret 0\n}\n";
        let formatted = format_code(input);
        assert!(formatted.contains("// Header comment"));
        assert!(formatted.contains("    // Nested comment"));
    }
}
