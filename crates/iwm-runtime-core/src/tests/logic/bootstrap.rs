use super::*;
use iwm_runtime_model::{PathPointResource, PathResource};

#[test]
fn creation_code_path_start_initializes_and_moves_instance_on_tick() {
    let mut package = sample_package();
    package.resources.paths.push(PathResource {
        id: 4,
        name: "pathCrimson".into(),
        smooth: false,
        precision: 4,
        closed: false,
        points: vec![
            PathPointResource {
                x: 0.0,
                y: 0.0,
                speed: 100.0,
            },
            PathPointResource {
                x: 100.0,
                y: 0.0,
                speed: 100.0,
            },
        ],
    });
    package.rooms[0].instances[1].creation_block_id = Some("instance:1:create".into());
    append_lowered_entry(
        &mut package,
        "instance:1:create".into(),
        vec![LoweredLogicStatement::FunctionCall {
            name: "path_start".into(),
            args: vec![
                LoweredLogicExpr::Identifier("pathCrimson".into()),
                LoweredLogicExpr::LiteralNumber(10.0),
                LoweredLogicExpr::LiteralNumber(0.0),
                LoweredLogicExpr::LiteralBool(false),
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let initial_x = core.current_room().unwrap().instances[1].x;
    assert_eq!(
        core.current_room().unwrap().instances[1]
            .vars
            .get("path_index"),
        Some(&RuntimeValue::Number(4.0))
    );

    core.tick(&mut host()).unwrap();

    assert_eq!(
        core.current_room().unwrap().instances[1].x,
        initial_x + 10.0
    );
}

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

#[test]
fn core_resolves_named_sprite_constants_in_room_creation_code() {
    let mut package = sample_package();
    package.resources.sprites[1].name = "spr_room_marker".into();
    add_room_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "room_marker_sprite".into(),
            },
            value: LoweredLogicExpr::Identifier("spr_room_marker".into()),
        }],
    );

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(
        core.globals.get("global.room_marker_sprite"),
        Some(&RuntimeValue::Number(1.0))
    );
}
