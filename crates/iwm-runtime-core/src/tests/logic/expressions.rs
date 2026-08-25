use super::*;

fn tick_package(package: crate::RuntimePackage) -> RuntimeCore {
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();
    core
}

fn run_step(statements: Vec<LoweredLogicStatement>) -> RuntimeCore {
    let mut package = sample_package();
    add_step_block(&mut package, statements);
    tick_package(package)
}

fn assign_var(name: &str, value: LoweredLogicExpr) -> LoweredLogicStatement {
    LoweredLogicStatement::Assignment {
        target: LoweredLogicExpr::Identifier(name.into()),
        value,
    }
}

#[test]
fn core_resolves_named_sprite_constants_in_expressions() {
    let mut package = sample_package();
    package.resources.sprites[1].name = "spr_running".into();
    add_step_block(
        &mut package,
        vec![assign_var(
            "selected_sprite",
            LoweredLogicExpr::Identifier("spr_running".into()),
        )],
    );

    let core = tick_package(package);

    assert_eq!(
        player_var(&core, "selected_sprite"),
        Some(&RuntimeValue::Number(1.0))
    );
}

#[test]
fn core_resolves_named_sound_constants_in_expressions() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "opbgm".into();
    add_step_block(
        &mut package,
        vec![assign_var(
            "stage_bgm",
            LoweredLogicExpr::Identifier("opbgm".into()),
        )],
    );

    let core = tick_package(package);

    assert_eq!(
        player_var(&core, "stage_bgm"),
        Some(&RuntimeValue::Number(42.0))
    );
}

#[test]
fn core_instance_variables_shadow_named_sprite_constants() {
    let mut package = sample_package();
    package.resources.sprites[1].name = "spr_running".into();
    add_create_block(
        &mut package,
        vec![assign_var(
            "spr_running",
            LoweredLogicExpr::LiteralNumber(9.0),
        )],
    );
    add_step_block(
        &mut package,
        vec![assign_var(
            "selected_sprite",
            LoweredLogicExpr::Identifier("spr_running".into()),
        )],
    );

    let core = tick_package(package);

    assert_eq!(
        player_var(&core, "selected_sprite"),
        Some(&RuntimeValue::Number(9.0))
    );
}

#[test]
fn image_alpha_compound_assignment_uses_gm_default() {
    let core = run_step(vec![assign_var(
        "image_alpha",
        LoweredLogicExpr::BinaryExpr {
            op: "-".into(),
            left: Box::new(LoweredLogicExpr::Identifier("image_alpha".into())),
            right: Box::new(LoweredLogicExpr::LiteralNumber(0.02)),
        },
    )]);

    let alpha = player_var(&core, "image_alpha")
        .and_then(crate::helpers::as_number)
        .expect("image_alpha should be numeric");
    assert!((alpha - 0.98).abs() < f64::EPSILON, "alpha={alpha}");
}

#[test]
fn explicit_view_assignment_precedes_live_view_fallback() {
    let core = run_step(vec![
        assign_var("view_xview", LoweredLogicExpr::LiteralNumber(123.0)),
        assign_var(
            "observed_view_x",
            LoweredLogicExpr::IndexAccess {
                target: Box::new(LoweredLogicExpr::Identifier("view_xview".into())),
                index: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
            },
        ),
    ]);

    assert_eq!(
        player_var(&core, "observed_view_x"),
        Some(&RuntimeValue::Number(123.0))
    );
}

#[test]
fn core_evaluates_unary_negative_in_assignments() {
    let cases = [(
        "score",
        LoweredLogicExpr::UnaryExpr {
            op: "-".into(),
            child: Box::new(LoweredLogicExpr::Identifier("y".into())),
        },
        RuntimeValue::Number(-24.0),
    )];

    let core = run_step(
        cases
            .iter()
            .map(|(target, value, _)| assign_var(target, value.clone()))
            .collect(),
    );

    for (target, _, expected) in cases {
        assert_eq!(player_var(&core, target), Some(&expected));
    }
}

#[test]
fn core_evaluates_unary_not_in_conditionals() {
    let cases = [(
        "flag",
        "armed",
        LoweredLogicExpr::UnaryExpr {
            op: "!".into(),
            child: Box::new(LoweredLogicExpr::Identifier("flag".into())),
        },
        RuntimeValue::Bool(true),
    )];

    let mut package = sample_package();
    add_create_block(
        &mut package,
        cases
            .iter()
            .map(|(source, _, _, _)| assign_var(source, LoweredLogicExpr::LiteralBool(false)))
            .collect(),
    );
    add_step_block(
        &mut package,
        cases
            .iter()
            .map(
                |(_, target, condition, _)| LoweredLogicStatement::Conditional {
                    condition: condition.clone(),
                    then_branch: vec![assign_var(target, LoweredLogicExpr::LiteralBool(true))],
                    else_branch: vec![],
                },
            )
            .collect(),
    );

    let core = tick_package(package);

    for (_, target, _, expected) in cases {
        assert_eq!(player_var(&core, target), Some(&expected));
    }
}

