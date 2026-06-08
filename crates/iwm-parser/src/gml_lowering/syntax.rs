pub(super) fn extract_parenthesized_block(input: &str) -> Option<(String, String)> {
    let mut depth = 0usize;
    let mut start = None;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => {
                depth += 1;
                if start.is_none() {
                    start = Some(index + ch.len_utf8());
                }
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let begin = start?;
                    let inside = input[begin..index].to_string();
                    let rest = input[index + ch.len_utf8()..].to_string();
                    return Some((inside, rest));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn extract_braced_block(input: &str) -> Option<(String, String)> {
    let mut depth = 0usize;
    let mut start = None;
    for (index, ch) in input.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                if start.is_none() {
                    start = Some(index + ch.len_utf8());
                }
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let begin = start?;
                    let inside = input[begin..index].to_string();
                    let rest = input[index + ch.len_utf8()..].to_string();
                    return Some((inside, rest));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn split_head_and_body(rest: &str) -> Option<(String, String, String)> {
    let rest = rest.trim_start();
    let (head, tail) = if rest.starts_with('(') {
        extract_parenthesized_block(rest)?
    } else {
        let brace_index = rest.find('{')?;
        (
            rest[..brace_index].trim().to_string(),
            rest[brace_index..].to_string(),
        )
    };

    let tail = tail.trim_start();
    let (body, after_body) = extract_braced_block(tail)?;
    Some((normalize_group_head(&head), body, after_body.to_string()))
}

fn normalize_group_head(head: &str) -> String {
    let trimmed = head.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if extract_parenthesized_block(trimmed)
            .map(|(_, rest)| rest.trim().is_empty())
            .unwrap_or(false)
        {
            return inner.trim().to_string();
        }
    }

    trimmed.to_string()
}

pub(super) fn strip_balanced_outer_parens(s: &str) -> &str {
    let trimmed = s.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let mut depth = 0usize;
        let mut valid = true;
        for ch in inner.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        valid = false;
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }
        if valid && depth == 0 {
            return strip_balanced_outer_parens(inner);
        }
    }
    trimmed
}

pub(super) fn split_top_level_statements(source: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;

    for (index, ch) in source.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => {
                brace_depth += 1;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(ch);
                if paren_depth == 0 && brace_depth == 0 {
                    let next = source[index + ch.len_utf8()..].trim_start();
                    if !next.starts_with("else") {
                        let stmt = current.trim();
                        if !stmt.is_empty() {
                            statements.push(stmt.to_string());
                        }
                        current.clear();
                        continue;
                    }
                }
                continue;
            }
            ';' if paren_depth == 0 && brace_depth == 0 => {
                let stmt = current.trim();
                if !stmt.is_empty() {
                    statements.push(stmt.to_string());
                }
                current.clear();
                continue;
            }
            '\n' if paren_depth == 0 && brace_depth == 0 => {
                let next = source[index + ch.len_utf8()..].trim_start();
                if should_split_top_level_newline(current.trim(), next) {
                    let stmt = current.trim();
                    if !stmt.is_empty() {
                        statements.push(stmt.to_string());
                    }
                    current.clear();
                    continue;
                }
            }
            _ => {}
        }
        current.push(ch);
    }

    let stmt = current.trim();
    if !stmt.is_empty() {
        statements.push(stmt.to_string());
    }

    statements
}

pub(super) fn split_top_level_commas_or_semicolons(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in source.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ';' if paren_depth == 0 && brace_depth == 0 => {
                let part = current.trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }

    let part = current.trim();
    if !part.is_empty() {
        parts.push(part.to_string());
    }

    parts
}

pub(super) fn split_top_level_csv(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in source.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                let part = current.trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                current.clear();
                continue;
            }
            _ => {}
        }

        current.push(ch);
    }

    let part = current.trim();
    if !part.is_empty() {
        parts.push(part.to_string());
    }

    parts
}

pub(super) fn split_top_level_operator(source: &str, operator: &str) -> Option<(String, String)> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let op_len = operator.len();
    let mut i = 0usize;

    while i < chars.len() {
        let (byte_index, ch) = chars[i];
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }

        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
            let tail = &source[byte_index..];
            if tail.starts_with(operator)
                && operator_occurrence_is_valid(source, byte_index, operator)
            {
                let left = source[..byte_index].trim();
                let right = source[byte_index + op_len..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left.to_string(), right.to_string()));
                }
            }
        }

        i += 1;
    }

    None
}

fn should_split_top_level_newline(current: &str, next: &str) -> bool {
    if current.is_empty() || next.is_empty() {
        return false;
    }

    let next_lower = next.to_ascii_lowercase();
    if next_lower.starts_with("else") {
        return false;
    }

    if current.ends_with('{')
        || current.ends_with('(')
        || current.ends_with('[')
        || current.ends_with(',')
        || current.ends_with('+')
        || current.ends_with('-')
        || current.ends_with('*')
        || current.ends_with('/')
        || current.ends_with('=')
        || current.ends_with("&&")
        || current.ends_with("||")
    {
        return false;
    }

    let current_lower = current.to_ascii_lowercase();
    if (current_lower.starts_with("if")
        || current_lower.starts_with("with")
        || current_lower.starts_with("repeat")
        || current_lower.starts_with("while")
        || current_lower.starts_with("for")
        || current_lower == "else")
        && (next.starts_with('{') || next.starts_with("if"))
    {
        return false;
    }

    true
}

fn operator_occurrence_is_valid(source: &str, byte_index: usize, operator: &str) -> bool {
    if operator != "=" {
        return true;
    }

    let before = source[..byte_index].chars().next_back();
    let after = source[byte_index + operator.len()..].chars().next();

    !matches!(
        before,
        Some('=')
            | Some('!')
            | Some('<')
            | Some('>')
            | Some('+')
            | Some('-')
            | Some('*')
            | Some('/')
    ) && !matches!(after, Some('='))
}

pub(super) fn split_top_level_trailing_index(source: &str) -> Option<(String, String)> {
    if !source.ends_with(']') {
        return None;
    }

    let mut bracket_depth = 0usize;
    for (index, ch) in source.char_indices().rev() {
        match ch {
            ']' => bracket_depth += 1,
            '[' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                if bracket_depth == 0 {
                    let target = source[..index].trim();
                    let index_expr = source[index + 1..source.len() - 1].trim();
                    if !target.is_empty() && !index_expr.is_empty() {
                        return Some((target.to_string(), index_expr.to_string()));
                    }
                    return None;
                }
            }
            _ => {}
        }
    }

    None
}

pub(super) fn find_top_level_dot(source: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut last_dot = None;

    for (index, ch) in source.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '.' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                last_dot = Some(index)
            }
            _ => {}
        }
    }

    last_dot
}
