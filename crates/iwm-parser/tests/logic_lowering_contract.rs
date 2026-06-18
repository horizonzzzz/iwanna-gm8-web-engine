use std::fs;

use iwm_runtime_model::ObjectDefinition;

#[test]
fn raw_logic_file_preserves_gml_ownership_and_source_text() {
    use iwm_parser::models::{
        RawCodeAction, RawLogicEventBinding, RawLogicFile, RawLogicOwner, RawLogicOwnerKind,
        RawLogicScript, RawLogicTimelineMoment, RawLogicTrigger,
    };

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![RawLogicOwner {
            owner_kind: RawLogicOwnerKind::Room,
            owner_id: 7,
            owner_name: "room_07".to_string(),
            event_type: None,
            sub_event: None,
            collision_object_id: None,
            block_id: "room:7:create".to_string(),
            gml_source: "global.boss_hp = 100;".to_string(),
        }],
        instance_creation_codes: vec![RawLogicOwner {
            owner_kind: RawLogicOwnerKind::RoomInstance,
            owner_id: 1001,
            owner_name: "obj_spike".to_string(),
            event_type: None,
            sub_event: None,
            collision_object_id: None,
            block_id: "room:7:instance:1001:create".to_string(),
            gml_source: "timer = -30;".to_string(),
        }],
        object_events: vec![RawLogicEventBinding {
            object_id: 12,
            object_name: "obj_spike".to_string(),
            event_type: 3,
            sub_event: 0,
            event_tag: "step".to_string(),
            collision_object_id: None,
            block_id: "object:12:event:3:0".to_string(),
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: "code".to_string(),
                fn_code: "timer += 1; if timer > 60 { y -= 2; }".to_string(),
                args: vec!["timer".to_string()],
            }],
        }],
        scripts: vec![RawLogicScript {
            script_id: 3,
            script_name: "scr_bullet_pattern".to_string(),
            gml_source: "instance_create(x, y, obj_bullet);".to_string(),
        }],
        triggers: vec![RawLogicTrigger {
            trigger_id: 2,
            trigger_name: "tr_player_near".to_string(),
            constant_name: "tr_player_near".to_string(),
            moment: "step".to_string(),
            condition_gml: "distance_to_object(obj_player) < 32".to_string(),
        }],
        timelines: vec![RawLogicTimelineMoment {
            timeline_id: 4,
            timeline_name: "tml_intro".to_string(),
            moment: 30,
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: "code".to_string(),
                fn_code: "alarm[0] = 60;".to_string(),
                args: vec![],
            }],
        }],
    };

    let json = serde_json::to_value(&raw).unwrap();
    assert_eq!(json["format"], "iwm-raw-logic-v1");
    assert_eq!(
        json["room_creation_codes"][0]["gml_source"],
        "global.boss_hp = 100;"
    );
    assert_eq!(
        json["instance_creation_codes"][0]["gml_source"],
        "timer = -30;"
    );
    assert_eq!(
        json["object_events"][0]["actions"][0]["fn_code"],
        "timer += 1; if timer > 60 { y -= 2; }"
    );
    assert_eq!(
        json["scripts"][0]["gml_source"],
        "instance_create(x, y, obj_bullet);"
    );
    assert_eq!(
        json["triggers"][0]["condition_gml"],
        "distance_to_object(obj_player) < 32"
    );
    assert_eq!(
        json["timelines"][0]["actions"][0]["fn_code"],
        "alarm[0] = 60;"
    );
}