#[test]
fn core_treats_uninitialized_instance_variables_as_zero_in_expressions() {
    let core = run_step(vec![LoweredLogicStatement::Conditional {
        condition: LoweredLogicExpr::BinaryExpr {
            op: "&&".into(),
            left: Box::new(LoweredLogicExpr::BinaryExpr {
                op: "=".into(),
                left: Box::new(LoweredLogicExpr::Identifier("warpX".into())),
                right: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
            }),
            right: Box::new(LoweredLogicExpr::BinaryExpr {
                op: "=".into(),
                left: Box::new(LoweredLogicExpr::Identifier("warpY".into())),
                right: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
            }),
        },
        then_branch: vec![assign_var("entered", LoweredLogicExpr::LiteralBool(true))],
        else_branch: vec![],
    }]);

    assert_eq!(
        player_var(&core, "entered"),
        Some(&RuntimeValue::Bool(true))
    );
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
    player_mut(&mut core).vspeed = -8.0;

    core.tick(&mut host).unwrap();

    assert_eq!(player(&core).vspeed, -4.0);
}

#[test]
fn core_binds_script_arguments_and_returns_values_from_expressions() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![assign_var(
            "result",
            LoweredLogicExpr::Call {
                name: "doubleValue".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(6.0)],
            },
        )],
    );
    add_script_block(
        &mut package,
        14,
        "doubleValue",
        vec![
            LoweredLogicStatement::Return {
                value: Some(LoweredLogicExpr::BinaryExpr {
                    op: "*".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("argument0".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(2.0)),
                }),
            },
            assign_var("after_return", LoweredLogicExpr::LiteralBool(true)),
        ],
    );

    let core = tick_package(package);

    assert_eq!(
        player_var(&core, "result"),
        Some(&RuntimeValue::Number(12.0))
    );
    assert_eq!(player_var(&core, "after_return"), None);
}

