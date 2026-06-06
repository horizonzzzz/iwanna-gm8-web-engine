use iwm_runtime_host::{ButtonState, RuntimeButton, RuntimeFileHost};

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

use super::support::{
    add_alarm_block, add_create_block, add_keyboard_block, add_room_create_block, add_script_block,
    add_step_block, append_lowered_entry, host, real_sample_package, sample_package,
};
use iwm_runtime_model::{ObjectDefinition, ObjectEventEntry};

#[test]
fn core_applies_lowered_create_assignments_to_player_vars_and_movement() {
    let mut package = sample_package();
    package.lowered_logic = Some(crate::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![crate::LoweredLogicEntry {
            block_id: "object:0:event:0:0".into(),
            statements: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("moveSpeed".into()),
                    value: LoweredLogicExpr::LiteralNumber(6.0),
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("jump".into()),
                    value: LoweredLogicExpr::LiteralNumber(11.0),
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("gravity".into()),
                    value: LoweredLogicExpr::LiteralNumber(2.0),
                },
            ],
        }],
    });

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x27),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("moveSpeed"),
        Some(&RuntimeValue::Number(6.0))
    );
    assert_eq!(player.vars.get("jump"), Some(&RuntimeValue::Number(11.0)));
    assert_eq!(player.hspeed, 6.0);
    assert!(player.vspeed >= 0.0);
}

#[test]
fn core_applies_lowered_room_creation_assignments_to_globals() {
    let mut package = sample_package();
    add_room_create_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "difficulty".into(),
                },
                value: LoweredLogicExpr::LiteralNumber(2.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "practice".into(),
                },
                value: LoweredLogicExpr::LiteralBool(true),
            },
        ],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.difficulty"),
        Some(&RuntimeValue::Number(2.0))
    );
    assert_eq!(
        core.globals.get("global.practice"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_executes_lowered_step_room_goto_calls() {
    let mut package = sample_package();
    package.rooms[0].instances[0].creation_block_id = None;
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "room_goto".into(),
            args: vec![LoweredLogicExpr::LiteralNumber(9.0)],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(core.snapshot().room_id, Some(9));
}

#[test]
fn core_applies_lowered_step_assignments_before_player_movement() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("moveSpeed".into()),
            value: LoweredLogicExpr::LiteralNumber(6.0),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x27),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("moveSpeed"),
        Some(&RuntimeValue::Number(6.0))
    );
    assert_eq!(player.hspeed, 6.0);
}

#[test]
fn core_updates_active_view_from_lowered_camera_step() {
    let mut package = sample_package();
    package.rooms[0].width = 2400;
    package.rooms[0].height = 1824;
    package.rooms[0].views_enabled = true;
    package.rooms[0].views[0].visible = true;
    package.rooms[0].views[0].source_w = 800;
    package.rooms[0].views[0].source_h = 600;
    package.rooms[0].views[0].port_w = 800;
    package.rooms[0].views[0].port_h = 600;
    package.rooms[0].instances[0].x = 812;
    package.rooms[0].instances[0].y = 624;
    for instance in &mut package.rooms[0].instances {
        instance.is_checkpoint = false;
    }
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("view_xview".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "*".into(),
                    left: Box::new(LoweredLogicExpr::Call {
                        name: "floor".into(),
                        args: vec![LoweredLogicExpr::BinaryExpr {
                            op: "/".into(),
                            left: Box::new(LoweredLogicExpr::MemberAccess {
                                target: Box::new(LoweredLogicExpr::Identifier("obj_player".into())),
                                member: "x".into(),
                            }),
                            right: Box::new(LoweredLogicExpr::LiteralNumber(800.0)),
                        }],
                    }),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(800.0)),
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("view_yview".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "*".into(),
                    left: Box::new(LoweredLogicExpr::Call {
                        name: "floor".into(),
                        args: vec![LoweredLogicExpr::BinaryExpr {
                            op: "/".into(),
                            left: Box::new(LoweredLogicExpr::MemberAccess {
                                target: Box::new(LoweredLogicExpr::Identifier("obj_player".into())),
                                member: "y".into(),
                            }),
                            right: Box::new(LoweredLogicExpr::LiteralNumber(608.0)),
                        }],
                    }),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(608.0)),
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    assert_eq!(room.views[0].source_x, 800);
    assert_eq!(room.views[0].source_y, 608);
    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!(frame.width, 800);
    assert_eq!(frame.height, 600);
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        iwm_runtime_host::RuntimeDrawCommand::DrawSprite {
            sprite_id: 0,
            x: 12,
            y: 17,
            ..
        }
    )));
}

