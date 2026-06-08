use iwm_runtime_model::LoweredLogicStatement;

use super::statement::lower_statement;
use super::syntax::split_top_level_statements;

pub(super) fn looks_like_gml_source(source: &str) -> bool {
    let trimmed = source.trim();
    !trimmed.is_empty()
        && (trimmed.contains('=')
            || trimmed.contains('(')
            || trimmed.contains('{')
            || trimmed.contains('}')
            || trimmed.contains(';'))
}

pub(super) fn lower_source(source: &str) -> Vec<LoweredLogicStatement> {
    let source = strip_block_comments(&strip_line_comments(source));
    split_top_level_statements(&source)
        .into_iter()
        .filter_map(|stmt| lower_statement(&stmt))
        .collect()
}

fn strip_line_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;

    while let Some(ch) = chars.next() {
        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }

        if !in_string && ch == '/' && matches!(chars.peek(), Some('/')) {
            chars.next();
            for next in chars.by_ref() {
                if next == '\n' {
                    result.push('\n');
                    break;
                }
            }
            continue;
        }

        result.push(ch);
    }

    result
}

fn strip_block_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;

    while let Some(ch) = chars.next() {
        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }

        if !in_string && ch == '/' && matches!(chars.peek(), Some('*')) {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && matches!(chars.peek(), Some('/')) {
                    chars.next();
                    break;
                }
            }
            continue;
        }

        result.push(ch);
    }

    result
}