#[test]
fn core_binds_arguments_for_statement_script_calls() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "setValue".into(),
            args: vec![LoweredLogicExpr::LiteralNumber(9.0)],
        }],
    );
    add_script_block(
        &mut package,
        15,
        "setValue",
        vec![assign_var(
            "script_value",
            LoweredLogicExpr::Identifier("argument0".into()),
        )],
    );

    let core = tick_package(package);

    assert_eq!(
        player_var(&core, "script_value"),
        Some(&RuntimeValue::Number(9.0))
    );
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

    assert_eq!(player(&core).vspeed, -4.0);
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

    assert_eq!(
        player_var(&core, "pressed"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(player_var(&core, "held"), Some(&RuntimeValue::Bool(true)));

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
    core.rebuild_lowered_logic_caches();

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
fn core_executes_keyboard_set_numlock_before_get_query() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "keyboard_set_numlock".into(),
                args: vec![LoweredLogicExpr::LiteralBool(true)],
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "keyboard_get_numlock".into(),
                    args: vec![],
                },
                then_branch: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("numlock_seen".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                }],
                else_branch: vec![],
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
    assert_eq!(
        player.vars.get("numlock_seen"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_treats_off_identifier_as_false_for_keyboard_set_numlock() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "keyboard_set_numlock".into(),
                args: vec![LoweredLogicExpr::LiteralBool(true)],
            },
            LoweredLogicStatement::FunctionCall {
                name: "keyboard_set_numlock".into(),
                args: vec![LoweredLogicExpr::Identifier("off".into())],
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::Call {
                    name: "keyboard_get_numlock".into(),
                    args: vec![],
                },
                then_branch: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("numlock_state".into()),
                    value: LoweredLogicExpr::LiteralText("on".into()),
                }],
                else_branch: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("numlock_state".into()),
                    value: LoweredLogicExpr::LiteralText("off".into()),
                }],
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
    assert_eq!(
        player.vars.get("numlock_state"),
        Some(&RuntimeValue::Text("off".into()))
    );
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
fn core_evaluates_two_argument_place_free_against_solid_instances() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("free_below".into()),
            value: LoweredLogicExpr::Call {
                name: "place_free".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("x".into()),
                    LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    },
                ],
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
    assert_eq!(
        player.vars.get("free_below"),
        Some(&RuntimeValue::Bool(false))
    );
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
        background_colour: 0,
        clear_screen: true,
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
        background_colour: 0,
        clear_screen: true,
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
                    args: vec![LoweredLogicExpr::LiteralText("custom_slot".into())],
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
        .write_temp(std::path::Path::new("custom_slot"), b"checkpoint")
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
fn core_executes_file_delete_against_host_files() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "file_delete".into(),
                args: vec![LoweredLogicExpr::LiteralText("custom_slot".into())],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("slot_exists".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_exists".into(),
                    args: vec![LoweredLogicExpr::LiteralText("custom_slot".into())],
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.files
        .write_temp(std::path::Path::new("custom_slot"), b"checkpoint")
        .unwrap();

    core.tick(&mut host).unwrap();

    assert!(
        host.files
            .read(std::path::Path::new("custom_slot"))
            .is_err(),
        "file_delete should remove the evaluated path from the host"
    );
    assert_eq!(
        player_var(&core, "slot_exists"),
        Some(&RuntimeValue::Bool(false))
    );
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_evaluates_gml_div_and_mod_binary_expressions() {
    let core = run_step(vec![
        assign_var(
            "hour",
            LoweredLogicExpr::BinaryExpr {
                op: "div".into(),
                left: Box::new(LoweredLogicExpr::LiteralNumber(7261.0)),
                right: Box::new(LoweredLogicExpr::LiteralNumber(3600.0)),
            },
        ),
        assign_var(
            "second",
            LoweredLogicExpr::BinaryExpr {
                op: "mod".into(),
                left: Box::new(LoweredLogicExpr::LiteralNumber(7261.0)),
                right: Box::new(LoweredLogicExpr::LiteralNumber(60.0)),
            },
        ),
    ]);

    assert_eq!(player_var(&core, "hour"), Some(&RuntimeValue::Number(2.0)));
    assert_eq!(
        player_var(&core, "second"),
        Some(&RuntimeValue::Number(1.0))
    );
}

#[test]
fn core_evaluates_room_identifier_against_named_room_constants() {
    let mut package = sample_package();
    package.rooms.push(iwm_runtime_model::RoomDefinition {
        id: 11,
        name: "rSelectStage".into(),
        width: 160,
        height: 120,
        speed: 60,
        persistent: false,
        background_colour: 0,
        clear_screen: true,
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
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::BinaryExpr {
                op: "!=".into(),
                left: Box::new(LoweredLogicExpr::Identifier("room".into())),
                right: Box::new(LoweredLogicExpr::Identifier("rSelectStage".into())),
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("not_select_stage".into()),
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
        player.vars.get("not_select_stage"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn core_executes_file_bin_write_and_read_byte_calls_against_host_files() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["f".into()],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("f".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_open".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralText("DeathTime".into()),
                        LoweredLogicExpr::LiteralNumber(1.0),
                    ],
                },
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_write_byte".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("f".into()),
                    LoweredLogicExpr::LiteralNumber(65.0),
                ],
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_close".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("f".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_open".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralText("DeathTime".into()),
                        LoweredLogicExpr::LiteralNumber(0.0),
                    ],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("first_byte".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_read_byte".into(),
                    args: vec![LoweredLogicExpr::Identifier("f".into())],
                },
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_close".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    assert_eq!(
        host.files.read(std::path::Path::new("DeathTime")).unwrap(),
        vec![65]
    );
    assert_eq!(
        player_var(&core, "first_byte"),
        Some(&RuntimeValue::Number(65.0))
    );
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_reads_file_bin_byte_after_absolute_seek() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["f".into()],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("f".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_open".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralText("save1".into()),
                        LoweredLogicExpr::LiteralNumber(0.0),
                    ],
                },
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_seek".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("f".into()),
                    LoweredLogicExpr::LiteralNumber(2.0),
                ],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("seek_byte".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_read_byte".into(),
                    args: vec![LoweredLogicExpr::Identifier("f".into())],
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.files
        .write_temp(std::path::Path::new("save1"), &[1, 2, 65])
        .unwrap();
    core.tick(&mut host).unwrap();

    assert_eq!(
        player_var(&core, "seek_byte"),
        Some(&RuntimeValue::Number(65.0))
    );
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_writes_file_bin_byte_after_absolute_seek_with_zero_fill() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["f".into()],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("f".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_open".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralText("save1".into()),
                        LoweredLogicExpr::LiteralNumber(1.0),
                    ],
                },
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_seek".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("f".into()),
                    LoweredLogicExpr::LiteralNumber(10.0),
                ],
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_write_byte".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("f".into()),
                    LoweredLogicExpr::LiteralNumber(65.0),
                ],
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_close".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    assert_eq!(
        host.files.read(std::path::Path::new("save1")).unwrap(),
        vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 65]
    );
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_evaluates_file_bin_read_byte_inside_binary_expressions() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["f".into(), "roomTo".into()],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("f".into()),
                value: LoweredLogicExpr::Call {
                    name: "file_bin_open".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralText("save1".into()),
                        LoweredLogicExpr::LiteralNumber(0.0),
                    ],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("roomTo".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "*".into(),
                    left: Box::new(LoweredLogicExpr::Call {
                        name: "file_bin_read_byte".into(),
                        args: vec![LoweredLogicExpr::Identifier("f".into())],
                    }),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(10000.0)),
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("roomTo".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "+".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("roomTo".into())),
                    right: Box::new(LoweredLogicExpr::BinaryExpr {
                        op: "*".into(),
                        left: Box::new(LoweredLogicExpr::Call {
                            name: "file_bin_read_byte".into(),
                            args: vec![LoweredLogicExpr::Identifier("f".into())],
                        }),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(100.0)),
                    }),
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("loaded_room".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "+".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("roomTo".into())),
                    right: Box::new(LoweredLogicExpr::Call {
                        name: "file_bin_read_byte".into(),
                        args: vec![LoweredLogicExpr::Identifier("f".into())],
                    }),
                },
            },
            LoweredLogicStatement::FunctionCall {
                name: "file_bin_close".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    host.files
        .write_temp(std::path::Path::new("save1"), &[0, 1, 43])
        .unwrap();
    core.tick(&mut host).unwrap();

    assert_eq!(
        player_var(&core, "loaded_room"),
        Some(&RuntimeValue::Number(143.0))
    );
}

#[test]
fn core_evaluates_instance_number_against_live_object_instances() {
    let mut package = sample_package();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "bullet".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: true,
        solid: false,
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
            x: 64,
            y: 64,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: false,
            is_checkpoint: false,
        });
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 17,
            object_id: 4,
            x: 80,
            y: 64,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: false,
            is_checkpoint: false,
        });
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("bullet_count".into()),
            value: LoweredLogicExpr::Call {
                name: "instance_number".into(),
                args: vec![LoweredLogicExpr::Identifier("bullet".into())],
            },
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    assert_eq!(
        player_var(&core, "bullet_count"),
        Some(&RuntimeValue::Number(2.0))
    );
}

