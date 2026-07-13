use iwm_runtime_model::ObjectEventEntry;

use super::support::{append_lowered_entry, host, sample_package};
use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

fn assignment(name: &str, value: LoweredLogicExpr) -> LoweredLogicStatement {
    LoweredLogicStatement::Assignment {
        target: LoweredLogicExpr::Identifier(name.into()),
        value,
    }
}

#[test]
fn runtime_timeline_fires_crossed_moments_and_runs_created_instance_create_event() {
    let mut package = sample_package();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 0,
        sub_event: 0,
        event_tag: "create".into(),
        block_id: "object:1:event:0:0".into(),
        action_count: 1,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:0:0".into(),
        vec![assignment(
            "created_by_timeline",
            LoweredLogicExpr::LiteralBool(true),
        )],
    );
    append_lowered_entry(
        &mut package,
        "timeline:18:1".into(),
        vec![assignment(
            "timeline_moment",
            LoweredLogicExpr::LiteralNumber(1.0),
        )],
    );
    append_lowered_entry(
        &mut package,
        "timeline:18:2".into(),
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_create".into(),
            args: vec![
                LoweredLogicExpr::LiteralNumber(80.0),
                LoweredLogicExpr::LiteralNumber(90.0),
                LoweredLogicExpr::LiteralNumber(1.0),
            ],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let owner = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .first_mut()
        .unwrap();
    owner
        .vars
        .insert("timeline_index".into(), RuntimeValue::Number(18.0));
    owner
        .vars
        .insert("timeline_position".into(), RuntimeValue::Number(0.0));
    owner
        .vars
        .insert("timeline_speed".into(), RuntimeValue::Number(1.0));
    owner
        .vars
        .insert("timeline_running".into(), RuntimeValue::Bool(true));
    let mut runtime_host = host();

    core.tick(&mut runtime_host).unwrap();
    assert!(!core.current_room().unwrap().instances[0]
        .vars
        .contains_key("timeline_moment"));
    core.tick(&mut runtime_host).unwrap();
    assert_eq!(
        core.current_room().unwrap().instances[0]
            .vars
            .get("timeline_moment"),
        Some(&RuntimeValue::Number(1.0))
    );
    core.tick(&mut runtime_host).unwrap();
    assert!(core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .any(|instance| {
            instance.object_id == 1
                && instance.x == 80.0
                && instance.y == 90.0
                && instance.vars.get("created_by_timeline") == Some(&RuntimeValue::Bool(true))
        }));
}

#[test]
fn timeline_applies_to_object_target_without_destroying_owner() {
    let mut package = sample_package();
    append_lowered_entry(
        &mut package,
        "timeline:18:1".into(),
        vec![LoweredLogicStatement::With {
            target: LoweredLogicExpr::Call {
                name: "__iwm_object".into(),
                args: vec![LoweredLogicExpr::LiteralNumber(1.0)],
            },
            body: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_destroy".into(),
                args: vec![],
            }],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let owner = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .first_mut()
        .unwrap();
    owner
        .vars
        .insert("timeline_index".into(), RuntimeValue::Number(18.0));
    owner
        .vars
        .insert("timeline_running".into(), RuntimeValue::Bool(true));
    let mut runtime_host = host();

    core.tick(&mut runtime_host).unwrap();
    core.tick(&mut runtime_host).unwrap();

    let room = core.current_room().unwrap();
    assert!(
        room.instances[0].alive,
        "timeline owner must survive targeted action"
    );
    assert!(
        !room
            .instances
            .iter()
            .find(|instance| instance.object_id == 1)
            .unwrap()
            .alive
    );
}

#[test]
fn runtime_dispatches_outside_event_after_motion() {
    let mut package = sample_package();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 7,
        sub_event: 0,
        event_tag: "other:outside".into(),
        block_id: "object:1:event:7:0".into(),
        action_count: 1,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:7:0".into(),
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_destroy".into(),
            args: vec![],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker.x = 400.0;

    core.tick(&mut host()).unwrap();
    assert!(
        !core
            .current_room()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.object_id == 1)
            .unwrap()
            .alive
    );
}

#[test]
fn runtime_action_wrap_moves_horizontal_instance_across_room() {
    let mut package = sample_package();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 7,
        sub_event: 0,
        event_tag: "other:outside".into(),
        block_id: "object:1:event:7:0".into(),
        action_count: 1,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:7:0".into(),
        vec![LoweredLogicStatement::FunctionCall {
            name: "__iwm_action_wrap".into(),
            args: vec![LoweredLogicExpr::LiteralNumber(0.0)],
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker.x = 321.0;
    marker.set_hspeed(1.0);

    core.tick(&mut host()).unwrap();
    let marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    assert!(marker.alive);
    assert!(marker.x < 10.0, "expected horizontal wrap, x={}", marker.x);
}

#[test]
fn point_direction_and_random_are_deterministic_but_stateful() {
    fn run_once() -> (RuntimeValue, RuntimeValue, RuntimeValue, RuntimeValue) {
        let mut package = sample_package();
        super::support::add_step_block(
            &mut package,
            vec![
                assignment(
                    "angle",
                    LoweredLogicExpr::Call {
                        name: "point_direction".into(),
                        args: vec![
                            LoweredLogicExpr::LiteralNumber(0.0),
                            LoweredLogicExpr::LiteralNumber(0.0),
                            LoweredLogicExpr::LiteralNumber(0.0),
                            LoweredLogicExpr::LiteralNumber(-10.0),
                        ],
                    },
                ),
                assignment(
                    "random_a",
                    LoweredLogicExpr::Call {
                        name: "random".into(),
                        args: vec![LoweredLogicExpr::LiteralNumber(100.0)],
                    },
                ),
                assignment(
                    "random_b",
                    LoweredLogicExpr::Call {
                        name: "random".into(),
                        args: vec![LoweredLogicExpr::LiteralNumber(100.0)],
                    },
                ),
                assignment(
                    "irandom_value",
                    LoweredLogicExpr::Call {
                        name: "irandom".into(),
                        args: vec![LoweredLogicExpr::LiteralNumber(5.0)],
                    },
                ),
            ],
        );
        let mut core = RuntimeCore::load(package).unwrap();
        core.tick(&mut host()).unwrap();
        let vars = &core.current_room().unwrap().instances[0].vars;
        (
            vars["angle"].clone(),
            vars["random_a"].clone(),
            vars["random_b"].clone(),
            vars["irandom_value"].clone(),
        )
    }

    let first = run_once();
    let second = run_once();
    assert_eq!(first, second);
    assert_eq!(first.0, RuntimeValue::Number(90.0));
    assert_ne!(first.1, first.2);
    assert!(
        matches!(first.3, RuntimeValue::Number(value) if value.fract() == 0.0 && (0.0..=5.0).contains(&value))
    );
}