#[test]
fn core_executes_lowered_step_game_restart_calls() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "game_restart".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x27),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!((player.x, player.y), (12.0, 24.0));
    assert_eq!(core.snapshot().status, crate::RuntimeStatus::Ready);
}

#[test]
fn core_applies_structured_member_assignment_and_binary_value_to_globals() {
    let mut package = sample_package();
    add_room_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "difficulty".into(),
            },
            value: LoweredLogicExpr::BinaryExpr {
                op: "+".into(),
                left: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
            },
        }],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.difficulty"),
        Some(&RuntimeValue::Number(2.0))
    );
}

#[test]
fn core_executes_room_goto_from_structured_binary_expression_argument() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "room_goto".into(),
            args: vec![LoweredLogicExpr::BinaryExpr {
                op: "+".into(),
                left: Box::new(LoweredLogicExpr::LiteralNumber(4.0)),
                right: Box::new(LoweredLogicExpr::LiteralNumber(5.0)),
            }],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(core.snapshot().room_id, Some(9));
}

#[test]
fn core_dispatches_keyboard_event_blocks_for_non_hardcoded_keys() {
    let mut package = sample_package();
    add_keyboard_block(
        &mut package,
        0x42,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("armed".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x42),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("armed"), Some(&RuntimeValue::Bool(true)));
}

#[test]
fn core_dispatches_keyboard_event_blocks_for_hex_named_keys() {
    let mut package = sample_package();
    add_keyboard_block(
        &mut package,
        0x25,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("hex_key_armed".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x25),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("hex_key_armed"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_dispatches_alarm_events_for_nonzero_alarm_slots() {
    let mut package = sample_package();
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::IndexAccess {
                target: Box::new(LoweredLogicExpr::Identifier("alarm".into())),
                index: Box::new(LoweredLogicExpr::LiteralNumber(3.0)),
            },
            value: LoweredLogicExpr::LiteralNumber(1.0),
        }],
    );
    add_alarm_block(
        &mut package,
        3,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("alarm_fired".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();
    host.input.clear_transitions();
    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("alarm_fired"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_preserves_instance_field_assignments_from_create_blocks() {
    let mut package = sample_package();
    add_create_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("x".into()),
                value: LoweredLogicExpr::LiteralNumber(99.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("y".into()),
                value: LoweredLogicExpr::LiteralNumber(77.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("hspeed".into()),
                value: LoweredLogicExpr::LiteralNumber(5.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("vspeed".into()),
                value: LoweredLogicExpr::LiteralNumber(-2.0),
            },
        ],
    );

    let core = RuntimeCore::load(package).unwrap();
    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();

    assert_eq!(
        (player.x, player.y, player.hspeed, player.vspeed),
        (99.0, 77.0, 5.0, -2.0)
    );
}

#[test]
fn core_dispatches_keyboard_and_alarm_blocks_through_shared_event_path() {
    let mut package = sample_package();
    add_keyboard_block(
        &mut package,
        0x41,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("armed".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    add_alarm_block(
        &mut package,
        0,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("alarm_fired".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    // Set alarm[0] to 1 - it will fire after first decrement
    package.objects[0].events.push(ObjectEventEntry {
        event_type: 0,
        sub_event: 0,
        event_tag: "create".into(),
        block_id: "object:0:event:0:0".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:0:event:0:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::IndexAccess {
                target: Box::new(LoweredLogicExpr::Identifier("alarm".into())),
                index: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
            },
            value: LoweredLogicExpr::LiteralNumber(1.0),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x41),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    // First tick: create event sets alarm[0]=1, keyboard event fires (armed=true)
    core.tick(&mut host).unwrap();
    // Clear transitions after first tick
    host.input.clear_transitions();

    // Second tick: alarm fires (alarm_fired=true)
    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("armed"), Some(&RuntimeValue::Bool(true)));
    assert_eq!(
        player.vars.get("alarm_fired"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_executes_conditional_branch_before_room_transition() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "==".into(),
                left: Box::new(LoweredLogicExpr::Identifier("can_exit".into())),
                right: Box::new(LoweredLogicExpr::LiteralBool(true)),
            },
            then_branch: vec![LoweredLogicStatement::FunctionCall {
                name: "room_goto".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(9.0)],
            }],
            else_branch: vec![],
        }],
    );
    // Add create block to set can_exit = true
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("can_exit".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    assert_eq!(core.snapshot().room_id, Some(9));
}

#[test]
fn core_evaluates_unary_negative_in_assignments() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("score".into()),
            value: LoweredLogicExpr::UnaryExpr {
                op: "-".into(),
                child: Box::new(LoweredLogicExpr::Identifier("y".into())),
            },
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("score"), Some(&RuntimeValue::Number(-24.0)));
}

#[test]
fn core_evaluates_unary_not_in_conditionals() {
    let mut package = sample_package();
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("flag".into()),
            value: LoweredLogicExpr::LiteralBool(false),
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::UnaryExpr {
                op: "!".into(),
                child: Box::new(LoweredLogicExpr::Identifier("flag".into())),
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("armed".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("armed"), Some(&RuntimeValue::Bool(true)));
}

#[test]
fn core_executes_zero_arg_script_calls_from_lowered_step_events() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "playerVJump".into(),
            args: vec![],
        }],
    );
    add_script_block(
        &mut package,
        13,
        "playerVJump",
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("vspeed".into()),
            value: LoweredLogicExpr::BinaryExpr {
                op: "*".into(),
                left: Box::new(LoweredLogicExpr::Identifier("vspeed".into())),
                right: Box::new(LoweredLogicExpr::LiteralNumber(0.5)),
            },
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    {
        let room = core.current_room.as_mut().unwrap();
        let player = room
            .instances
            .iter_mut()
            .find(|instance| instance.player_candidate)
            .unwrap();
        player.vspeed = -8.0;
    }

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vspeed, -4.0);
}

#[test]
fn core_preserves_script_applied_vspeed_without_step_player_overwrite() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("vspeed".into()),
            value: LoweredLogicExpr::LiteralNumber(-4.0),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vspeed, -4.0);
}