#[test]
fn core_evaluates_abs_calls_in_lowered_expressions() {
    let cases = [
        ("positive", -12.5, RuntimeValue::Number(12.5)),
        ("already_positive", 3.0, RuntimeValue::Number(3.0)),
    ];

    let core = run_step(
        cases
            .iter()
            .map(|(target, input, _)| {
                assign_var(
                    target,
                    LoweredLogicExpr::Call {
                        name: "abs".into(),
                        args: vec![LoweredLogicExpr::LiteralNumber(*input)],
                    },
                )
            })
            .collect(),
    );

    for (target, _, expected) in cases {
        assert_eq!(player_var(&core, target), Some(&expected));
    }
}

#[test]
fn core_evaluates_random_and_choose_calls_in_lowered_expressions() {
    let core = run_step(vec![
        assign_var(
            "random_value",
            LoweredLogicExpr::Call {
                name: "random".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(5.0)],
            },
        ),
        assign_var(
            "choose_value",
            LoweredLogicExpr::Call {
                name: "choose".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(2.0),
                    LoweredLogicExpr::LiteralNumber(4.0),
                    LoweredLogicExpr::LiteralNumber(6.0),
                ],
            },
        ),
        assign_var(
            "range_value",
            LoweredLogicExpr::Call {
                name: "random_range".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(3.0),
                    LoweredLogicExpr::LiteralNumber(6.0),
                ],
            },
        ),
    ]);

    let Some(RuntimeValue::Number(random_value)) = player_var(&core, "random_value") else {
        panic!("random_value should be assigned");
    };
    assert!((0.0..5.0).contains(random_value));
    assert!(matches!(
        player_var(&core, "choose_value"),
        Some(RuntimeValue::Number(2.0 | 4.0 | 6.0))
    ));
    let Some(RuntimeValue::Number(range_value)) = player_var(&core, "range_value") else {
        panic!("range_value should be assigned");
    };
    assert!((3.0..6.0).contains(range_value));
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_evaluates_string_calls_in_lowered_expressions() {
    let cases = [
        (
            "integer_text",
            LoweredLogicExpr::Call {
                name: "string".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(12.0)],
            },
            RuntimeValue::Text("12".into()),
        ),
        (
            "fraction_text",
            LoweredLogicExpr::Call {
                name: "string".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(-3.25)],
            },
            RuntimeValue::Text("-3.25".into()),
        ),
        (
            "bool_text",
            LoweredLogicExpr::Call {
                name: "string".into(),
                args: vec![LoweredLogicExpr::LiteralBool(true)],
            },
            RuntimeValue::Text("true".into()),
        ),
        (
            "path_text",
            LoweredLogicExpr::BinaryExpr {
                op: "+".into(),
                left: Box::new(LoweredLogicExpr::LiteralText("save".into())),
                right: Box::new(LoweredLogicExpr::Call {
                    name: "string".into(),
                    args: vec![LoweredLogicExpr::LiteralNumber(1.0)],
                }),
            },
            RuntimeValue::Text("save1".into()),
        ),
    ];

    let core = run_step(
        cases
            .iter()
            .map(|(target, value, _)| assign_var(target, value.clone()))
            .collect(),
    );

    for (target, _, expected) in cases {
        assert_eq!(player_var(&core, target), Some(&expected));
    }
}

#[test]
fn core_evaluates_distance_to_object_against_nearest_target_bbox() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("marker_distance".into()),
            value: LoweredLogicExpr::Call {
                name: "distance_to_object".into(),
                args: vec![LoweredLogicExpr::Identifier("obj_marker".into())],
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
        player.x = 12.0;
        player.y = 24.0;

        let marker = room
            .instances
            .iter_mut()
            .find(|instance| instance.object_name == "obj_marker")
            .unwrap();
        marker.x = 48.0;
        marker.y = 64.0;
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
        player.vars.get("marker_distance"),
        Some(&RuntimeValue::Number(
            (21.0_f64.powi(2) + 25.0_f64.powi(2)).sqrt()
        ))
    );
}

#[test]
fn core_evaluates_distance_to_object_as_gm_default_when_target_missing() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("missing_distance".into()),
            value: LoweredLogicExpr::Call {
                name: "distance_to_object".into(),
                args: vec![LoweredLogicExpr::Identifier("obj_marker".into())],
            },
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    {
        let room = core.current_room.as_mut().unwrap();
        let marker = room
            .instances
            .iter_mut()
            .find(|instance| instance.object_name == "obj_marker")
            .unwrap();
        marker.alive = false;
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
        player.vars.get("missing_distance"),
        Some(&RuntimeValue::Number(1_000_000.0))
    );
}

#[test]
fn core_evaluates_collision_line_against_object_name_targets() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("line_hit".into()),
            value: LoweredLogicExpr::Call {
                name: "collision_line".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(40.0),
                    LoweredLogicExpr::LiteralNumber(72.0),
                    LoweredLogicExpr::LiteralNumber(80.0),
                    LoweredLogicExpr::LiteralNumber(72.0),
                    LoweredLogicExpr::Identifier("obj_marker".into()),
                    LoweredLogicExpr::LiteralBool(false),
                    LoweredLogicExpr::LiteralBool(true),
                ],
            },
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    {
        let room = core.current_room.as_mut().unwrap();
        let marker = room
            .instances
            .iter_mut()
            .find(|instance| instance.object_name == "obj_marker")
            .unwrap();
        marker.x = 48.0;
        marker.y = 64.0;
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
        player.vars.get("line_hit"),
        Some(&RuntimeValue::Number(12.0))
    );
}