#[test]
fn lowered_logic_file_tokenizes_assignments_and_calls_from_raw_logic() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{
        RawCodeAction, RawLogicEventBinding, RawLogicFile, RawLogicOwner, RawLogicOwnerKind,
        RawLogicScript,
    };
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![RawLogicOwner {
            owner_kind: RawLogicOwnerKind::Room,
            owner_id: 7,
            owner_name: "room_07".to_string(),
            event_type: None,
            sub_event: None,
            collision_object_id: None,
            block_id: "room:7:create".to_string(),
            gml_source: "global.score = 0; instance_create(x, y, obj_bullet);".to_string(),
        }],
        instance_creation_codes: vec![],
        object_events: vec![RawLogicEventBinding {
            object_id: 12,
            object_name: "obj_spike".to_string(),
            event_type: 3,
            sub_event: 0,
            event_tag: "step".to_string(),
            collision_object_id: None,
            block_id: "object:12:event:3:0".to_string(),
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: "code".to_string(),
                fn_code: "if place_meeting(x, y, obj_player) { game_restart(); }".to_string(),
                args: vec![],
            }],
        }],
        scripts: vec![RawLogicScript {
            script_id: 3,
            script_name: "scr_bullet_pattern".to_string(),
            gml_source: "instance_create(x, y, obj_bullet);".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    assert_eq!(lowered.entries.len(), 3);
    assert!(matches!(
        lowered.entries[0].statements[0],
        LoweredLogicStatement::Assignment { ref target, ref value }
            if matches!(target, LoweredLogicExpr::MemberAccess { target, member }
                if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                && member == "score")
            && matches!(value, LoweredLogicExpr::LiteralNumber(number) if (*number - 0.0).abs() < f64::EPSILON)
    ));
    assert!(matches!(
        lowered.entries[0].statements[1],
        LoweredLogicStatement::FunctionCall { ref name, .. } if name == "instance_create"
    ));
    assert!(matches!(
        lowered.entries[1].statements[0],
        LoweredLogicStatement::Conditional { ref condition, .. }
            if matches!(condition, LoweredLogicExpr::Call { name, .. } if name == "place_meeting")
    ));
    assert!(matches!(
        lowered.entries[2].statements[0],
        LoweredLogicStatement::FunctionCall { ref name, .. } if name == "instance_create"
    ));
}

#[test]
fn lowered_logic_file_recognizes_control_flow_blocks() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawCodeAction, RawLogicEventBinding, RawLogicFile};
    use iwm_parser::LoweredLogicStatement;

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![RawLogicEventBinding {
            object_id: 12,
            object_name: "obj_spike".to_string(),
            event_type: 3,
            sub_event: 0,
            event_tag: "step".to_string(),
            collision_object_id: None,
            block_id: "object:12:event:3:0".to_string(),
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: "code".to_string(),
                fn_code: "if place_meeting(x, y, obj_player) { game_restart(); }".to_string(),
                args: vec![],
            }],
        }],
        scripts: vec![],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    assert!(matches!(
        lowered.entries[0].statements[0],
        LoweredLogicStatement::Conditional { .. }
    ));
}

#[test]
fn lowered_logic_file_recognizes_common_loop_blocks() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawCodeAction, RawLogicEventBinding, RawLogicFile};
    use iwm_parser::LoweredLogicStatement;

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![RawLogicEventBinding {
            object_id: 12,
            object_name: "obj_spike".to_string(),
            event_type: 3,
            sub_event: 0,
            event_tag: "step".to_string(),
            collision_object_id: None,
            block_id: "object:12:event:3:0".to_string(),
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: "code".to_string(),
                fn_code: "with (obj_player) { x += hspeed; } repeat (3) { y -= 2; } while (y < 100) { y += 1; } for (i = 0; i < 3; i += 1) { alarm[0] = 60; }".to_string(),
                args: vec![],
            }],
        }],
        scripts: vec![],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;
    assert!(matches!(statements[0], LoweredLogicStatement::With { .. }));
    assert!(matches!(
        statements[1],
        LoweredLogicStatement::Repeat { .. }
    ));
    assert!(matches!(statements[2], LoweredLogicStatement::While { .. }));
    assert!(matches!(statements[3], LoweredLogicStatement::For { .. }));
}

