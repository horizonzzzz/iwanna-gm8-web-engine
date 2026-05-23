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

#[test]
fn lowering_preserves_nested_call_arguments_in_function_calls() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 5,
            script_name: "scr_nested_call".to_string(),
            gml_source: "room_goto(room_next(room));".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::FunctionCall { name, args } => {
            assert_eq!(name, "room_goto");
            assert!(matches!(
                &args[0],
                LoweredLogicExpr::Call { name, args }
                    if name == "room_next"
                    && args.len() == 1
                    && matches!(args[0], LoweredLogicExpr::Identifier(ref ident) if ident == "room")
            ));
        }
        other => panic!("expected nested function call, got {other:?}"),
    }
}

#[test]
fn lowering_preserves_chained_member_and_index_assignment_targets() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 6,
            script_name: "scr_chain".to_string(),
            gml_source: "global.save[slot].x = x;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(value, LoweredLogicExpr::Identifier(name) if name == "x"));
            assert!(matches!(
                target,
                LoweredLogicExpr::MemberAccess { target, member }
                    if member == "x"
                    && matches!(
                        target.as_ref(),
                        LoweredLogicExpr::IndexAccess { target, index }
                            if matches!(
                                target.as_ref(),
                                LoweredLogicExpr::MemberAccess { target, member }
                                    if member == "save"
                                    && matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                            )
                            && matches!(index.as_ref(), LoweredLogicExpr::Identifier(name) if name == "slot")
                    )
            ));
        }
        other => panic!("expected chained assignment target, got {other:?}"),
    }
}

#[test]
fn lowering_preserves_unary_negative_and_not_expressions() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 7,
            script_name: "scr_unary".to_string(),
            gml_source: "if !flag { x = -y; }".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Conditional { condition, then_branch, .. } => {
            assert!(matches!(
                condition,
                LoweredLogicExpr::UnaryExpr { op, child }
                    if op == "!"
                    && matches!(child.as_ref(), LoweredLogicExpr::Identifier(name) if name == "flag")
            ));
            match &then_branch[0] {
                LoweredLogicStatement::Assignment { target, value } => {
                    assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "x"));
                    assert!(matches!(
                        value,
                        LoweredLogicExpr::UnaryExpr { op, child }
                            if op == "-"
                            && matches!(child.as_ref(), LoweredLogicExpr::Identifier(name) if name == "y")
                    ));
                }
                other => panic!("expected unary assignment in branch, got {other:?}"),
            }
        }
        other => panic!("expected conditional with unary condition, got {other:?}"),
    }
}
