use super::*;

use iwm_runtime_model::RoomInstancePlacement;

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
fn lowered_step_locals_feed_later_expressions_without_polluting_instance_vars() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["tmp_speed".into()],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("tmp_speed".into()),
                value: LoweredLogicExpr::LiteralNumber(4.0),
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("moveSpeed".into()),
                value: LoweredLogicExpr::BinaryExpr {
                    op: "+".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("tmp_speed".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(2.0)),
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

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
    assert_eq!(player.vars.get("tmp_speed"), None);
}

#[test]
fn lowered_script_locals_do_not_leak_into_calling_event_scope() {
    let mut package = sample_package();
    add_script_block(
        &mut package,
        10,
        "setTemp",
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["tmp_speed".into()],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("tmp_speed".into()),
                value: LoweredLogicExpr::LiteralNumber(4.0),
            },
        ],
    );
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "setTemp".into(),
                args: vec![],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("moveSpeed".into()),
                value: LoweredLogicExpr::Identifier("tmp_speed".into()),
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.vars.get("moveSpeed"), None);
    assert_eq!(player.vars.get("tmp_speed"), None);
}

#[test]
fn lowered_with_executes_body_against_matching_instances_with_other_as_caller() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::With {
            target: LoweredLogicExpr::Identifier("obj_block".into()),
            body: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("touched_by_with".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("caller_x_seen".into()),
                    value: LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("other".into())),
                        member: "x".into(),
                    },
                },
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    let block = room
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_block")
        .unwrap();
    assert_eq!(player.vars.get("touched_by_with"), None);
    assert_eq!(
        block.vars.get("touched_by_with"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        block.vars.get("caller_x_seen"),
        Some(&RuntimeValue::Number(12.0))
    );
}

#[test]
fn lowered_with_writes_are_visible_to_later_statements_in_same_event() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::With {
                target: LoweredLogicExpr::Identifier("obj_block".into()),
                body: vec![LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("with_value".into()),
                    value: LoweredLogicExpr::LiteralNumber(42.0),
                }],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("observed_with_value".into()),
                value: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("obj_block".into())),
                    member: "with_value".into(),
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
        player.vars.get("observed_with_value"),
        Some(&RuntimeValue::Number(42.0))
    );
}

#[test]
fn nested_with_other_restores_immediate_caller_context() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::With {
            target: LoweredLogicExpr::Identifier("obj_block".into()),
            body: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("outer_other_x".into()),
                    value: LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("other".into())),
                        member: "x".into(),
                    },
                },
                LoweredLogicStatement::With {
                    target: LoweredLogicExpr::Identifier("other".into()),
                    body: vec![LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("nested_other_x".into()),
                        value: LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("other".into())),
                            member: "x".into(),
                        },
                    }],
                },
            ],
        }],
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
    let block = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_block")
        .unwrap();

    assert_eq!(
        block.vars.get("outer_other_x"),
        Some(&RuntimeValue::Number(12.0))
    );
    assert_eq!(
        player.vars.get("nested_other_x"),
        Some(&RuntimeValue::Number(block.x))
    );
}

#[test]
fn with_other_member_reads_see_updates_made_through_other_handle() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::With {
            target: LoweredLogicExpr::Identifier("obj_block".into()),
            body: vec![
                LoweredLogicStatement::With {
                    target: LoweredLogicExpr::Identifier("other".into()),
                    body: vec![LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("x".into()),
                        value: LoweredLogicExpr::LiteralNumber(99.0),
                    }],
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("updated_other_x".into()),
                    value: LoweredLogicExpr::MemberAccess {
                        target: Box::new(LoweredLogicExpr::Identifier("other".into())),
                        member: "x".into(),
                    },
                },
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let block = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_block")
        .unwrap();
    assert_eq!(
        block.vars.get("updated_other_x"),
        Some(&RuntimeValue::Number(99.0))
    );
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
fn lowered_step_dispatch_keeps_many_prior_updates_fast_and_visible() {
    const MARKER_COUNT: usize = 600;
    const MAX_ELAPSED_MS: u128 = 40;

    let mut package = sample_package();
    package.rooms[0].instances.truncate(1);
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 3,
        sub_event: 0,
        event_tag: "step".into(),
        block_id: "object:1:event:3:0".into(),
        action_count: 0,
    });

    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("x".into()),
            value: LoweredLogicExpr::LiteralNumber(99.0),
        }],
    );
    append_lowered_entry(
        &mut package,
        "object:1:event:3:0".into(),
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("seen_player_x".into()),
                value: LoweredLogicExpr::MemberAccess {
                    target: Box::new(LoweredLogicExpr::Identifier("obj_player".into())),
                    member: "x".into(),
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("player_count".into()),
                value: LoweredLogicExpr::Call {
                    name: "instance_number".into(),
                    args: vec![LoweredLogicExpr::Identifier("obj_player".into())],
                },
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("player_distance".into()),
                value: LoweredLogicExpr::Call {
                    name: "distance_to_object".into(),
                    args: vec![LoweredLogicExpr::Identifier("obj_player".into())],
                },
            },
        ],
    );

    for index in 0..MARKER_COUNT {
        package.rooms[0].instances.push(RoomInstancePlacement {
            instance_id: 1000 + index as i32,
            object_id: 1,
            x: 32 + index as i32,
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
    }

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let start = std::time::Instant::now();

    core.execute_lowered_step_events(&mut host).unwrap();

    let elapsed = start.elapsed();
    let last_marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .rev()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    assert_eq!(
        last_marker.vars.get("seen_player_x"),
        Some(&RuntimeValue::Number(99.0))
    );
    assert_eq!(
        last_marker.vars.get("player_count"),
        Some(&RuntimeValue::Number(1.0))
    );
    assert!(
        elapsed.as_millis() < MAX_ELAPSED_MS,
        "large step dispatch took {}ms for {MARKER_COUNT} marker instances",
        elapsed.as_millis()
    );
}
