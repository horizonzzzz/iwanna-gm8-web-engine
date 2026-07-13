use iwm_runtime_model::ObjectEventEntry;

#[cfg(feature = "local-sample-tests")]
use super::support::local_sample_package;
use super::support::{add_step_block, append_lowered_entry, host, sample_package};
use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

fn assignment(name: &str, value: LoweredLogicExpr) -> LoweredLogicStatement {
    LoweredLogicStatement::Assignment {
        target: LoweredLogicExpr::Identifier(name.into()),
        value,
    }
}

fn indexed_member(receiver: &str, member: &str, index: f64) -> LoweredLogicExpr {
    LoweredLogicExpr::IndexAccess {
        target: Box::new(LoweredLogicExpr::MemberAccess {
            target: Box::new(LoweredLogicExpr::Identifier(receiver.into())),
            member: member.into(),
        }),
        index: Box::new(LoweredLogicExpr::LiteralNumber(index)),
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
fn timeline_object_indexed_alarm_assignment_starts_target_alarm() {
    let mut package = sample_package();
    package.objects[1].events.push(ObjectEventEntry {
        event_type: 2,
        sub_event: 0,
        event_tag: "alarm:0".into(),
        block_id: "object:1:event:2:0".into(),
        action_count: 1,
    });
    append_lowered_entry(
        &mut package,
        "object:1:event:2:0".into(),
        vec![assignment(
            "alarm_fired",
            LoweredLogicExpr::LiteralBool(true),
        )],
    );
    append_lowered_entry(
        &mut package,
        "timeline:18:1".into(),
        vec![LoweredLogicStatement::Assignment {
            target: indexed_member("obj_marker", "alarm", 0.0),
            value: LoweredLogicExpr::LiteralNumber(2.0),
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

    for _ in 0..3 {
        core.tick(&mut runtime_host).unwrap();
    }

    let room = core.current_room().unwrap();
    let owner = &room.instances[0];
    let marker = room
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    assert_eq!(
        marker.vars.get("alarm_fired"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        marker.vars.get("alarm[0]"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert!(!owner.vars.contains_key("obj_marker.alarm[0]"));
}

#[test]
fn object_indexed_assignment_updates_every_live_target() {
    let mut package = sample_package();
    package.objects[2].parent_index = 1;
    let mut second_marker = package.rooms[0]
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap()
        .clone();
    second_marker.instance_id = 16;
    second_marker.x = 80;
    package.rooms[0].instances.push(second_marker);
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: indexed_member("obj_marker", "values", 3.0),
            value: LoweredLogicExpr::LiteralNumber(7.0),
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();

    core.tick(&mut host()).unwrap();

    let room = core.current_room().unwrap();
    let markers = room
        .instances
        .iter()
        .filter(|instance| matches!(instance.object_id, 1 | 2))
        .collect::<Vec<_>>();
    assert_eq!(markers.len(), 3);
    assert!(markers
        .iter()
        .all(|marker| { marker.vars.get("values[3]") == Some(&RuntimeValue::Number(7.0)) }));
    assert!(!room.instances[0].vars.contains_key("obj_marker.values[3]"));
}

#[test]
fn object_indexed_assignment_with_no_live_target_is_a_noop() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .retain(|instance| instance.object_id != 1);
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: indexed_member("obj_marker", "values", 3.0),
            value: LoweredLogicExpr::LiteralNumber(7.0),
        }],
    );
    let mut core = RuntimeCore::load(package).unwrap();

    core.tick(&mut host()).unwrap();

    let room = core.current_room().unwrap();
    assert!(room
        .instances
        .iter()
        .all(|instance| !instance.vars.contains_key("values[3]")));
    assert!(!room.instances[0].vars.contains_key("obj_marker.values[3]"));
}

#[test]
fn object_indexed_read_uses_first_live_target() {
    let mut package = sample_package();
    add_step_block(
        &mut package,
        vec![assignment(
            "observed_value",
            indexed_member("obj_marker", "values", 3.0),
        )],
    );
    let mut core = RuntimeCore::load(package).unwrap();
    core.current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap()
        .vars
        .insert("values[3]".into(), RuntimeValue::Number(9.0));

    core.tick(&mut host()).unwrap();

    assert_eq!(
        core.current_room().unwrap().instances[0]
            .vars
            .get("observed_value"),
        Some(&RuntimeValue::Number(9.0))
    );
}

#[cfg(feature = "local-sample-tests")]
#[test]
fn ariotrials_room156_time_limit_counts_down() {
    let Some(package) = local_sample_package("ariotrials") else {
        return;
    };
    let mut core = RuntimeCore::load(package).unwrap();
    core.reload_room(156).unwrap();
    let mut runtime_host = host();

    for _ in 0..120 {
        core.tick(&mut runtime_host).unwrap();
    }

    let room = core.current_room().unwrap();
    let timer = room
        .instances
        .iter()
        .find(|instance| instance.object_name == "timelimitobject")
        .unwrap();
    assert!(
        matches!(timer.vars.get("setS"), Some(RuntimeValue::Number(seconds)) if *seconds < 55.0),
        "expected room156 setS to count down from 55, vars={:?}",
        timer.vars
    );
    assert!(room
        .instances
        .iter()
        .filter(|instance| instance.object_name == "Taiko")
        .all(|instance| !instance.vars.contains_key("timelimitobject.alarm[0]")));
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
