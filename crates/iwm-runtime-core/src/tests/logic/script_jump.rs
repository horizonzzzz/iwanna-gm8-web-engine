use super::*;

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
fn non_solid_platform_top_contact_preserves_walkoff_double_jump() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "platform".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![],
    });
    package.rooms[0].instances.push(RoomInstancePlacement {
        instance_id: 16,
        object_id: 4,
        x: 12,
        y: 40,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: false,
        is_checkpoint: false,
    });
    add_create_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("djump".into()),
                value: LoweredLogicExpr::LiteralBool(false),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("onPlatform".into()),
                value: LoweredLogicExpr::LiteralBool(false),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("air_jump_count".into()),
                value: LoweredLogicExpr::LiteralNumber(0.0),
            },
        ],
    );
    package.objects[0].events.push(ObjectEventEntry {
        event_type: 4,
        sub_event: 4,
        event_tag: "collision".into(),
        block_id: "object:0:event:4:4".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:0:event:4:4".into(),
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("onPlatform".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("djump".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            },
        ],
    );
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::BinaryExpr {
                    op: "==".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("onPlatform".into())),
                    right: Box::new(LoweredLogicExpr::LiteralBool(true)),
                },
                then_branch: vec![LoweredLogicStatement::Conditional {
                    condition: LoweredLogicExpr::BinaryExpr {
                        op: "==".into(),
                        left: Box::new(LoweredLogicExpr::Call {
                            name: "place_meeting".into(),
                            args: vec![
                                LoweredLogicExpr::Identifier("x".into()),
                                LoweredLogicExpr::BinaryExpr {
                                    op: "+".into(),
                                    left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                                    right: Box::new(LoweredLogicExpr::LiteralNumber(4.0)),
                                },
                                LoweredLogicExpr::Identifier("platform".into()),
                            ],
                        }),
                        right: Box::new(LoweredLogicExpr::LiteralBool(false)),
                    },
                    then_branch: vec![LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("onPlatform".into()),
                        value: LoweredLogicExpr::LiteralBool(false),
                    }],
                    else_branch: vec![],
                }],
                else_branch: vec![],
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
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "==".into(),
                left: Box::new(LoweredLogicExpr::Identifier("djump".into())),
                right: Box::new(LoweredLogicExpr::LiteralBool(true)),
            },
            then_branch: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("air_jump_count".into()),
                    value: LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("air_jump_count".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    },
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("djump".into()),
                    value: LoweredLogicExpr::LiteralBool(false),
                },
            ],
            else_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("missed_air_jump".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.globals.insert(
        "global.jumpbutton".into(),
        RuntimeValue::Number(0x10 as f64),
    );
    {
        let player = player_mut(&mut core);
        player.y = 20.0;
        player.previous_y = 20.0;
        player.vspeed = 3.0;
    }

    core.tick(&mut host).unwrap();
    host.input.clear_transitions();

    {
        let player = player_mut(&mut core);
        player.x = 96.0;
        player.previous_x = 96.0;
        player.hspeed = 0.0;
        player.vspeed = 0.0;
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

    let player = player(&core);
    assert_eq!(
        player.vars.get("air_jump_count"),
        Some(&RuntimeValue::Number(1.0)),
        "walking off a just-contacted platform should leave djump available; vars={:?}",
        player.vars
    );
    assert_ne!(
        player.vars.get("missed_air_jump"),
        Some(&RuntimeValue::Bool(true))
    );
}