#[test]
fn lowered_logic_file_translates_common_function_actions() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawCodeAction, RawLogicEventBinding, RawLogicFile};
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![RawLogicEventBinding {
            object_id: 819,
            object_name: "obj_respawn_helper".to_string(),
            event_type: 2,
            sub_event: 0,
            event_tag: "alarm:0".to_string(),
            collision_object_id: None,
            block_id: "object:819:event:2:0".to_string(),
            actions: vec![
                RawCodeAction {
                    action_id: 301,
                    lib_id: 1,
                    action_kind: 0,
                    execution_type: 1,
                    fn_name: "action_set_alarm".to_string(),
                    fn_code: String::new(),
                    args: vec!["80".to_string(), "0".to_string()],
                },
                RawCodeAction {
                    action_id: 201,
                    lib_id: 1,
                    action_kind: 0,
                    execution_type: 1,
                    fn_name: "action_create_object".to_string(),
                    fn_code: String::new(),
                    args: vec!["5".to_string(), "x".to_string(), "y".to_string()],
                },
                RawCodeAction {
                    action_id: 203,
                    lib_id: 1,
                    action_kind: 0,
                    execution_type: 1,
                    fn_name: "action_kill_object".to_string(),
                    fn_code: String::new(),
                    args: vec![],
                },
            ],
        }],
        scripts: vec![],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;
    assert_eq!(statements.len(), 3);
    assert!(matches!(
        statements[0],
        LoweredLogicStatement::Assignment { ref target, ref value }
            if matches!(
                target,
                LoweredLogicExpr::IndexAccess { target, index }
                    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "alarm")
                    && matches!(index.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 0.0).abs() < f64::EPSILON)
            )
            && matches!(value, LoweredLogicExpr::LiteralNumber(number) if (*number - 80.0).abs() < f64::EPSILON)
    ));
    assert!(matches!(
        statements[1],
        LoweredLogicStatement::FunctionCall { ref name, ref args }
            if name == "instance_create"
            && matches!(args[0], LoweredLogicExpr::Identifier(ref ident) if ident == "x")
            && matches!(args[1], LoweredLogicExpr::Identifier(ref ident) if ident == "y")
            && matches!(args[2], LoweredLogicExpr::LiteralNumber(number) if (number - 5.0).abs() < f64::EPSILON)
    ));
    assert!(matches!(
        statements[2],
        LoweredLogicStatement::FunctionCall { ref name, ref args }
            if name == "instance_destroy" && args.is_empty()
    ));
}

#[test]
fn lowered_logic_file_preserves_nested_function_call_arguments() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawLogicFile, RawLogicScript};
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 7,
            script_name: "scr_spawn".to_string(),
            gml_source: "instance_create(x, y - 4, choose(obj_player2, obj_player3));".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);

    match &lowered.entries[0].statements[0] {
        LoweredLogicStatement::FunctionCall { name, args } => {
            assert_eq!(name, "instance_create");
            assert!(matches!(args[0], LoweredLogicExpr::Identifier(ref ident) if ident == "x"));
            assert!(matches!(
                args[1],
                LoweredLogicExpr::BinaryExpr { ref op, ref left, ref right }
                    if op == "-"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "y")
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 4.0).abs() < f64::EPSILON)
            ));
            assert!(matches!(
                args[2],
                LoweredLogicExpr::Call { ref name, ref args }
                    if name == "choose"
                    && args.len() == 2
                    && matches!(args[0], LoweredLogicExpr::Identifier(ref ident) if ident == "obj_player2")
                    && matches!(args[1], LoweredLogicExpr::Identifier(ref ident) if ident == "obj_player3")
            ));
        }
        other => panic!("expected function call, got {other:?}"),
    }
}

#[test]
fn lowered_logic_file_does_not_treat_comparisons_as_assignments() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawCodeAction, RawLogicEventBinding, RawLogicFile};
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![RawLogicEventBinding {
            object_id: 12,
            object_name: "obj_logic".to_string(),
            event_type: 3,
            sub_event: 0,
            event_tag: "step".to_string(),
            collision_object_id: None,
            block_id: "object:12:event:3:0".to_string(),
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: "code".to_string(),
                fn_code: "if a == b { game_restart(); } x = y >= z;".to_string(),
                args: vec![],
            }],
        }],
        scripts: vec![],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;

    assert!(matches!(
        statements[0],
        LoweredLogicStatement::Conditional { .. }
    ));
    assert!(matches!(
        statements[1],
        LoweredLogicStatement::Assignment { ref target, ref value }
            if matches!(target, LoweredLogicExpr::Identifier(name) if name == "x")
            && matches!(
                value,
                LoweredLogicExpr::BinaryExpr { ref op, ref left, ref right }
                    if op == ">="
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "y")
                    && matches!(right.as_ref(), LoweredLogicExpr::Identifier(name) if name == "z")
            )
    ));
}