#[test]
fn core_evaluates_keyboard_query_calls_against_host_button_state() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "jumpbutton".into(),
                },
                value: LoweredLogicExpr::LiteralNumber(0x20 as f64),
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "keyboard_check_pressed".into(),
                    args: vec![LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                        member: "jumpbutton".into(),
                    }],
                },
                then_branch: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("pressed".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                }],
                else_branch: vec![],
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "keyboard_check".into(),
                    args: vec![LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                        member: "jumpbutton".into(),
                    }],
                },
                then_branch: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("held".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                }],
                else_branch: vec![],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x20),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("pressed"), Some(&RuntimeValue::Bool(true)));
    assert_eq!(player.vars.get("held"), Some(&RuntimeValue::Bool(true)));

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x20),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
        },
    );
    add_step_block(
        &mut core.package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "keyboard_check_released".into(),
                args: vec![LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "jumpbutton".into(),
                }],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("released".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );
    core.lowered_logic_index = core
        .package
        .lowered_logic
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.block_id.clone(), index))
        .collect();

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("released"), Some(&RuntimeValue::Bool(true)));
}

#[test]
fn core_evaluates_place_queries_and_boolean_operators_inside_conditions() {
    let mut package = sample_package();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "platform".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: false,
        solid: true,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![],
    });
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 16,
            object_id: 4,
            x: 12,
            y: 40,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: true,
            is_hazard: false,
            is_checkpoint: false,
        });
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "&&".into(),
                left: Box::new(LoweredLogicExpr::Call {
                    name: "place_meeting".into(),
                    args: vec![
                        LoweredLogicExpr::Identifier("x".into()),
                        LoweredLogicExpr::BinaryExpr {
                            op: "+".into(),
                            left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                            right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                        },
                        LoweredLogicExpr::Identifier("platform".into()),
                    ],
                }),
                right: Box::new(LoweredLogicExpr::BinaryExpr {
                    op: "||".into(),
                    left: Box::new(LoweredLogicExpr::Call {
                        name: "place_free".into(),
                        args: vec![
                            LoweredLogicExpr::Identifier("x".into()),
                            LoweredLogicExpr::Identifier("y".into()),
                            LoweredLogicExpr::Identifier("platform".into()),
                        ],
                    }),
                    right: Box::new(LoweredLogicExpr::LiteralBool(false)),
                }),
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("query_ok".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("query_ok"), Some(&RuntimeValue::Bool(true)));
}

