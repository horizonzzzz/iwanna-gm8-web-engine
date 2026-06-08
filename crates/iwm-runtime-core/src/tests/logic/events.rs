use super::*;

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
