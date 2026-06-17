use super::*;

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
fn core_honors_instance_destroy_during_room_create_events() {
    let mut package = sample_package();
    package.lowered_logic = Some(crate::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![crate::LoweredLogicEntry {
            block_id: "object:0:event:0:0".into(),
            statements: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_destroy".into(),
                args: vec![],
            }],
        }],
    });

    let core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_player")
        .expect("sample package should include obj_player");

    assert!(
        !player.alive,
        "Create-time instance_destroy should remove room-placed instances from live runtime participation"
    );
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
