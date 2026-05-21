use iwm_runtime_host::{ButtonState, RuntimeButton};

use crate::{LoweredLogicStatement, RuntimeCore, RuntimeValue};

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
                    lhs: "moveSpeed".into(),
                    rhs: "6".into(),
                },
                LoweredLogicStatement::Assignment {
                    lhs: "jump".into(),
                    rhs: "11".into(),
                },
                LoweredLogicStatement::Assignment {
                    lhs: "gravity".into(),
                    rhs: "2".into(),
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
                lhs: "global.difficulty".into(),
                rhs: "2".into(),
            },
            LoweredLogicStatement::Assignment {
                lhs: "global.practice".into(),
                rhs: "true".into(),
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
            args: vec!["9".into()],
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
            lhs: "moveSpeed".into(),
            rhs: "6".into(),
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