#[test]
fn lowered_logic_file_emits_structured_member_index_and_binary_expressions() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawLogicFile, RawLogicScript};
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![RawLogicScript {
            script_id: 9,
            script_name: "scr_logic".to_string(),
            gml_source: "global.grav = arr[0] + 2; instance_create(x, y - 4, player2);".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;

    match &statements[0] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(
                target,
                LoweredLogicExpr::MemberAccess { target, member }
                    if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                    && member == "grav"
            ));
            assert!(matches!(
                value,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "+"
                    && matches!(
                        left.as_ref(),
                        LoweredLogicExpr::IndexAccess { target, index }
                            if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "arr")
                            && matches!(index.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 0.0).abs() < f64::EPSILON)
                    )
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 2.0).abs() < f64::EPSILON)
            ));
        }
        other => panic!("expected structured assignment, got {other:?}"),
    }

    match &statements[1] {
        LoweredLogicStatement::FunctionCall { name, args } => {
            assert_eq!(name, "instance_create");
            assert!(matches!(args[0], LoweredLogicExpr::Identifier(ref name) if name == "x"));
            assert!(matches!(
                args[1],
                LoweredLogicExpr::BinaryExpr { ref op, ref left, ref right }
                    if op == "-"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "y")
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 4.0).abs() < f64::EPSILON)
            ));
            assert!(matches!(args[2], LoweredLogicExpr::Identifier(ref name) if name == "player2"));
        }
        other => panic!("expected structured function call, got {other:?}"),
    }
}

#[test]
fn lowered_logic_file_handles_compound_assignments() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::RawLogicFile;
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![iwm_parser::models::RawLogicScript {
            script_id: 11,
            script_name: "scr_compound".to_string(),
            gml_source: "x += 1; timer -= 2; score *= 3;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;

    match &statements[0] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "x"));
            assert!(matches!(
                value,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "+"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "x")
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 1.0).abs() < f64::EPSILON)
            ));
        }
        other => panic!("expected compound assignment for x += 1, got {other:?}"),
    }

    match &statements[1] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "timer"));
            assert!(matches!(
                value,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "-"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "timer")
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 2.0).abs() < f64::EPSILON)
            ));
        }
        other => panic!("expected compound assignment for timer -= 2, got {other:?}"),
    }

    match &statements[2] {
        LoweredLogicStatement::Assignment { target, value } => {
            assert!(matches!(target, LoweredLogicExpr::Identifier(name) if name == "score"));
            assert!(matches!(
                value,
                LoweredLogicExpr::BinaryExpr { op, left, right }
                    if op == "*"
                    && matches!(left.as_ref(), LoweredLogicExpr::Identifier(name) if name == "score")
                    && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 3.0).abs() < f64::EPSILON)
            ));
        }
        other => panic!("expected compound assignment for score *= 3, got {other:?}"),
    }
}

#[test]
fn lowered_logic_file_handles_increment_and_decrement() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::RawLogicFile;
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![],
        scripts: vec![iwm_parser::models::RawLogicScript {
            script_id: 12,
            script_name: "scr_incdec".to_string(),
            gml_source: "i++; ++j; k--; --m;".to_string(),
        }],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;

    for (index, (name, op)) in [("i", "+"), ("j", "+"), ("k", "-"), ("m", "-")]
        .into_iter()
        .enumerate()
    {
        match &statements[index] {
            LoweredLogicStatement::Assignment { target, value } => {
                assert!(matches!(target, LoweredLogicExpr::Identifier(ident) if ident == name));
                assert!(matches!(
                    value,
                    LoweredLogicExpr::BinaryExpr { op: value_op, left, right }
                        if value_op == op
                        && matches!(left.as_ref(), LoweredLogicExpr::Identifier(ident) if ident == name)
                        && matches!(right.as_ref(), LoweredLogicExpr::LiteralNumber(number) if (*number - 1.0).abs() < f64::EPSILON)
                ));
            }
            other => panic!("expected increment/decrement assignment for {name}, got {other:?}"),
        }
    }
}

// ============================================================================
// Step 2: Tests for new contract fields (Tighten Runtime Execution Contract)
// ============================================================================

