use iwm_parser::gml_lowering::lower_raw_logic_file;
use iwm_parser::models::{RawLogicFile, RawLogicScript};
use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

fn lower_script(source: &str) -> Vec<LoweredLogicStatement> {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 1,
            script_name: "scr_logic".to_string(),
            gml_source: source.to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    lower_raw_logic_file(&raw).entries[0].statements.clone()
}

/// Punctuation inside a GML string literal must never be treated as syntax.
/// A stray `.` used to split `"temp.dat"` into a member access, which silently
/// broke every save/load path that names a file with an extension.
fn expr_has_member_access(expr: &LoweredLogicExpr) -> bool {
    match expr {
        LoweredLogicExpr::MemberAccess { .. } => true,
        LoweredLogicExpr::UnaryExpr { child, .. } => expr_has_member_access(child),
        LoweredLogicExpr::BinaryExpr { left, right, .. } => {
            expr_has_member_access(left) || expr_has_member_access(right)
        }
        LoweredLogicExpr::Call { args, .. } => args.iter().any(expr_has_member_access),
        LoweredLogicExpr::IndexAccess { target, index } => {
            expr_has_member_access(target) || expr_has_member_access(index)
        }
        _ => false,
    }
}

fn assignment_value(statement: &LoweredLogicStatement) -> &LoweredLogicExpr {
    match statement {
        LoweredLogicStatement::Assignment { value, .. } => value,
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn lowering_keeps_dotted_string_literal_as_text_argument() {
    let statements = lower_script("f = file_bin_open(\"temp.dat\", 1);");
    let value = assignment_value(&statements[0]);

    match value {
        LoweredLogicExpr::Call { name, args } => {
            assert_eq!(name, "file_bin_open");
            assert!(
                matches!(&args[0], LoweredLogicExpr::LiteralText(text) if text == "temp.dat"),
                "expected literal text argument, got {:?}",
                args[0]
            );
            assert!(
                matches!(&args[1], LoweredLogicExpr::LiteralNumber(mode) if *mode == 1.0),
                "expected mode argument, got {:?}",
                args[1]
            );
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn lowering_keeps_dotted_extension_in_string_concatenation() {
    let statements = lower_script("name = \"save\" + string(global.savenum) + \".dat\";");
    let value = assignment_value(&statements[0]);

    // The lowering splits at the first top-level `+`, so the tree is
    // right-associative: `"save" + (string(global.savenum) + ".dat")`.
    // `global.savenum` is a real member access, so assert on the literal instead.
    match value {
        LoweredLogicExpr::BinaryExpr { op, right, .. } => {
            assert_eq!(op, "+");
            match right.as_ref() {
                LoweredLogicExpr::BinaryExpr { op, right, .. } => {
                    assert_eq!(op, "+");
                    assert!(
                        matches!(right.as_ref(), LoweredLogicExpr::LiteralText(text) if text == ".dat"),
                        "expected trailing \".dat\" literal, got {right:?}"
                    );
                }
                other => panic!("expected nested binary expression, got {other:?}"),
            }
        }
        other => panic!("expected binary expression, got {other:?}"),
    }
}

#[test]
fn lowering_does_not_split_call_arguments_on_comma_inside_string() {
    let statements = lower_script("label = choose(\"a,b\", \"c\");");
    let value = assignment_value(&statements[0]);

    match value {
        LoweredLogicExpr::Call { name, args } => {
            assert_eq!(name, "choose");
            assert_eq!(
                args.len(),
                2,
                "comma inside a string split the args: {args:?}"
            );
            assert!(
                matches!(&args[0], LoweredLogicExpr::LiteralText(text) if text == "a,b"),
                "expected first literal to keep its comma, got {:?}",
                args[0]
            );
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn lowering_does_not_split_statements_on_semicolon_inside_string() {
    let statements = lower_script("message = \"a;b\";");

    assert_eq!(
        statements.len(),
        1,
        "semicolon inside a string split the statement: {statements:?}"
    );
    assert!(
        matches!(
            assignment_value(&statements[0]),
            LoweredLogicExpr::LiteralText(text) if text == "a;b"
        ),
        "expected literal text, got {:?}",
        assignment_value(&statements[0])
    );
}

#[test]
fn lowering_does_not_treat_punctuation_in_strings_as_syntax() {
    for source in [
        "f = file_bin_open(\"temp.dat\", 1);",
        "ok = file_exists(\"temp.dat\");",
        "path = \"data/save.1.dat\";",
        "single = file_exists('temp.dat');",
    ] {
        let statements = lower_script(source);
        let value = assignment_value(&statements[0]);
        assert!(
            !expr_has_member_access(value),
            "string punctuation lowered as member access for `{source}`: {value:?}"
        );
    }
}

#[test]
fn lowering_ignores_equals_inside_string_literals() {
    let statements = lower_script(
        "file_text_write_string(f, \"key=value\"); label = \"a=b\"; ok = x == \"a=b\";",
    );

    assert!(matches!(
        &statements[0],
        LoweredLogicStatement::FunctionCall { name, args }
            if name == "file_text_write_string"
                && matches!(&args[1], LoweredLogicExpr::LiteralText(value) if value == "key=value")
    ));
    assert!(matches!(
        assignment_value(&statements[1]),
        LoweredLogicExpr::LiteralText(value) if value == "a=b"
    ));
    assert!(matches!(
        assignment_value(&statements[2]),
        LoweredLogicExpr::BinaryExpr { op, .. } if op == "=="
    ));
}
