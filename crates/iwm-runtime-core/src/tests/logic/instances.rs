use super::*;

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

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.runtime_id == 0)
        .unwrap();
    assert!(!player.alive);
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
}

#[test]
fn runtime_instance_create_event_can_see_created_instance() {
    let mut package = sample_package();
    package.objects.push(ObjectDefinition {
        id: 4,
        name: "obj_spawned".into(),
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
            block_id: "object:4:event:0:0".into(),
            action_count: 0,
        }],
    });
    package.manifest.object_count = package.objects.len();
    append_lowered_entry(
        &mut package,
        "object:4:event:0:0".into(),
        vec![LoweredLogicStatement::Conditional {
            condition: LoweredLogicExpr::Call {
                name: "instance_exists".into(),
                args: vec![LoweredLogicExpr::Identifier("obj_spawned".into())],
            },
            then_branch: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("saw_self_object".into()),
                value: LoweredLogicExpr::LiteralBool(true),
            }],
            else_branch: vec![],
        }],
    );
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_create".into(),
            args: vec![
                LoweredLogicExpr::LiteralNumber(80.0),
                LoweredLogicExpr::LiteralNumber(96.0),
                LoweredLogicExpr::Identifier("obj_spawned".into()),
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.execute_lowered_step_events(&mut host).unwrap();

    let created = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| {
            instance.object_name == "obj_spawned" && (instance.x, instance.y) == (80.0, 96.0)
        })
        .unwrap();
    assert_eq!(
        created.vars.get("saw_self_object"),
        Some(&RuntimeValue::Bool(true))
    );
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