#[test]
fn core_preserves_fractional_motion_assignments_from_lowered_logic() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("vspeed".into()),
                value: LoweredLogicExpr::LiteralNumber(-8.5),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("hspeed".into()),
                value: LoweredLogicExpr::LiteralNumber(3.25),
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(((player.vspeed as f64) - -8.5).abs() < f64::EPSILON);
    assert!(((player.hspeed as f64) - 3.25).abs() < f64::EPSILON);
}

#[test]
fn core_executes_lowered_step_room_goto_next_calls() {
    let mut package = sample_package();
    package.rooms.push(iwm_runtime_model::RoomDefinition {
        id: 11,
        name: "room11".into(),
        width: 160,
        height: 120,
        speed: 60,
        persistent: false,
        backgrounds: vec![],
        views_enabled: false,
        views: vec![],
        tiles: vec![],
        instances: vec![],
        creation_block_id: None,
        playable: true,
        transition_targets: vec![],
    });
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "room_goto_next".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(core.snapshot().room_id, Some(9));
}

#[test]
fn core_executes_room_goto_next_using_manifest_room_order() {
    let mut package = sample_package();
    package.manifest.room_order = vec![7, 11, 9];
    package.rooms.push(iwm_runtime_model::RoomDefinition {
        id: 11,
        name: "room11".into(),
        width: 160,
        height: 120,
        speed: 60,
        persistent: false,
        backgrounds: vec![],
        views_enabled: false,
        views: vec![],
        tiles: vec![],
        instances: vec![],
        creation_block_id: None,
        playable: true,
        transition_targets: vec![],
    });
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "room_goto_next".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(core.snapshot().room_id, Some(11));
}

#[test]
fn core_evaluates_file_exists_conditions_against_host_files() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "==".into(),
                left: Box::new(LoweredLogicExpr::Call {
                    name: "file_exists".into(),
                    args: vec![LoweredLogicExpr::LiteralText("temp".into())],
                }),
                right: Box::new(LoweredLogicExpr::LiteralBool(true)),
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("loaded_temp".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("loaded_temp".into()),
                value: LoweredLogicExpr::LiteralBool(false),
            }],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.files
        .write_temp(std::path::Path::new("temp"), b"checkpoint")
        .unwrap();

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("loaded_temp"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_does_not_inject_player_into_rooms_without_spawn_logic() {
    let mut package = sample_package();
    package.manifest.default_room_id = Some(11);
    package.rooms.push(iwm_runtime_model::RoomDefinition {
        id: 11,
        name: "menu_like_room".into(),
        width: 320,
        height: 240,
        speed: 60,
        persistent: false,
        backgrounds: vec![],
        views_enabled: false,
        views: vec![],
        tiles: vec![],
        instances: vec![iwm_runtime_model::RoomInstancePlacement {
            instance_id: 99,
            object_id: 1,
            x: 120,
            y: 80,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: false,
            is_checkpoint: false,
        }],
        creation_block_id: None,
        playable: false,
        transition_targets: vec![],
    });

    let core = RuntimeCore::load(package).unwrap();

    assert!(core.snapshot().player.is_none());
}

#[test]
fn core_dispatches_room_start_events_after_room_build() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .retain(|instance| instance.object_id != 0);
    package.objects[1]
        .events
        .push(iwm_runtime_model::ObjectEventEntry {
            event_type: 7,
            sub_event: 4,
            event_tag: "other:room-start".into(),
            block_id: "object:1:event:7:4".into(),
            action_count: 1,
        });
    package.scripts.blocks.push(iwm_runtime_model::LogicBlock {
        id: "object:1:event:7:4".into(),
        name: "marker room start".into(),
        kind: "object-event".into(),
        support: "source-only".into(),
        executable_action_count: 0,
        ops: vec![],
    });
    package.lowered_logic = Some(crate::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![crate::LoweredLogicEntry {
            block_id: "object:1:event:7:4".into(),
            statements: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_create".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("x".into()),
                    LoweredLogicExpr::Identifier("y".into()),
                    LoweredLogicExpr::Identifier("obj_player".into()),
                ],
            }],
        }],
    });

    let core = RuntimeCore::load(package).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_player")
        .expect("room-start event should create player");
    assert_eq!((player.x, player.y), (48.0, 64.0));
}

