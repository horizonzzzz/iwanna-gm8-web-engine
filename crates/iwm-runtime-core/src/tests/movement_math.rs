use crate::helpers::{move_instance_axis, Axis};
use crate::movement::apply_gm_motion_vars;
use crate::{RuntimeCollisionMask, RuntimeInstance, RuntimeJumpState, RuntimeValue};

#[test]
fn set_speed_syncs_hvspeed() {
    let mut instance = make_test_instance();
    instance.set_speed(4.0);
    assert_eq!(instance.hspeed, 4.0);
    assert_eq!(instance.vspeed, 0.0);
}

#[test]
fn set_direction_syncs_hvspeed() {
    let mut instance = make_test_instance();
    instance
        .vars
        .insert("speed".into(), RuntimeValue::Number(3.0));
    instance.set_direction(90.0);
    assert!((instance.hspeed - 0.0).abs() < 1e-9);
    assert!((instance.vspeed - (-3.0)).abs() < 1e-9);
}

#[test]
fn set_hspeed_syncs_speed_direction() {
    let mut instance = make_test_instance();
    instance.set_hvspeed(3.0, 4.0);
    assert!((instance.vars.get("speed").and_then(as_number).unwrap() - 5.0).abs() < 1e-9);
    let dir = instance.vars.get("direction").and_then(as_number).unwrap();
    assert!((dir - 306.86989764584405).abs() < 1e-6, "direction={dir}");
}

#[test]
fn gm_round_tolerance_snaps_near_integers() {
    let mut instance = make_test_instance();
    instance.set_direction(0.0);
    instance.set_speed(3.0);
    assert_eq!(instance.hspeed, 3.0);
    assert_eq!(instance.vspeed, 0.0);
}

#[test]
fn friction_pulls_speed_toward_zero() {
    let mut instance = make_test_instance();
    instance.set_speed(5.0);
    instance.set_direction(0.0);
    instance
        .vars
        .insert("friction".into(), RuntimeValue::Number(1.0));
    apply_gm_motion_vars(&mut instance);
    let speed = instance.vars.get("speed").and_then(as_number).unwrap();
    assert!((speed - 4.0).abs() < 1e-9);
}

#[test]
fn friction_does_not_overshoot_zero() {
    let mut instance = make_test_instance();
    instance.set_speed(0.5);
    instance.set_direction(0.0);
    instance
        .vars
        .insert("friction".into(), RuntimeValue::Number(1.0));
    apply_gm_motion_vars(&mut instance);
    let speed = instance.vars.get("speed").and_then(as_number).unwrap();
    assert!((speed - 0.0).abs() < 1e-9);
}

#[test]
fn gravity_applied_after_friction() {
    let mut instance = make_test_instance();
    instance.set_speed(4.0);
    instance.set_direction(0.0);
    instance
        .vars
        .insert("friction".into(), RuntimeValue::Number(1.0));
    instance
        .vars
        .insert("gravity".into(), RuntimeValue::Number(0.5));
    instance
        .vars
        .insert("gravity_direction".into(), RuntimeValue::Number(270.0));
    apply_gm_motion_vars(&mut instance);
    assert!((instance.hspeed - 3.0).abs() < 1e-9);
    assert!((instance.vspeed - 0.5).abs() < 1e-9);
}

#[test]
fn blood2_direction_speed_assignment_syncs_immediately() {
    let mut instance = make_test_instance();
    instance.set_direction(45.0);
    instance.set_speed(6.0);
    let expected_h = 45f64.to_radians().cos() * 6.0;
    let expected_v = -45f64.to_radians().sin() * 6.0;
    assert!(
        (instance.hspeed - expected_h).abs() < 1e-9,
        "hspeed={}, expected={}",
        instance.hspeed,
        expected_h
    );
    assert!(
        (instance.vspeed - expected_v).abs() < 1e-9,
        "vspeed={}, expected={}",
        instance.vspeed,
        expected_v
    );
}

#[test]
fn collision_stop_keeps_speed_direction_in_sync() {
    let mut mover = make_test_instance();
    mover.width = 16;
    mover.height = 16;
    mover.bbox_right = 15;
    mover.bbox_bottom = 15;
    mover.collision_masks = vec![filled_runtime_mask(16, 16)];
    mover.set_direction(0.0);
    mover.set_speed(4.0);

    let mut solid = make_test_instance();
    solid.runtime_id = 1;
    solid.instance_id = 1;
    solid.object_name = "solid".into();
    solid.x = 19.0;
    solid.y = 0.0;
    solid.width = 16;
    solid.height = 16;
    solid.bbox_right = 15;
    solid.bbox_bottom = 15;
    solid.solid = true;
    solid.collision_masks = vec![filled_runtime_mask(16, 16)];
    let mover_runtime_id = mover.runtime_id;
    let delta = mover.hspeed;

    let collided = move_instance_axis(
        &mut mover,
        &[solid],
        Some(mover_runtime_id),
        Axis::Horizontal,
        delta,
    );

    assert!(collided, "expected horizontal collision to stop the mover");
    assert_eq!(mover.hspeed, 0.0);
    assert_eq!(mover.vspeed, 0.0);
    assert_eq!(mover.vars.get("speed").and_then(as_number), Some(0.0));
    assert_eq!(mover.vars.get("direction").and_then(as_number), Some(0.0));
}

#[test]
fn zero_hvspeed_assignment_clears_speed_direction() {
    let mut instance = make_test_instance();
    instance.set_direction(45.0);
    instance.set_speed(6.0);

    instance.set_hvspeed(0.0, 0.0);

    assert_eq!(instance.hspeed, 0.0);
    assert_eq!(instance.vspeed, 0.0);
    assert_eq!(instance.vars.get("speed").and_then(as_number), Some(0.0));
    assert_eq!(
        instance.vars.get("direction").and_then(as_number),
        Some(0.0)
    );
}

fn make_test_instance() -> RuntimeInstance {
    RuntimeInstance {
        runtime_id: 0,
        instance_id: 0,
        object_id: 0,
        object_name: "test".into(),
        x: 0.0,
        y: 0.0,
        previous_x: 0.0,
        previous_y: 0.0,
        hspeed: 0.0,
        vspeed: 0.0,
        width: 16,
        height: 16,
        origin_x: 0,
        origin_y: 0,
        bbox_left: 0,
        bbox_right: 15,
        bbox_top: 0,
        bbox_bottom: 15,
        collision_masks: vec![],
        per_frame_collision_masks: false,
        facing_left: false,
        visible: true,
        alive: true,
        persistent: false,
        solid: false,
        hazard: false,
        checkpoint: false,
        player_candidate: false,
        jump: RuntimeJumpState::default(),
        vars: std::collections::HashMap::new(),
    }
}

fn filled_runtime_mask(size: u32, height: u32) -> RuntimeCollisionMask {
    RuntimeCollisionMask {
        width: size,
        height,
        bbox_left: 0,
        bbox_top: 0,
        bbox_right: size as i32 - 1,
        bbox_bottom: height as i32 - 1,
        data: vec![true; (size * height) as usize],
    }
}

fn as_number(value: &RuntimeValue) -> Option<f64> {
    match value {
        RuntimeValue::Number(n) => Some(*n),
        RuntimeValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        RuntimeValue::Text(t) => t.parse().ok(),
    }
}
