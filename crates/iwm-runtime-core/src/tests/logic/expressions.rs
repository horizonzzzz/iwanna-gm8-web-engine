use super::*;

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