#[test]
fn core_skips_builtin_jump_when_step_scripts_own_jump_queries() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "jumpbutton".into(),
                },
                value: LoweredLogicExpr::LiteralNumber(0x10 as f64),
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "keyboard_check_pressed".into(),
                    args: vec![LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                        member: "jumpbutton".into(),
                    }],
                },
                then_branch: vec![LoweredLogicStatement::FunctionCall {
                    name: "playerJump".into(),
                    args: vec![],
                }],
                else_branch: vec![],
            },
        ],
    );
    add_script_block(
        &mut package,
        11,
        "playerJump",
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump_pressed_seen".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("vspeed".into()),
                value: LoweredLogicExpr::LiteralNumber(-3.0),
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("jump_pressed_seen"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(player.vspeed < 0.0);
    assert!(player.vspeed > -8.0);
    assert_eq!(player.jump.active, false);
}

#[test]
fn core_applies_room_create_script_calls_to_globals_for_control_bootstrap() {
    let mut package = sample_package();
    add_room_create_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "defControls".into(),
            args: vec![],
        }],
    );
    add_script_block(
        &mut package,
        16,
        "defControls",
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "jumpbutton".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0x10 as f64),
        }],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.jumpbutton"),
        Some(&RuntimeValue::Number(0x10 as f64))
    );
}

#[test]
fn place_meeting_matches_instances_through_parent_object_inheritance() {
    let mut package = sample_package();
    package.objects[1].name = "block".into();
    package.objects[1].parent_index = -1;
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "slipblock".into(),
        sprite_index: -1,
        parent_index: 1,
        depth: 0,
        persistent: false,
        visible: false,
        solid: true,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![],
    });
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 16,
            object_id: 4,
            x: 12,
            y: 40,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: true,
            is_hazard: false,
            is_checkpoint: false,
        });
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "place_meeting".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("x".into()),
                    LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    },
                    LoweredLogicExpr::Identifier("block".into()),
                ],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("inherited_ground".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("inherited_ground"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn script_owned_jump_can_retrigger_after_collision_restores_djump() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects[2].name = "block".into();
    add_create_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump".into()),
                value: LoweredLogicExpr::LiteralNumber(8.5),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump2".into()),
                value: LoweredLogicExpr::LiteralNumber(7.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("djump".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump_count".into()),
                value: LoweredLogicExpr::LiteralNumber(0.0),
            },
        ],
    );
    package.objects[0].events.push(ObjectEventEntry {
        event_type: 4,
        sub_event: 2,
        event_tag: "collision".into(),
        block_id: "object:0:event:4:2".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:0:event:4:2".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("djump".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "keyboard_check_pressed".into(),
                args: vec![LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "jumpbutton".into(),
                }],
            },
            then_branch: vec![LoweredLogicStatement::FunctionCall {
                name: "playerJump".into(),
                args: vec![],
            }],
            else_branch: vec![],
        }],
    );
    add_script_block(
        &mut package,
        11,
        "playerJump",
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "==".into(),
                left: Box::new(LoweredLogicExpr::Identifier("djump".into())),
                right: Box::new(LoweredLogicExpr::LiteralBool(true)),
            },
            then_branch: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("djump".into()),
                    value: LoweredLogicExpr::LiteralBool(false),
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("jump_count".into()),
                    value: LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("jump_count".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    },
                },
            ],
            else_branch: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.globals.insert(
        "global.jumpbutton".into(),
        RuntimeValue::Number(0x10 as f64),
    );

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    {
        let player = core
            .current_room()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.player_candidate)
            .unwrap();
        assert_eq!(
            player.vars.get("jump_count"),
            Some(&RuntimeValue::Number(1.0))
        );
        assert_eq!(player.vars.get("djump"), Some(&RuntimeValue::Bool(false)));
    }

    {
        let room = core.current_room.as_mut().unwrap();
        let player = room
            .instances
            .iter_mut()
            .find(|instance| instance.player_candidate)
            .unwrap();
        player.y = 48.0;
        player.previous_y = 48.0;
        player.vspeed = 8.0;
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    {
        let player = core
            .current_room()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.player_candidate)
            .unwrap();
        assert_eq!(player.vars.get("djump"), Some(&RuntimeValue::Bool(true)));
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("jump_count"),
        Some(&RuntimeValue::Number(2.0))
    );
}

