use iwm_runtime_host::{ButtonState, RuntimeButton};
use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

use super::support::{
    add_alarm_block, add_collision_block, add_create_block, add_keyboard_block,
    add_keyboard_press_block, add_keyboard_release_block, host, sample_package,
};
use crate::event_dispatch::{object_event_block_ids, RuntimeEventSelector};

#[test]
fn core_dispatches_held_keyboard_event_blocks() {
    let mut package = sample_package();
    add_keyboard_block(
        &mut package,
        0x41,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("held_key".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x41),
        ButtonState {
            pressed: true,
            just_pressed: false,
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
    assert_eq!(player.vars.get("held_key"), Some(&RuntimeValue::Bool(true)));
}

#[test]
fn core_dispatches_keyboard_press_event_blocks() {
    let mut package = sample_package();
    add_keyboard_press_block(
        &mut package,
        0x41,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("press_edge".into()),
            value: LoweredLogicExpr::LiteralBool(true),
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

    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("press_edge"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_does_not_retrigger_keyboard_press_events_for_held_input() {
    let mut package = sample_package();
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("press_count".into()),
            value: LoweredLogicExpr::LiteralNumber(0.0),
        }],
    );
    add_keyboard_press_block(
        &mut package,
        0x41,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("press_count".into()),
            value: LoweredLogicExpr::BinaryExpr {
                op: "+".into(),
                left: Box::new(LoweredLogicExpr::Identifier("press_count".into())),
                right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
            },
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
    assert_eq!(player.vars.get("press_count"), Some(&RuntimeValue::Number(1.0)));
}

#[test]
fn core_dispatches_keyboard_release_event_blocks() {
    let mut package = sample_package();
    add_keyboard_release_block(
        &mut package,
        0x41,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("release_edge".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x41),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
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
        player.vars.get("release_edge"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_dispatches_alarm_event_blocks() {
    let mut package = sample_package();
    add_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::IndexAccess {
                target: Box::new(LoweredLogicExpr::Identifier("alarm".into())),
                index: Box::new(LoweredLogicExpr::LiteralNumber(2.0)),
            },
            value: LoweredLogicExpr::LiteralNumber(1.0),
        }],
    );
    add_alarm_block(
        &mut package,
        2,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("alarm_edge".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();
    core.tick(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("alarm_edge"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_falls_back_through_parent_index_for_event_lookup() {
    let mut package = sample_package();
    package.objects[1].parent_index = 0;
    add_keyboard_block(
        &mut package,
        0x41,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("parent_armed".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.input.set_button_state(
        RuntimeButton::Keyboard(0x41),
        ButtonState {
            pressed: true,
            just_pressed: false,
            just_released: false,
        },
    );

    core.tick(&mut host).unwrap();

    let child_instance = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    assert_eq!(
        child_instance.vars.get("parent_armed"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn collision_selector_uses_sub_event_target_object_id() {
    let mut package = sample_package();
    add_collision_block(
        &mut package,
        1,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("collision_hit".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );

    let block_ids = object_event_block_ids(
        &package,
        0,
        RuntimeEventSelector::Collision {
            target_object_id: 1,
        },
    );

    assert_eq!(block_ids, vec!["object:0:event:4:1".to_string()]);
}

#[test]
fn core_dispatches_collision_event_blocks_when_player_overlaps_target() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects[2].name = "block".into();
    add_collision_block(
        &mut package,
        2,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("collision_hit".into()),
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
    assert_eq!(
        player.vars.get("collision_hit"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn collision_event_can_read_other_member_values() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects[2].name = "block".into();
    add_collision_block(
        &mut package,
        2,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("other_y_seen".into()),
                value: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("other".into())),
                    member: "y".into(),
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("other_vspeed_seen".into()),
                value: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("other".into())),
                    member: "vspeed".into(),
                },
            },
        ],
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
    assert_eq!(
        player.vars.get("other_y_seen"),
        Some(&RuntimeValue::Number(40.0))
    );
    assert_eq!(
        player.vars.get("other_vspeed_seen"),
        Some(&RuntimeValue::Number(0.0))
    );
}
