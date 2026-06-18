use super::*;

const SPAWNED_OBJECT_NAME: &str = "obj_spawned";
const SPAWNED_OBJECT_ID: usize = 4;
const SPAWNED_CREATE_BLOCK_ID: &str = "object:4:event:0:0";

fn add_spawned_object(
    package: &mut crate::RuntimePackage,
    create_statements: Vec<LoweredLogicStatement>,
) {
    package.objects.push(ObjectDefinition {
        id: SPAWNED_OBJECT_ID,
        name: SPAWNED_OBJECT_NAME.into(),
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
        events: vec![ObjectEventEntry {
            event_type: 0,
            sub_event: 0,
            event_tag: "create".into(),
            block_id: SPAWNED_CREATE_BLOCK_ID.into(),
            action_count: 0,
        }],
    });
    package.manifest.object_count = package.objects.len();
    append_lowered_entry(package, SPAWNED_CREATE_BLOCK_ID.into(), create_statements);
}

fn create_spawned_at(x: f64, y: f64) -> LoweredLogicStatement {
    LoweredLogicStatement::FunctionCall {
        name: "instance_create".into(),
        args: vec![
            LoweredLogicExpr::LiteralNumber(x),
            LoweredLogicExpr::LiteralNumber(y),
            LoweredLogicExpr::Identifier(SPAWNED_OBJECT_NAME.into()),
        ],
    }
}

fn spawned_instances(core: &RuntimeCore) -> Vec<&crate::RuntimeInstance> {
    instances_named(core, SPAWNED_OBJECT_NAME)
}

fn instances_named<'a>(
    core: &'a RuntimeCore,
    object_name: &str,
) -> Vec<&'a crate::RuntimeInstance> {
    core.current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| instance.object_name == object_name)
        .collect()
}

#[test]
fn instance_destroy_in_step_marks_current_instance_not_alive() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_destroy".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    assert!(!player(&core).alive);
}

#[test]
fn instance_destroy_dispatches_destroy_event_before_marking_dead() {
    let mut package = sample_package();
    add_destroy_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "destroy_ran".into(),
            },
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_destroy".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    assert_eq!(
        core.globals.get("global.destroy_ran"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-instance-destroyed"));
}

#[test]
fn instance_destroy_inside_with_marks_target_instance_not_caller() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::With {
            target: LoweredLogicExpr::Identifier("obj_block".into()),
            body: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_destroy".into(),
                args: vec![],
            }],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let caller = room
        .instances
        .iter()
        .find(|instance| instance.runtime_id == 0)
        .unwrap();
    let target = room
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_block")
        .unwrap();
    assert!(caller.alive);
    assert!(!target.alive);
}

#[test]
fn with_destroyed_target_is_not_visible_to_later_instance_exists_in_same_event() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::With {
                target: LoweredLogicExpr::Identifier("obj_block".into()),
                body: vec![LoweredLogicStatement::FunctionCall {
                    name: "instance_destroy".into(),
                    args: vec![],
                }],
            },
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("block_still_exists".into()),
                value: LoweredLogicExpr::Call {
                    name: "instance_exists".into(),
                    args: vec![LoweredLogicExpr::Identifier("obj_block".into())],
                },
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    assert_eq!(
        player_var(&core, "block_still_exists"),
        Some(&RuntimeValue::Bool(false))
    );
}

