use iwm_runtime_host::{ButtonState, RuntimeButton};

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

use super::support::{add_room_create_block, add_step_block, host, sample_package};

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
    assert_eq!(player.vars.get("moveSpeed"), Some(&RuntimeValue::Number(6.0)));
    assert_eq!(player.vars.get("jump"), Some(&RuntimeValue::Number(11.0)));
    assert_eq!(player.hspeed, 6);
    assert!(player.vspeed >= 0);
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
    assert_eq!(player.vars.get("moveSpeed"), Some(&RuntimeValue::Number(6.0)));
    assert_eq!(player.hspeed, 6);
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
    assert_eq!((player.x, player.y), (12, 24));
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