#[test]
fn core_evaluates_collision_rectangle_and_geometry_helpers() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("rect_hit".into()),
                value: LoweredLogicExpr::Call {
                    name: "collision_rectangle".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralNumber(40.0),
                        LoweredLogicExpr::LiteralNumber(60.0),
                        LoweredLogicExpr::LiteralNumber(64.0),
                        LoweredLogicExpr::LiteralNumber(80.0),
                        LoweredLogicExpr::Identifier("obj_marker".into()),
                        LoweredLogicExpr::LiteralBool(false),
                        LoweredLogicExpr::LiteralBool(true),
                    ],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("distance".into()),
                value: LoweredLogicExpr::Call {
                    name: "point_distance".into(),
                    args: vec![0.0, 0.0, 3.0, 4.0]
                        .into_iter()
                        .map(LoweredLogicExpr::LiteralNumber)
                        .collect(),
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("ldx".into()),
                value: LoweredLogicExpr::Call {
                    name: "lengthdir_x".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralNumber(10.0),
                        LoweredLogicExpr::LiteralNumber(0.0),
                    ],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("ldy".into()),
                value: LoweredLogicExpr::Call {
                    name: "lengthdir_y".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralNumber(10.0),
                        LoweredLogicExpr::LiteralNumber(90.0),
                    ],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("found".into()),
                value: LoweredLogicExpr::Call {
                    name: "instance_find".into(),
                    args: vec![
                        LoweredLogicExpr::Identifier("obj_marker".into()),
                        LoweredLogicExpr::LiteralNumber(0.0),
                    ],
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    {
        let marker = core
            .current_room
            .as_mut()
            .unwrap()
            .instances
            .iter_mut()
            .find(|instance| instance.object_name == "obj_marker")
            .unwrap();
        marker.x = 48.0;
        marker.y = 64.0;
    }

    core.tick(&mut host).unwrap();

    let player = player(&core);
    assert_eq!(
        player.vars.get("rect_hit"),
        Some(&RuntimeValue::Number(12.0))
    );
    assert_eq!(
        player.vars.get("distance"),
        Some(&RuntimeValue::Number(5.0))
    );
    assert_eq!(player.vars.get("ldx"), Some(&RuntimeValue::Number(10.0)));
    assert!(
        matches!(player.vars.get("ldy"), Some(RuntimeValue::Number(value)) if (*value + 10.0).abs() < 1e-10)
    );
    assert_eq!(player.vars.get("found"), Some(&RuntimeValue::Number(12.0)));
}

#[test]
fn core_evaluates_collision_line_as_noone_when_no_target_intersects() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("line_hit".into()),
            value: LoweredLogicExpr::Call {
                name: "collision_line".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(8.0),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::Identifier("obj_marker".into()),
                    LoweredLogicExpr::LiteralBool(false),
                    LoweredLogicExpr::LiteralBool(true),
                ],
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
    assert_eq!(
        player.vars.get("line_hit"),
        Some(&RuntimeValue::Number(-4.0))
    );
}

#[test]
fn core_treats_collision_line_noone_result_as_false_in_conditionals() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "collision_line".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(8.0),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::Identifier("obj_marker".into()),
                    LoweredLogicExpr::LiteralBool(false),
                    LoweredLogicExpr::LiteralBool(true),
                ],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("line_branch".into()),
                value: LoweredLogicExpr::LiteralText("hit".into()),
            }],
            else_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("line_branch".into()),
                value: LoweredLogicExpr::LiteralText("miss".into()),
            }],
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
        player.vars.get("line_branch"),
        Some(&RuntimeValue::Text("miss".into()))
    );
}

#[test]
fn core_executes_lowered_for_loop_assignments() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("sum".into()),
                value: LoweredLogicExpr::LiteralNumber(0.0),
            },
            LoweredLogicStatement::For {
                init: LoweredLogicExpr::BinaryExpr {
                    op: "=".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
                },
                condition: LoweredLogicExpr::BinaryExpr {
                    op: "<".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(3.0)),
                },
                step: LoweredLogicExpr::BinaryExpr {
                    op: "=".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    }),
                },
                body: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("sum".into()),
                    value: LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("sum".into())),
                        right: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    },
                }],
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
    assert_eq!(player.vars.get("sum"), Some(&RuntimeValue::Number(3.0)));
    assert_eq!(player.vars.get("i"), Some(&RuntimeValue::Number(3.0)));
}

#[test]
fn core_executes_lowered_for_loop_with_local_iterator_scope() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["i".into()],
            },
            LoweredLogicStatement::For {
                init: LoweredLogicExpr::BinaryExpr {
                    op: "=".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
                },
                condition: LoweredLogicExpr::BinaryExpr {
                    op: "<".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(2.0)),
                },
                step: LoweredLogicExpr::BinaryExpr {
                    op: "=".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    }),
                },
                body: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("last_i".into()),
                    value: LoweredLogicExpr::Identifier("i".into()),
                }],
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
    assert_eq!(player.vars.get("last_i"), Some(&RuntimeValue::Number(1.0)));
    assert_eq!(player.vars.get("i"), None);
}

#[test]
fn core_stops_lowered_for_loop_when_body_requests_room_transition() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::For {
            init: LoweredLogicExpr::BinaryExpr {
                op: "=".into(),
                left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                right: Box::new(LoweredLogicExpr::LiteralNumber(0.0)),
            },
            condition: LoweredLogicExpr::BinaryExpr {
                op: "<".into(),
                left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                right: Box::new(LoweredLogicExpr::LiteralNumber(3.0)),
            },
            step: LoweredLogicExpr::BinaryExpr {
                op: "=".into(),
                left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                right: Box::new(LoweredLogicExpr::BinaryExpr {
                    op: "+".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                }),
            },
            body: vec![LoweredLogicStatement::FunctionCall {
                name: "room_goto".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(9.0)],
            }],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    assert_eq!(core.snapshot().room_id, Some(9));
}