#[test]
fn instance_create_in_step_creates_duplicate_instances_and_runs_create_event() {
    let mut package = sample_package();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 0,
        sub_event: 0,
        event_tag: "create".into(),
        block_id: "object:1:event:0:0".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:0:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("created_by_event".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_create".into(),
            args: vec![
                LoweredLogicExpr::LiteralNumber(80.0),
                LoweredLogicExpr::LiteralNumber(96.0),
                LoweredLogicExpr::Identifier("obj_marker".into()),
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let markers = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| instance.object_name == "obj_marker")
        .collect::<Vec<_>>();
    assert_eq!(markers.len(), 2);
    let created = markers
        .iter()
        .find(|instance| (instance.x, instance.y) == (80.0, 96.0))
        .unwrap();
    assert_eq!(
        created.vars.get("created_by_event"),
        Some(&RuntimeValue::Bool(true))
    );
    assert!(core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-instance-created"));
}

#[test]
fn instance_created_inside_with_all_does_not_run_same_with_iteration() {
    let mut package = sample_package();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 0,
        sub_event: 0,
        event_tag: "create".into(),
        block_id: "object:1:event:0:0".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:0:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("created_by_event".into()),
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::With {
            target: LoweredLogicExpr::Identifier("all".into()),
            body: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("visited_by_with_all".into()),
                    value: LoweredLogicExpr::LiteralBool(true),
                },
                LoweredLogicStatement::FunctionCall {
                    name: "instance_create".into(),
                    args: vec![
                        LoweredLogicExpr::LiteralNumber(80.0),
                        LoweredLogicExpr::LiteralNumber(96.0),
                        LoweredLogicExpr::Identifier("obj_marker".into()),
                    ],
                },
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let created_marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| instance.object_name == "obj_marker")
        .max_by_key(|instance| instance.runtime_id)
        .unwrap();
    assert_eq!(
        created_marker.vars.get("created_by_event"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(created_marker.vars.get("visited_by_with_all"), None);
}

#[test]
fn runtime_instance_create_event_can_see_created_instance() {
    let mut package = sample_package();
    add_spawned_object(
        &mut package,
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "instance_exists".into(),
                args: vec![LoweredLogicExpr::Identifier(SPAWNED_OBJECT_NAME.into())],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("saw_self_object".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );
    add_step_block(&mut package, vec![create_spawned_at(80.0, 96.0)]);

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let created = spawned_instances(&core)
        .into_iter()
        .find(|instance| (instance.x, instance.y) == (80.0, 96.0))
        .unwrap();
    assert_eq!(
        created.vars.get("saw_self_object"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn instance_create_events_remain_visible_across_multiple_queued_creates() {
    let mut package = sample_package();
    add_spawned_object(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("saw_spawned_count".into()),
            value: LoweredLogicExpr::Call {
                name: "instance_number".into(),
                args: vec![LoweredLogicExpr::Identifier(SPAWNED_OBJECT_NAME.into())],
            },
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Repeat {
            count: LoweredLogicExpr::LiteralNumber(3.0),
            body: vec![create_spawned_at(80.0, 96.0)],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let created = spawned_instances(&core);
    assert_eq!(created.len(), 3);
    let seen_counts = created
        .iter()
        .map(|instance| instance.vars.get("saw_spawned_count"))
        .collect::<Vec<_>>();
    assert_eq!(
        seen_counts,
        vec![
            Some(&RuntimeValue::Number(1.0)),
            Some(&RuntimeValue::Number(2.0)),
            Some(&RuntimeValue::Number(3.0))
        ]
    );
}

#[test]
fn instance_create_nested_queue_keeps_instance_counts_visible_in_order() {
    let mut package = sample_package();
    add_spawned_object(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("spawned_count".into()),
                value: LoweredLogicExpr::Call {
                    name: "instance_number".into(),
                    args: vec![LoweredLogicExpr::Identifier(SPAWNED_OBJECT_NAME.into())],
                },
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::BinaryExpr {
                    op: "==".into(),
                    left: Box::new(LoweredLogicExpr::Identifier("spawned_count".into())),
                    right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                },
                then_branch: vec![create_spawned_at(112.0, 96.0)],
                else_branch: vec![],
            },
        ],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Repeat {
            count: LoweredLogicExpr::LiteralNumber(2.0),
            body: vec![create_spawned_at(80.0, 96.0)],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let created = spawned_instances(&core);
    assert_eq!(created.len(), 3);
    let seen_counts = created
        .iter()
        .map(|instance| instance.vars.get("spawned_count"))
        .collect::<Vec<_>>();
    assert_eq!(
        seen_counts,
        vec![
            Some(&RuntimeValue::Number(1.0)),
            Some(&RuntimeValue::Number(2.0)),
            Some(&RuntimeValue::Number(3.0))
        ]
    );
}

#[test]
fn destroyed_create_instance_is_not_counted_by_later_queued_creates() {
    let mut package = sample_package();
    add_spawned_object(
        &mut package,
        vec![
            LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("spawned_count".into()),
                value: LoweredLogicExpr::Call {
                    name: "instance_number".into(),
                    args: vec![LoweredLogicExpr::Identifier(SPAWNED_OBJECT_NAME.into())],
                },
            },
            LoweredLogicStatement::Conditional {
                condition: LoweredLogicExpr::BinaryExpr {
                    op: "&&".into(),
                    left: Box::new(LoweredLogicExpr::BinaryExpr {
                        op: "==".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("spawned_count".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    }),
                    right: Box::new(LoweredLogicExpr::BinaryExpr {
                        op: "==".into(),
                        left: Box::new(LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                            member: "first_spawn_destroyed".into(),
                        }),
                        right: Box::new(LoweredLogicExpr::LiteralBool(false)),
                    }),
                },
                then_branch: vec![
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                            member: "first_spawn_destroyed".into(),
                        },
                        value: LoweredLogicExpr::LiteralBool(true),
                    },
                    LoweredLogicStatement::FunctionCall {
                        name: "instance_destroy".into(),
                        args: vec![],
                    },
                ],
                else_branch: vec![],
            },
        ],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Repeat {
            count: LoweredLogicExpr::LiteralNumber(2.0),
            body: vec![create_spawned_at(80.0, 96.0)],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    core.globals.insert(
        "global.first_spawn_destroyed".into(),
        RuntimeValue::Bool(false),
    );

    core.execute_lowered_step_events(&mut host).unwrap();

    let created = spawned_instances(&core);
    assert_eq!(created.len(), 2);
    assert_eq!(
        created[0].vars.get("spawned_count"),
        Some(&RuntimeValue::Number(1.0))
    );
    assert!(!created[0].alive);
    assert_eq!(
        created[1].vars.get("spawned_count"),
        Some(&RuntimeValue::Number(1.0))
    );
    assert!(created[1].alive);
}

#[test]
fn repeat_instance_create_expression_assigns_members_to_created_instances_before_motion() {
    let mut package = sample_package();
    package.objects[1].name = "blood2".into();
    add_step_block(
        &mut package,
        vec![
            LoweredLogicStatement::VariableDeclaration {
                names: vec!["b".into()],
            },
            LoweredLogicStatement::Repeat {
                count: LoweredLogicExpr::LiteralNumber(2.0),
                body: vec![
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::Identifier("b".into()),
                        value: LoweredLogicExpr::Call {
                            name: "instance_create".into(),
                            args: vec![
                                LoweredLogicExpr::Identifier("x".into()),
                                LoweredLogicExpr::Identifier("y".into()),
                                LoweredLogicExpr::Identifier("blood2".into()),
                            ],
                        },
                    },
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("b".into())),
                            member: "direction".into(),
                        },
                        value: LoweredLogicExpr::LiteralNumber(0.0),
                    },
                    LoweredLogicStatement::Assignment {
                        target: LoweredLogicExpr::MemberAccess {
                            target: Box::new(LoweredLogicExpr::Identifier("b".into())),
                            member: "speed".into(),
                        },
                        value: LoweredLogicExpr::LiteralNumber(4.0),
                    },
                ],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let original_instance_count = core.current_room().unwrap().instances.len();

    core.tick(&mut host).unwrap();

    let created = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| {
            instance.object_name == "blood2" && instance.runtime_id >= original_instance_count
        })
        .collect::<Vec<_>>();
    assert_eq!(created.len(), 2);
    for instance in created {
        assert_eq!(instance.previous_x, 12.0);
        assert_eq!(instance.x, 16.0);
        assert_eq!(
            instance.vars.get("direction"),
            Some(&RuntimeValue::Number(0.0))
        );
        assert_eq!(instance.vars.get("speed"), Some(&RuntimeValue::Number(4.0)));
    }
}

#[test]
fn instance_create_accepts_numeric_object_id_argument() {
    let mut package = sample_package();
    package.objects[1].name = "obj_spawned_by_id".into();
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_create".into(),
            args: vec![
                LoweredLogicExpr::Identifier("x".into()),
                LoweredLogicExpr::Identifier("y".into()),
                LoweredLogicExpr::LiteralNumber(1.0),
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let original_instance_count = core.current_room().unwrap().instances.len();

    core.tick(&mut host).unwrap();

    assert!(core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .any(|instance| {
            instance.object_id == 1
                && instance.object_name == "obj_spawned_by_id"
                && instance.runtime_id >= original_instance_count
        }));
}

#[test]
fn collision_instance_destroy_dispatches_destroy_and_marks_owner_dead() {
    let mut package = sample_package();
    package.objects[0].name = "player".into();
    package.objects[2].name = "block".into();
    add_destroy_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "collision_destroy_ran".into(),
            },
            value: LoweredLogicExpr::LiteralBool(true),
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
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_destroy".into(),
            args: vec![],
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
        .find(|instance| instance.runtime_id == 0)
        .unwrap();
    assert!(!player.alive);
    assert_eq!(
        core.globals.get("global.collision_destroy_ran"),
        Some(&RuntimeValue::Bool(true))
    );
}

#[test]
fn non_player_step_motion_advances_instance_and_allows_collision_destroy() {
    let mut package = sample_package();
    package.objects[1].name = "bullet".into();
    package.objects[2].name = "block".into();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 1,
        sub_event: 0,
        event_tag: "destroy".into(),
        block_id: "object:1:event:1:0".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:1:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "bullet_destroy_ran".into(),
            },
            value: LoweredLogicExpr::LiteralBool(true),
        }],
    );
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 3,
        sub_event: 0,
        event_tag: "step".into(),
        block_id: "object:1:event:3:0".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:3:0".into(),
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::Identifier("hspeed".into()),
            value: LoweredLogicExpr::LiteralNumber(16.0),
        }],
    );
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 4,
        sub_event: 2,
        event_tag: "collision".into(),
        block_id: "object:1:event:4:2".into(),
        action_count: 0,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:4:2".into(),
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_destroy".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    {
        let room = core.current_room.as_mut().unwrap();
        let bullet = room
            .instances
            .iter_mut()
            .find(|instance| instance.object_name == "bullet")
            .unwrap();
        bullet.x = 24.0;
        bullet.y = 40.0;
        bullet.previous_x = 24.0;
        bullet.previous_y = 40.0;

        let block = room
            .instances
            .iter_mut()
            .find(|instance| instance.object_name == "block")
            .unwrap();
        block.x = 40.0;
        block.y = 40.0;
        block.previous_x = 40.0;
        block.previous_y = 40.0;
    }

    core.tick(&mut host).unwrap();

    let bullet = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_name == "bullet")
        .unwrap();
    assert_eq!(bullet.previous_x, 24.0);
    assert_eq!(bullet.x, 24.0);
    assert!(!bullet.alive);
    assert_eq!(
        core.globals.get("global.bullet_destroy_ran"),
        Some(&RuntimeValue::Bool(true))
    );
}
