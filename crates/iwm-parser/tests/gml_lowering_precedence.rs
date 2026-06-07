use iwm_parser::gml_lowering::lower_raw_logic_file;
use iwm_parser::models::{RawLogicFile, RawLogicScript};
use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

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
        LoweredLogicStatement::Return {
            value: Some(LoweredLogicExpr::LiteralBool(false))
        }
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
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            ..
        } => {
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

#[test]
fn lowering_splits_top_level_newline_separated_assignments() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 8,
            script_name: "scr_newlines".to_string(),
            gml_source: "gravity=0.4\r\nL = keyboard_check_direct(global.leftbutton);\r\nR = keyboard_check_direct(global.rightbutton);\r\n".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;

    assert_eq!(statements.len(), 3);

    match &statements[0] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "gravity"));
            assert!(matches!(
                value,
                LoweredLogicExpr::LiteralNumber(number) if (*number - 0.4).abs() < f64::EPSILON
            ));
        }
        other => panic!("expected gravity assignment, got {other:?}"),
    }

    for (statement, expected_name, expected_member) in [
        (&statements[1], "L", "leftbutton"),
        (&statements[2], "R", "rightbutton"),
    ] {
        match statement {
            LoweredLogicStatement::Assignment { target, value } => {
                assert!(
                    matches!(target, LoweredLogicExpr::Identifier(name) if name == expected_name)
                );
                assert!(matches!(
                    value,
                    LoweredLogicExpr::Call { name, args }
                        if name == "keyboard_check_direct"
                        && args.len() == 1
                        && matches!(
                            &args[0],
                            LoweredLogicExpr::MemberAccess { target, member }
                                if member == expected_member
                                && matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                        )
                ));
            }
            other => panic!("expected keyboard assignment for {expected_name}, got {other:?}"),
        }
    }
}

#[test]
fn lowering_treats_single_equals_as_comparison_and_preserves_decimal_literals() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 9,
            script_name: "scr_jump_cut".to_string(),
            gml_source: "if(global.grav=0 && vspeed<0){ vspeed*=0.45; image_speed = 0.5; }"
                .to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            assert!(else_branch.is_empty());
            assert!(matches!(
                condition,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "&&"
                    && matches!(
                        left.as_ref(),
                        LoweredLogicExpr::BinaryExpr { op, left, right }
                            if op == "="
                            && matches!(
                                left.as_ref(),
                                LoweredLogicExpr::MemberAccess { target, member }
                                    if member == "grav"
                                    && matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                            )
                            && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 0.0).abs() < f64::EPSILON)
                    )
                    && matches!(
                        right.as_ref(),
                        LoweredLogicExpr::BinaryExpr { op, left, right }
                            if op == "<"
                            && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "vspeed")
                            && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 0.0).abs() < f64::EPSILON)
                    )
            ));
            assert_eq!(then_branch.len(), 2);

            match &then_branch[0] {
                LoweredLogicStatement::Assignment { target, value } => {
                    assert!(
                        matches!(target, LoweredLogicExpr::Identifier(name) if name == "vspeed")
                    );
                    assert!(matches!(
                        value,
                        LoweredLogicExpr::BinaryExpr { op, left, right }
                            if op == "*"
                            && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "vspeed")
                            && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 0.45).abs() < f64::EPSILON)
                    ));
                }
                other => panic!("expected vspeed compound assignment, got {other:?}"),
            }

            match &then_branch[1] {
                LoweredLogicStatement::Assignment { target, value } => {
                    assert!(
                        matches!(target, LoweredLogicExpr::Identifier(name) if name == "image_speed")
                    );
                    assert!(matches!(
                        value,
                        LoweredLogicExpr::LiteralNumber(number) if (*number - 0.5).abs() < f64::EPSILON
                    ));
                }
                other => panic!("expected image_speed assignment, got {other:?}"),
            }
        }
        other => panic!("expected conditional, got {other:?}"),
    }
}

#[test]
fn lowering_preserves_else_branch_function_calls_after_conditionals() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 10,
            script_name: "scr_else".to_string(),
            gml_source:
                "if(file_exists(\"temp\") == true){ tempExe(); } else { room_goto_next(); }"
                    .to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            assert!(matches!(
                condition,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "=="
                    && matches!(
                        left.as_ref(),
                        LoweredLogicExpr::Call { name, args }
                            if name == "file_exists"
                            && args.len() == 1
                            && matches!(args[0], LoweredLogicExpr::LiteralText(ref text) if text == "temp")
                    )
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralBool(true))
            ));
            assert!(matches!(
                then_branch.as_slice(),
                [LoweredLogicStatement::FunctionCall { name, args }]
                    if name == "tempExe" && args.is_empty()
            ));
            assert!(matches!(
                else_branch.as_slice(),
                [LoweredLogicStatement::FunctionCall { name, args }]
                    if name == "room_goto_next" && args.is_empty()
            ));
        }
        other => panic!("expected conditional with else branch, got {other:?}"),
    }
}

#[test]
fn lowering_preserves_else_branch_when_else_starts_on_next_line() {
    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 11,
            script_name: "scr_else_newline".to_string(),
            gml_source:
                "if(file_exists(\"temp\") == true){\r\n  tempExe();\r\n}\r\nelse{\r\n  room_goto_next();\r\n}\r\n"
                    .to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::Conditional {
            then_branch,
            else_branch,
            ..
        } => {
            assert!(matches!(
                then_branch.as_slice(),
                [LoweredLogicStatement::FunctionCall { name, args }]
                    if name == "tempExe" && args.is_empty()
            ));
            assert!(matches!(
                else_branch.as_slice(),
                [LoweredLogicStatement::FunctionCall { name, args }]
                    if name == "room_goto_next" && args.is_empty()
            ));
        }
        other => panic!("expected conditional with newline else branch, got {other:?}"),
    }
}