#[test]
fn core_evaluates_collision_line_notme_by_skipping_current_instance() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("line_hit".into()),
            value: LoweredLogicExpr::Call {
                name: "collision_line".into(),
                args: vec![
                    LoweredLogicExpr::LiteralNumber(12.0),
                    LoweredLogicExpr::LiteralNumber(24.0),
                    LoweredLogicExpr::LiteralNumber(24.0),
                    LoweredLogicExpr::LiteralNumber(24.0),
                    LoweredLogicExpr::Identifier("obj_player".into()),
                    LoweredLogicExpr::LiteralBool(false),
                    LoweredLogicExpr::LiteralBool(true),
                ],
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
        player.x = 12.0;
        player.y = 24.0;
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
        player.vars.get("line_hit"),
        Some(&RuntimeValue::Number(-4.0))
    );
}

#[test]
fn core_evaluates_instance_place_result_member_accesses() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("a".into()),
                value: LoweredLogicExpr::Call {
                    name: "instance_place".into(),
                    args: vec![
                        LoweredLogicExpr::Identifier("x".into()),
                        LoweredLogicExpr::BinaryExpr {
                            op: "+".into(),
                            left: Box::new(LoweredLogicExpr::Identifier("y".into())),
                            right: Box::new(LoweredLogicExpr::LiteralNumber(16.0)),
                        },
                        LoweredLogicExpr::Identifier("obj_block".into()),
                    ],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("hit_block".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "=".into(),
                    left: Box::new(LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("a".into())),
                        member: "object_index".into(),
                    }),
                    right: Box::new(LoweredLogicExpr::Identifier("obj_block".into())),
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        player.vars.get("hit_block"),
        Some(&RuntimeValue::Bool(true))
    );
}

fn call(name: &str, args: Vec<LoweredLogicExpr>) -> LoweredLogicExpr {
    LoweredLogicExpr::Call {
        name: name.into(),
        args,
    }
}

fn number(value: f64) -> LoweredLogicExpr {
    LoweredLogicExpr::LiteralNumber(value)
}

