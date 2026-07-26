use caria_lexer::{CariaLexer, Token};

pub fn format_code(input: &str) -> String {
    let mut lexer = CariaLexer::new(input);
    let mut formatted = String::new();
    let mut indent_level = 0;

    let mut need_newline = false;
    let mut was_space = false;

    // Simple rule-based formatter using lexer tokens
    while let Some((res, span)) = lexer.next() {
        if let Ok(token) = res {
            // Indent logic before writing token
            if need_newline {
                formatted.push('\n');
                for _ in 0..indent_level {
                    formatted.push_str("    ");
                }
                need_newline = false;
                was_space = false;
            }

            match &token {
                Token::LBrace => {
                    if !was_space && !formatted.is_empty() && !formatted.ends_with(' ') && !formatted.ends_with('\n') {
                        formatted.push(' ');
                    }
                    formatted.push('{');
                    indent_level += 1;
                    need_newline = true;
                }
                Token::RBrace => {
                    if indent_level > 0 {
                        indent_level -= 1;
                    }
                    // backtrack to remove indent if line empty
                    if formatted.ends_with("    ") {
                        formatted.truncate(formatted.len() - 4);
                    }
                    if !formatted.ends_with('\n') {
                        formatted.push('\n');
                    }
                    for _ in 0..indent_level {
                        formatted.push_str("    ");
                    }
                    formatted.push('}');
                    need_newline = true;
                }
                // Calesteria does not have explicit Semi token, we handle newlines based on context or formatting
                Token::Comma => {
                    formatted.push(',');
                    formatted.push(' ');
                    was_space = true;
                }
                Token::Colon => {
                    formatted.push(':');
                    formatted.push(' ');
                    was_space = true;
                }
                Token::Eq => {
                    if !was_space && !formatted.ends_with(' ') {
                        formatted.push(' ');
                    }
                    formatted.push('=');
                    formatted.push(' ');
                    was_space = true;
                }
                Token::Plus | Token::Minus | Token::Star | Token::Slash | Token::EqEq | Token::NotEq | Token::Less | Token::Greater | Token::LessEq | Token::GreaterEq => {
                    if !was_space && !formatted.ends_with(' ') {
                        formatted.push(' ');
                    }
                    formatted.push_str(&input[span.clone()]);
                    formatted.push(' ');
                    was_space = true;
                }
                _ => {
                    let tok_str = &input[span.clone()];
                    formatted.push_str(tok_str);
                    was_space = false;
                }
            }
        }
    }

    formatted
}
