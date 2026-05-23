use iwm_parser::gml_lowering::lower_raw_logic_file;
use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};
use iwm_parser::models::{RawLogicFile, RawLogicScript};

#[test]
fn lowering_respects_boolean_precedence_between_or_and_and() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 1,
            script_name: "scr_logic".to_string(),
            gml_source: "flag = a || b && c;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "flag"));
            assert!(matches!(
                value,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "||"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "a")
                    && matches!(
                        right.as_ref(),
                        LoweredLogicExpr::BinaryExpr { op, left, right }
                            if op == "&&"
                            && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "b")
                            && matches!(right.as_ref(), LoweredLogicExpr::Identifier(name) if name == "c")
                    )
            ));
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn lowering_ignores_comment_lines_and_preserves_var_declarations() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 2,
            script_name: "scr_vars".to_string(),
            gml_source: "// comment\nvar a, i; x = 1;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    assert!(matches!(
        lowered.entries[0].statements[0],
        LoweredLogicStatement::VariableDeclaration { ref names } if names == &vec!["a".to_string(), "i".to_string()]
    ));
    assert!(matches!(
        lowered.entries[0].statements[1],
        LoweredLogicStatement::Assignment { .. }
    ));
}

#[test]
fn lowering_preserves_return_statements_and_ignores_block_comments() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 4,
            script_name: "scr_return".to_string(),
            gml_source: "/* intro */ return false;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    assert!(matches!(
        lowered.entries[0].statements[0],
        LoweredLogicStatement::Return { value: Some(LoweredLogicExpr::LiteralBool(false)) }
    ));
}