#[test]
fn create_logic_instance_create_bootstraps_world_globals_immediately() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "world".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: true,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![ObjectEventEntry {
            event_type: 0,
            sub_event: 0,
            event_tag: "create".into(),
            block_id: "object:4:event:0:0".into(),
            action_count: 0,
        }],
    });
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "==".into(),
                left: Box::new(LoweredLogicExpr::Call {
                    name: "instance_exists".into(),
                    args: vec![LoweredLogicExpr::Identifier("world".into())],
                }),
                right: Box::new(LoweredLogicExpr::LiteralBool(false)),
            },
            then_branch: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_create".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::Identifier("world".into()),
                ],
            }],
            else_branch: vec![],
        }],
    );
    append_lowered_entry(
        &mut package,
        "object:4:event:0:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "grav".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0.0),
        }],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.grav"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert!(core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .any(|instance| instance.object_name == "world"));
}

#[test]
fn script_owned_jump_uses_grounded_branch_after_world_grav_bootstrap() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects[2].name = "block".into();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "world".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: true,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![ObjectEventEntry {
            event_type: 0,
            sub_event: 0,
            event_tag: "create".into(),
            block_id: "object:4:event:0:0".into(),
            action_count: 0,
        }],
    });
    add_create_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump".into()),
                value: LoweredLogicExpr::LiteralNumber(8.5),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump2".into()),
                value: LoweredLogicExpr::LiteralNumber(7.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("djump".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::BinaryExpr {
                    op: "==".into(),
                    left: Box::new(LoweredLogicExpr::Call {
                        name: "instance_exists".into(),
                        args: vec![LoweredLogicExpr::Identifier("world".into())],
                    }),
                    right: Box::new(LoweredLogicExpr::LiteralBool(false)),
                },
                then_branch: vec![LoweredLogicStatement::FunctionCall {
                    name: "instance_create".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralNumber(0.0),
                        LoweredLogicExpr::LiteralNumber(0.0),
                        LoweredLogicExpr::Identifier("world".into()),
                    ],
                }],
                else_branch: vec![],
            },
        ],
    );
    append_lowered_entry(
        &mut package,
        "object:4:event:0:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "grav".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0.0),
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "keyboard_check_pressed".into(),
                args: vec![LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "jumpbutton".into(),
                }],
            },
            then_branch: vec![LoweredLogicStatement::FunctionCall {
                name: "playerJump".into(),
                args: vec![],
            }],
            else_branch: vec![],
        }],
    );
    add_script_block(
        &mut package,
        11,
        "playerJump",
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "=".into(),
                left: Box::new(LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                    member: "grav".into(),
                }),
                right: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
            },
            then_branch: vec![LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "place_meeting".into(),
                    args: vec![
                        LoweredLogicExpr::Identifier("x".into()),
                        LoweredLogicExpr::BinaryExpr {
                            op: "+".into(),
                            left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                            right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                        },
                        LoweredLogicExpr::Identifier("block".into()),
                    ],
                },
                then_branch: vec![
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("jump_branch".into()),
                        value: LoweredLogicExpr::LiteralText("ground".into()),
                    },
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("vspeed".into()),
                        value: LoweredLogicExpr::UnaryExpr {
                            op: "-".into(),
                            child: Box::new(LoweredLogicExpr::Identifier("jump".into())),
                        },
                    },
                ],
                else_branch: vec![LoweredLogicStatement::Conditional {
                    condition: LoweredLogicExpr::BinaryExpr {
                        op: "==".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("djump".into())),
                        right: Box::new(LoweredLogicExpr::LiteralBool(true)),
                    },
                    then_branch: vec![
                        LoweredLogicStatement::Assignment {
                            target: LoweredLogicExpr::Identifier("jump_branch".into()),
                            value: LoweredLogicExpr::LiteralText("air".into()),
                        },
                        LoweredLogicStatement::Assignment {
                            target: LoweredLogicExpr::Identifier("djump".into()),
                            value: LoweredLogicExpr::LiteralBool(false),
                        },
                    ],
                    else_branch: vec![],
                }],
            }],
            else_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("jump_branch".into()),
                value: LoweredLogicExpr::LiteralText("reverse".into()),
            }],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.globals.insert(
        "global.jumpbutton".into(),
        RuntimeValue::Number(0x10 as f64),
    );
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("jump_branch"),
        Some(&RuntimeValue::Text("ground".into()))
    );
}