#[test]
fn lowered_logic_file_uses_code_action_args_when_fn_code_is_empty() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawCodeAction, RawLogicEventBinding, RawLogicFile};
    use iwm_parser::{LoweredLogicExpr, LoweredLogicStatement};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![],
        object_events: vec![RawLogicEventBinding {
            object_id: 0,
            object_name: "player".to_string(),
            event_type: 3,
            sub_event: 0,
            event_tag: "step".to_string(),
            collision_object_id: None,
            block_id: "object:0:event:3:0".to_string(),
            actions: vec![RawCodeAction {
                action_id: 603,
                lib_id: 1,
                action_kind: 7,
                execution_type: 2,
                fn_name: String::new(),
                fn_code: String::new(),
                args: vec![
                    "if keyboard_check_pressed(global.jumpbutton) { playerJump(); }".to_string(),
                ],
            }],
        }],
        scripts: vec![],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let statements = &lowered.entries[0].statements;

    match &statements[0] {
        LoweredLogicStatement::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            assert!(else_branch.is_empty());
            assert!(matches!(
                condition,
                LoweredLogicExpr::Call { name, args }
                    if name == "keyboard_check_pressed"
                    && args.len() == 1
                    && matches!(
                        &args[0],
                        LoweredLogicExpr::MemberAccess { target, member }
                            if matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                            && member == "jumpbutton"
                    )
            ));
            assert!(matches!(
                then_branch.first(),
                Some(LoweredLogicStatement::FunctionCall { name, args })
                    if name == "playerJump" && args.is_empty()
            ));
        }
        other => panic!("expected conditional lowered from code action args, got {other:?}"),
    }
}

#[test]
fn built_gold_sample_preserves_player_step_jump_calls_in_lowered_logic() {
    let sample_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("runtime")
        .join("public")
        .join("packages")
        .join("sample");

    let lowered_path = sample_dir.join("logic.lowered.json");
    let objects_path = sample_dir.join("objects.json");
    if !lowered_path.exists() || !objects_path.exists() {
        return;
    }

    let lowered: iwm_parser::LoweredLogicFile =
        serde_json::from_str(&fs::read_to_string(lowered_path).unwrap()).unwrap();
    let objects: Vec<ObjectDefinition> =
        serde_json::from_str(&fs::read_to_string(objects_path).unwrap()).unwrap();

    let player = objects
        .iter()
        .find(|object| object.name == "player")
        .expect("expected player object");
    let step_block_id = player
        .events
        .iter()
        .find(|event| event.event_tag == "step")
        .map(|event| event.block_id.as_str())
        .expect("expected player step block");

    let step_entry = lowered
        .entries
        .iter()
        .find(|entry| entry.block_id == step_block_id)
        .expect("expected lowered step entry");

    assert!(
        !step_entry.statements.is_empty(),
        "player step entry should not be empty after lowering"
    );

    let lowered_json = serde_json::to_string(step_entry).unwrap();
    assert!(lowered_json.contains("keyboard_check_pressed"));
    assert!(lowered_json.contains("playerJump"));
    assert!(lowered_json.contains("keyboard_check_released"));
    assert!(lowered_json.contains("playerVJump"));
}

#[test]
fn fully_lowered_source_only_blocks_do_not_emit_missing_source_lowering_warning() {
    use iwm_parser::gml_lowering::lower_raw_logic_file;
    use iwm_parser::models::{RawLogicFile, RawLogicOwner, RawLogicOwnerKind};

    let raw = RawLogicFile {
        format: "iwm-raw-logic-v1".to_string(),
        room_creation_codes: vec![],
        instance_creation_codes: vec![RawLogicOwner {
            owner_kind: RawLogicOwnerKind::RoomInstance,
            owner_id: 1001,
            owner_name: "obj_exit".to_string(),
            event_type: None,
            sub_event: None,
            collision_object_id: None,
            block_id: "room:7:instance:1001:create".to_string(),
            gml_source: "roomTo=room8".to_string(),
        }],
        object_events: vec![],
        scripts: vec![],
        triggers: vec![],
        timelines: vec![],
    };

    let lowered = lower_raw_logic_file(&raw);
    let still_has_raw = lowered
        .entries
        .iter()
        .find(|entry| entry.block_id == "room:7:instance:1001:create")
        .unwrap()
        .statements
        .iter()
        .any(|statement| matches!(statement, iwm_parser::LoweredLogicStatement::Raw { .. }));

    assert!(!still_has_raw);
}