#[test]
fn core_evaluates_gm_math_builtins() {
    let core = run_step(vec![
        // Crimson's save codec packs bytes with `min(tem div power(256, i-1), 255)`.
        assign_var("packed", call("power", vec![number(256.0), number(2.0)])),
        assign_var(
            "byte",
            call(
                "min",
                vec![
                    LoweredLogicExpr::BinaryExpr {
                        op: "div".into(),
                        left: Box::new(number(600.0)),
                        right: Box::new(call("power", vec![number(256.0), number(1.0)])),
                    },
                    number(255.0),
                ],
            ),
        ),
        assign_var(
            "hi",
            call("max", vec![number(7.0), number(3.0), number(9.0)]),
        ),
        assign_var("neg", call("sign", vec![number(-4.0)])),
        assign_var("zero", call("sign", vec![number(0.0)])),
        assign_var("cos0", call("cos", vec![number(0.0)])),
        assign_var("half_pi", call("degtorad", vec![number(90.0)])),
        assign_var("diag", call("arctan2", vec![number(1.0), number(1.0)])),
    ]);

    assert_eq!(
        player_var(&core, "packed"),
        Some(&RuntimeValue::Number(65536.0))
    );
    assert_eq!(player_var(&core, "byte"), Some(&RuntimeValue::Number(2.0)));
    assert_eq!(player_var(&core, "hi"), Some(&RuntimeValue::Number(9.0)));
    assert_eq!(player_var(&core, "neg"), Some(&RuntimeValue::Number(-1.0)));
    assert_eq!(player_var(&core, "zero"), Some(&RuntimeValue::Number(0.0)));
    assert_eq!(player_var(&core, "cos0"), Some(&RuntimeValue::Number(1.0)));
    match player_var(&core, "half_pi") {
        Some(RuntimeValue::Number(value)) => {
            assert!((value - std::f64::consts::FRAC_PI_2).abs() < 1e-9)
        }
        other => panic!("expected radians, got {other:?}"),
    }
    match player_var(&core, "diag") {
        Some(RuntimeValue::Number(value)) => {
            assert!((value - std::f64::consts::FRAC_PI_4).abs() < 1e-9)
        }
        other => panic!("expected arctan2 result, got {other:?}"),
    }
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_instance_position_resolves_instance_and_member_at_point() {
    // Mirrors Crimson's boundary transition:
    // `roomTo = instance_position(x, y, roomChanger).roomTo`.
    // `obj_sparse_sprite` (id 705) has one placed instance (id 15) at (96, 96)
    // with a 16x16 mask-less bounding box, so a point inside it resolves to that
    // instance and a point outside resolves to GM `noone` (-4).
    let core = run_step(vec![
        assign_var(
            "hit_id",
            call(
                "instance_position",
                vec![
                    number(96.0),
                    number(96.0),
                    LoweredLogicExpr::Identifier("obj_sparse_sprite".into()),
                ],
            ),
        ),
        assign_var(
            "hit_obj",
            LoweredLogicExpr::MemberAccess {
                target: Box::new(call(
                    "instance_position",
                    vec![
                        number(100.0),
                        number(100.0),
                        LoweredLogicExpr::Identifier("obj_sparse_sprite".into()),
                    ],
                )),
                member: "object_index".into(),
            },
        ),
        assign_var(
            "miss",
            call(
                "instance_position",
                vec![
                    number(300.0),
                    number(300.0),
                    LoweredLogicExpr::Identifier("obj_sparse_sprite".into()),
                ],
            ),
        ),
    ]);

    assert_eq!(
        player_var(&core, "hit_id"),
        Some(&RuntimeValue::Number(15.0))
    );
    assert_eq!(
        player_var(&core, "hit_obj"),
        Some(&RuntimeValue::Number(705.0))
    );
    assert_eq!(player_var(&core, "miss"), Some(&RuntimeValue::Number(-4.0)));
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_executes_gm8_movement_helpers() {
    let mut package = sample_package();
    package.rooms[0].instances[2].x = 64;
    package.rooms[0].instances[2].y = 64;
    package.objects[1]
        .events
        .push(iwm_runtime_model::ObjectEventEntry {
            event_type: 3,
            sub_event: 0,
            event_tag: "step".into(),
            block_id: "object:1:event:3:0".into(),
            action_count: 1,
        });
    append_lowered_entry(
        &mut package,
        "object:1:event:3:0".into(),
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "move_towards_point".into(),
                args: vec![number(58.0), number(64.0), number(4.0)],
            },
            assign_var(
                "towards_hspeed",
                LoweredLogicExpr::Identifier("hspeed".into()),
            ),
            assign_var(
                "towards_vspeed",
                LoweredLogicExpr::Identifier("vspeed".into()),
            ),
            LoweredLogicStatement::FunctionCall {
                name: "motion_add".into(),
                args: vec![number(90.0), number(2.0)],
            },
            assign_var(
                "added_hspeed",
                LoweredLogicExpr::Identifier("hspeed".into()),
            ),
            assign_var(
                "added_vspeed",
                LoweredLogicExpr::Identifier("vspeed".into()),
            ),
            assign_var("x", number(-9.0)),
            LoweredLogicStatement::FunctionCall {
                name: "move_wrap".into(),
                args: vec![
                    LoweredLogicExpr::LiteralBool(true),
                    LoweredLogicExpr::LiteralBool(false),
                    number(8.0),
                ],
            },
            assign_var("wrapped_x", LoweredLogicExpr::Identifier("x".into())),
            assign_var("x", number(48.0)),
            assign_var("y", number(64.0)),
            assign_var("hspeed", number(2.0)),
            assign_var("vspeed", number(0.0)),
            LoweredLogicStatement::FunctionCall {
                name: "move_bounce_solid".into(),
                args: vec![number(0.0)],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    core.tick(&mut host()).unwrap();
    let marker = &core.current_room().unwrap().instances[1];
    assert_eq!(
        marker.vars.get("towards_hspeed"),
        Some(&RuntimeValue::Number(4.0))
    );
    assert_eq!(
        marker.vars.get("towards_vspeed"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert_eq!(
        marker.vars.get("added_hspeed"),
        Some(&RuntimeValue::Number(4.0))
    );
    assert_eq!(
        marker.vars.get("added_vspeed"),
        Some(&RuntimeValue::Number(-2.0))
    );
    assert_eq!(
        marker.vars.get("wrapped_x"),
        Some(&RuntimeValue::Number(328.0))
    );
    assert_eq!((marker.x, marker.y), (46.0, 64.0));
    assert_eq!((marker.hspeed, marker.vspeed), (-2.0, 0.0));
}

/// Opens a text file for writing, writes `chunks`, and closes it.
fn write_text_file(path: &str, chunks: &[&str]) -> Vec<LoweredLogicStatement> {
    let mut statements = vec![LoweredLogicStatement::Assignment {
        target: LoweredLogicExpr::Identifier("f".into()),
        value: LoweredLogicExpr::Call {
            name: "file_text_open_write".into(),
            args: vec![LoweredLogicExpr::LiteralText(path.into())],
        },
    }];
    for chunk in chunks {
        statements.push(LoweredLogicStatement::FunctionCall {
            name: "file_text_write_string".into(),
            args: vec![
                LoweredLogicExpr::Identifier("f".into()),
                LoweredLogicExpr::LiteralText((*chunk).into()),
            ],
        });
    }
    statements.push(LoweredLogicStatement::FunctionCall {
        name: "file_text_close".into(),
        args: vec![LoweredLogicExpr::Identifier("f".into())],
    });
    statements
}

#[test]
fn core_flushes_file_text_writes_to_the_host_on_close() {
    let mut statements = write_text_file("IWRTK_globaldata", &["musicOn = 1"]);
    statements.push(LoweredLogicStatement::FunctionCall {
        name: "file_text_writeln".into(),
        args: vec![LoweredLogicExpr::Identifier("f".into())],
    });

    let mut package = sample_package();
    add_step_block(&mut package, statements);
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.tick(&mut host).unwrap();

    assert_eq!(
        host.files
            .read(std::path::Path::new("IWRTK_globaldata"))
            .unwrap(),
        b"musicOn = 1".to_vec(),
        "writes after close must not reach the host"
    );
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_reads_back_text_files_line_by_line() {
    let mut statements = write_text_file("save0", &["12.5 rest\r\nsecond line"]);
    statements.extend([
        LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("f".into()),
            value: LoweredLogicExpr::Call {
                name: "file_text_open_read".into(),
                args: vec![LoweredLogicExpr::LiteralText("save0".into())],
            },
        },
        assign_var(
            "first_real",
            LoweredLogicExpr::Call {
                name: "file_text_read_real".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ),
        assign_var(
            "rest_of_line",
            LoweredLogicExpr::Call {
                name: "file_text_read_string".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ),
        assign_var(
            "at_line_end",
            LoweredLogicExpr::Call {
                name: "file_text_eoln".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ),
        LoweredLogicStatement::FunctionCall {
            name: "file_text_readln".into(),
            args: vec![LoweredLogicExpr::Identifier("f".into())],
        },
        assign_var(
            "second_line",
            LoweredLogicExpr::Call {
                name: "file_text_read_string".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ),
        assign_var(
            "at_eof",
            LoweredLogicExpr::Call {
                name: "file_text_eof".into(),
                args: vec![LoweredLogicExpr::Identifier("f".into())],
            },
        ),
        LoweredLogicStatement::FunctionCall {
            name: "file_text_close".into(),
            args: vec![LoweredLogicExpr::Identifier("f".into())],
        },
    ]);

    let core = run_step(statements);

    assert_eq!(player_var(&core, "first_real"), Some(&RuntimeValue::Number(12.5)));
    assert_eq!(
        player_var(&core, "rest_of_line"),
        Some(&RuntimeValue::Text(" rest".into()))
    );
    assert_eq!(player_var(&core, "at_line_end"), Some(&RuntimeValue::Bool(true)));
    assert_eq!(
        player_var(&core, "second_line"),
        Some(&RuntimeValue::Text("second line".into()))
    );
    assert_eq!(player_var(&core, "at_eof"), Some(&RuntimeValue::Bool(true)));
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_resolves_room_get_name_for_the_active_room() {
    let core = run_step(vec![assign_var(
        "active_room_name",
        LoweredLogicExpr::Call {
            name: "room_get_name".into(),
            args: vec![LoweredLogicExpr::Identifier("room".into())],
        },
    )]);

    assert_eq!(
        player_var(&core, "active_room_name"),
        Some(&RuntimeValue::Text("room7".into()))
    );
    assert_no_runtime_blockers(&core);
}

/// The Kamilia save chain end to end: `savegame()` writes a GML source file and
/// `loadgame()` replays it through `execute_file()`.
#[test]
fn core_executes_saved_gml_source_through_execute_file() {
    let mut statements = write_text_file(
        "IWRTK_save0",
        &["saved_x = 199\r\nsaved_y = 486\r\ndifficulty = 2\r\n"],
    );
    statements.push(LoweredLogicStatement::FunctionCall {
        name: "execute_file".into(),
        args: vec![LoweredLogicExpr::LiteralText("IWRTK_save0".into())],
    });

    let core = run_step(statements);

    assert_eq!(player_var(&core, "saved_x"), Some(&RuntimeValue::Number(199.0)));
    assert_eq!(player_var(&core, "saved_y"), Some(&RuntimeValue::Number(486.0)));
    assert_eq!(
        player_var(&core, "difficulty"),
        Some(&RuntimeValue::Number(2.0))
    );
    assert_no_runtime_blockers(&core);
}

/// `savegame()` writes `room_goto(<room_get_name(room)>)`, so the room name has
/// to survive the write/lower/execute round trip.
#[test]
fn core_execute_file_restores_the_saved_room() {
    let write_string = |value: LoweredLogicExpr| LoweredLogicStatement::FunctionCall {
        name: "file_text_write_string".into(),
        args: vec![LoweredLogicExpr::Identifier("f".into()), value],
    };
    let core = run_step(vec![
        LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("f".into()),
            value: LoweredLogicExpr::Call {
                name: "file_text_open_write".into(),
                args: vec![LoweredLogicExpr::LiteralText("IWRTK_save0".into())],
            },
        },
        write_string(LoweredLogicExpr::LiteralText("room_goto(".into())),
        write_string(LoweredLogicExpr::Call {
            name: "room_get_name".into(),
            args: vec![LoweredLogicExpr::LiteralNumber(9.0)],
        }),
        write_string(LoweredLogicExpr::LiteralText(");".into())),
        LoweredLogicStatement::FunctionCall {
            name: "file_text_close".into(),
            args: vec![LoweredLogicExpr::Identifier("f".into())],
        },
        LoweredLogicStatement::FunctionCall {
            name: "execute_file".into(),
            args: vec![LoweredLogicExpr::LiteralText("IWRTK_save0".into())],
        },
    ]);

    assert_eq!(core.current_room().unwrap().room_name, "room9");
    assert_no_runtime_blockers(&core);
}

#[test]
fn core_bounds_execute_file_recursion_through_self_executing_sources() {
    let mut statements = write_text_file(
        "IWRTK_loop",
        &["looped = looped + 1\r\nexecute_file(\"IWRTK_loop\");\r\n"],
    );
    statements.insert(0, assign_var("looped", LoweredLogicExpr::LiteralNumber(0.0)));
    statements.push(LoweredLogicStatement::FunctionCall {
        name: "execute_file".into(),
        args: vec![LoweredLogicExpr::LiteralText("IWRTK_loop".into())],
    });

    let core = run_step(statements);

    assert!(
        core.diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "runtime-execute-source-depth-limit"),
        "expected the recursion guard to fire: {:?}",
        core.diagnostics()
    );
    assert_no_runtime_blockers(&core);
}