#[test]
fn collision_block_event_restores_djump_after_landing() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects[2].name = "block".into();
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("djump".into()),
            value: LoweredLogicExpr::LiteralBool(false),
        }],
    );
    package.objects[0].events.push(ObjectEventEntry {
        event_type: 4,
        sub_event: 2,
        event_tag: "collision".into(),
        block_id: "object:0:event:4:2".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:0:event:4:2".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("djump".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    {
        let room = core.current_room.as_mut().unwrap();
        let player = room
            .instances
            .iter_mut()
            .find(|instance| instance.player_candidate)
            .unwrap();
        player.y = 48.0;
        player.previous_y = 48.0;
        player.vspeed = 8.0;
    }

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("djump"), Some(&RuntimeValue::Bool(true)));
}

#[test]
fn real_sample_second_shift_press_lacks_bootstrap_globals_after_manual_room_reload() {
    let Some(mut package) = real_sample_package() else {
        return;
    };

    if let Some(lowered) = package.lowered_logic.as_mut() {
        if let Some(step_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "object:0:event:3:0")
        {
            if let Some(jump_cond_index) = step_entry.statements.iter().position(|statement| {
                matches!(
                    statement,
                    LoweredLogicStatement::Conditional {
                        condition: LoweredLogicExpr::Call { name, args },
                        ..
                    } if name == "keyboard_check_pressed"
                        && matches!(
                            args.first(),
                            Some(LoweredLogicExpr::MemberAccess { target, member })
                                if member == "jumpbutton"
                                    && matches!(target.as_ref(), LoweredLogicExpr::Identifier(name) if name == "global")
                        )
                )
            }) {
                step_entry.statements.insert(
                    jump_cond_index + 1,
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("debug_after_jump_cond_vspeed".into()),
                        value: LoweredLogicExpr::Identifier("vspeed".into()),
                    },
                );
            }
            step_entry.statements.insert(
                0,
                LoweredLogicStatement::Conditional {
                    condition: LoweredLogicExpr::Call {
                        name: "keyboard_check_pressed".into(),
                        args: vec![LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                            member: "jumpbutton".into(),
                        }],
                    },
                    then_branch: vec![LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("debug_step_jump_pressed".into()),
                        value: LoweredLogicExpr::LiteralBool(true),
                    }],
                    else_branch: vec![],
                },
            );
        }

        if let Some(player_jump_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "script:11")
        {
            player_jump_entry.statements.insert(
                0,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_player_jump_called".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                },
            );
            player_jump_entry.statements.insert(
                1,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_ground_block".into()),
                    value: LoweredLogicExpr::Call {
                        name: "place_meeting".into(),
                        args: vec![
                            LoweredLogicExpr::Identifier("x".into()),
                            LoweredLogicExpr::BinaryExpr {
                                op: "+".into(),
                                left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                                right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                            },
                            LoweredLogicExpr::Identifier("block".into()),
                        ],
                    },
                },
            );
            player_jump_entry.statements.insert(
                2,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_ground_solidblock".into()),
                    value: LoweredLogicExpr::Call {
                        name: "place_meeting".into(),
                        args: vec![
                            LoweredLogicExpr::Identifier("x".into()),
                            LoweredLogicExpr::BinaryExpr {
                                op: "+".into(),
                                left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                                right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                            },
                            LoweredLogicExpr::Identifier("solidblock".into()),
                        ],
                    },
                },
            );
            player_jump_entry.statements.insert(
                3,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_pre_djump".into()),
                    value: LoweredLogicExpr::Identifier("djump".into()),
                },
            );
            player_jump_entry.statements.insert(
                4,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_pre_onPlatform".into()),
                    value: LoweredLogicExpr::Identifier("onPlatform".into()),
                },
            );
            if let Some(LoweredLogicStatement::Conditional { then_branch, .. }) =
                player_jump_entry.statements.get_mut(5)
            {
                if let Some(LoweredLogicStatement::Conditional {
                    then_branch: jump_ground_branch,
                    else_branch: jump_ground_else,
                    ..
                }) = then_branch.get_mut(0)
                {
                    jump_ground_branch.insert(
                        0,
                        LoweredLogicStatement::Assignment {
                            target: LoweredLogicExpr::Identifier(
                                "debug_ground_branch_taken".into(),
                            ),
                            value: LoweredLogicExpr::LiteralBool(true),
                        },
                    );
                    if let Some(LoweredLogicStatement::Conditional {
                        then_branch: jump_air_branch,
                        ..
                    }) = jump_ground_else.get_mut(0)
                    {
                        jump_air_branch.insert(
                            0,
                            LoweredLogicStatement::Assignment {
                                target: LoweredLogicExpr::Identifier(
                                    "debug_air_branch_taken".into(),
                                ),
                                value: LoweredLogicExpr::LiteralBool(true),
                            },
                        );
                    }
                }
            }
        }
    }

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
    }
    assert_eq!(core.snapshot().input_trace.jump_button_key, 0x10);

    core.reload_room(143).unwrap();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        let snapshot = core.snapshot();
        if snapshot
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }
    assert!(core
        .snapshot()
        .player
        .as_ref()
        .map(|player| player.jump.grounded)
        .unwrap_or(false));

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
        },
    );
    core.tick(&mut host).unwrap();

    host.input.clear_transitions();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: false,
        },
    );
    for _ in 0..180 {
        core.tick(&mut host).unwrap();
        if core
            .snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.tick(&mut host).unwrap();

    let after_second = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| crate::helpers::is_player_instance(instance))
        .unwrap();

    assert_eq!(
        core.globals.get("global.grav"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert_eq!(
        after_second.vars.get("debug_step_jump_pressed"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        after_second.vars.get("debug_player_jump_called"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        after_second.vars.get("debug_ground_block"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        after_second.vars.get("debug_ground_branch_taken"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(
        after_second.vspeed < 0.0,
        "second jump should produce upward vspeed once bootstrap globals exist, got {:?}",
        after_second.vspeed
    );
}

#[test]
fn real_sample_step_events_alone_show_second_shift_playerjump_effect() {
    let Some(mut package) = real_sample_package() else {
        return;
    };

    if let Some(lowered) = package.lowered_logic.as_mut() {
        if let Some(player_jump_entry) = lowered
            .entries
            .iter_mut()
            .find(|entry| entry.block_id == "script:11")
        {
            player_jump_entry.statements.insert(
                0,
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("debug_player_jump_called".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                },
            );
        }
    }

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core.snapshot().input_trace.jump_button_key == 0x10 {
            break;
        }
    }
    assert_eq!(core.snapshot().input_trace.jump_button_key, 0x10);

    core.reload_room(143).unwrap();
    for _ in 0..120 {
        core.tick(&mut host).unwrap();
        if core
            .snapshot()
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
    }

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x10),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    );
    core.execute_lowered_step_events(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| crate::helpers::is_player_instance(instance))
        .unwrap();

    assert_eq!(
        player.vars.get("debug_player_jump_called"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(
        player.vspeed < 0.0,
        "step events alone should apply upward jump velocity, got {:?}",
        player.vspeed
    );
}
