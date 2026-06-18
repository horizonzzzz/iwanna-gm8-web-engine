use iwm_runtime_host::{ButtonState, RuntimeButton};
use iwm_runtime_model::{RoomInstancePlacement, SpriteCollisionMask};

use crate::helpers::{collides_at, collision_candidate_indices_near, collision_candidates_near};
use crate::RuntimeCore;

use super::support::{add_room_create_block, capture_jump_trace, host, player, sample_package};
use crate::{LoweredLogicExpr, LoweredLogicStatement};

#[test]
fn core_moves_player_with_left_and_right_input() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
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
    let right_x = player(&core).x;
    assert!(right_x > 12.0);

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x25),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();
    assert!(player(&core).x <= right_x);
}

#[test]
fn core_jumps_when_on_spawn_and_jump_is_pressed() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
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

    assert!(player(&core).y <= 24.0);
}

#[test]
fn core_preserves_fractional_vertical_jump_motion() {
    let mut package = sample_package();
    package.lowered_logic = Some(crate::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![crate::LoweredLogicEntry {
            block_id: "object:0:event:0:0".into(),
            statements: vec![
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("jump".into()),
                    value: LoweredLogicExpr::LiteralNumber(8.5),
                },
                LoweredLogicStatement::Assignment {
                    target: LoweredLogicExpr::Identifier("gravity".into()),
                    value: LoweredLogicExpr::LiteralNumber(0.4),
                },
            ],
        }],
    });
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

    assert!(
        (player(&core).y - 15.9).abs() < 1e-9,
        "expected y=15.9 after jump/gravity, got {}",
        player(&core).y
    );
}

#[test]
fn core_uses_runtime_bound_jump_key_instead_of_hardcoded_space() {
    let mut package = sample_package();
    add_room_create_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "jumpbutton".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0x10 as f64),
        }],
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

    assert!(player(&core).y <= 24.0);
}

#[test]
fn core_uses_same_tick_step_updated_jump_binding_for_builtin_movement() {
    let mut package = sample_package();
    super::support::add_step_block(
        &mut package,
        vec![LoweredLogicStatement::Assignment {
            target: LoweredLogicExpr::MemberAccess {
                target: Box::new(LoweredLogicExpr::Identifier("global".into())),
                member: "jumpbutton".into(),
            },
            value: LoweredLogicExpr::LiteralNumber(0x10 as f64),
        }],
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

    assert!(player(&core).y < 24.0);
}

#[test]
fn core_stops_player_when_moving_into_a_solid() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 15,
            object_id: 2,
            x: 28,
            y: 24,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: true,
            is_hazard: false,
            is_checkpoint: false,
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

    assert_eq!(player(&core).x, 12.0);
}

#[test]
fn core_hazard_death_waits_for_restart_button_before_room_reset() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 13,
            object_id: 1,
            x: 12,
            y: 24,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: true,
            is_checkpoint: false,
        });
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert!(core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-player-died"));
    let death = core
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == "runtime-player-died")
        .unwrap();
    assert!(death.message.contains("room=7"));
    assert!(death.message.contains("tick=1"));
    assert!(death.message.contains("object=obj_player"));
    assert!(death.message.contains("reason=hazard"));
    assert!(!core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-room-changed"));

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x52),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    assert!(core.diagnostics().iter().any(|diagnostic| {
        diagnostic.code == "runtime-room-changed" && diagnostic.message.contains("reason=restart")
    }));
}

#[test]
fn core_hazard_collision_uses_sprite_masks_after_bbox_overlap() {
    let mut package = sample_package();
    package.objects[1].sprite_index = 1;
    package.objects[1].is_hazard = Some(true);
    package.resources.sprites[0].collision_masks = vec![filled_mask(16, 16)];
    package.resources.sprites[1].collision_masks = vec![single_pixel_mask(16, 16, 15, 15)];
    package.rooms[0].instances.push(RoomInstancePlacement {
        instance_id: 16,
        object_id: 1,
        x: 24,
        y: 24,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: true,
        is_checkpoint: false,
    });

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert!(!core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-player-died"));
}

#[test]
fn core_hazard_collision_triggers_when_sprite_mask_pixels_overlap() {
    let mut package = sample_package();
    package.objects[1].sprite_index = 1;
    package.objects[1].is_hazard = Some(true);
    package.resources.sprites[0].collision_masks = vec![filled_mask(16, 16)];
    package.resources.sprites[1].collision_masks = vec![single_pixel_mask(16, 16, 15, 15)];
    package.rooms[0].instances.push(RoomInstancePlacement {
        instance_id: 16,
        object_id: 1,
        x: 12,
        y: 24,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: true,
        is_checkpoint: false,
    });

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert!(core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-player-died"));
}

#[test]
fn core_updates_previous_position_before_moving_player() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
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
    assert_eq!((player.previous_x, player.previous_y), (12.0, 24.0));
    assert!(player.x > player.previous_x);
}

#[test]
fn core_moves_non_player_instances_from_direction_speed_and_gravity_vars() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let room = core.current_room.as_mut().unwrap();
    let marker = room
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker.x = 48.0;
    marker.y = 64.0;
    marker
        .vars
        .insert("direction".into(), crate::RuntimeValue::Number(0.0));
    marker
        .vars
        .insert("speed".into(), crate::RuntimeValue::Number(4.0));
    marker
        .vars
        .insert("gravity".into(), crate::RuntimeValue::Number(0.5));

    core.step_non_player_instances().unwrap();

    let marker = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    assert_eq!(marker.previous_x, 48.0);
    assert_eq!(marker.x, 52.0);
    assert_eq!(marker.y, 64.5);
}

#[test]
fn core_collisions_use_runtime_bbox_instead_of_whole_sprite_extents() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let room = core.current_room.as_mut().unwrap();
    let player_index = room
        .instances
        .iter()
        .position(|instance| instance.player_candidate)
        .unwrap();
    let solid_index = room
        .instances
        .iter()
        .position(|instance| instance.solid)
        .unwrap();

    room.instances[player_index].x = 12.0;
    room.instances[player_index].y = 24.0;
    room.instances[player_index].width = 32;
    room.instances[player_index].height = 32;
    room.instances[player_index].bbox_left = 8;
    room.instances[player_index].bbox_right = 23;
    room.instances[player_index].bbox_top = 8;
    room.instances[player_index].bbox_bottom = 23;
    room.instances[solid_index].x = 12.0;
    room.instances[solid_index].y = 48.0;
    room.instances[solid_index].width = 32;
    room.instances[solid_index].height = 32;
    room.instances[solid_index].bbox_left = 0;
    room.instances[solid_index].bbox_right = 31;
    room.instances[solid_index].bbox_top = 0;
    room.instances[solid_index].bbox_bottom = 31;

    let player = room.instances[player_index].clone();
    let solids = vec![room.instances[solid_index].clone()];
    assert!(!collides_at(
        &player,
        player.x,
        player.y,
        &solids,
        Some(player.runtime_id)
    ));
}

#[test]
fn core_filters_player_collision_candidates_to_nearby_instances() {
    let mut package = sample_package();
    for index in 0..250 {
        package.rooms[0]
            .instances
            .push(iwm_runtime_model::RoomInstancePlacement {
                instance_id: 2000 + index,
                object_id: 2,
                x: 10_000 + index as i32 * 32,
                y: 10_000,
                xscale: 1.0,
                yscale: 1.0,
                angle: 0.0,
                blend: 0x00ff_ffff,
                creation_block_id: None,
                is_solid: true,
                is_hazard: false,
                is_checkpoint: false,
            });
    }
    let core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();
    let player_index = room
        .instances
        .iter()
        .position(|instance| instance.player_candidate)
        .unwrap();
    let player = room.instances[player_index].clone();
    let candidates = collision_candidates_near(
        &player,
        player.x,
        player.y,
        &room.instances,
        Some(player.runtime_id),
        32.0,
        |instance| instance.alive && instance.solid,
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].object_name, "obj_block");
    assert!(collides_at(
        &player,
        player.x,
        player.y + 16.0,
        &candidates,
        Some(player.runtime_id)
    ));
}

#[test]
fn core_can_find_nearby_collision_candidate_indices_without_cloning_candidates() {
    let mut package = sample_package();
    for index in 0..250 {
        package.rooms[0].instances.push(RoomInstancePlacement {
            instance_id: 3000 + index,
            object_id: 2,
            x: 20_000 + index as i32 * 32,
            y: 20_000,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: true,
            is_hazard: false,
            is_checkpoint: false,
        });
    }
    let core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();
    let player_index = room
        .instances
        .iter()
        .position(|instance| instance.player_candidate)
        .unwrap();
    let player = &room.instances[player_index];

    let candidate_indices = collision_candidate_indices_near(
        player,
        player.x,
        player.y,
        &room.instances,
        Some(player.runtime_id),
        32.0,
        |instance| instance.alive && instance.solid,
    );

    assert_eq!(candidate_indices.len(), 1);
    assert_eq!(
        room.instances[candidate_indices[0]].object_name,
        "obj_block"
    );
}

#[test]
fn core_tracks_left_facing_state_for_player_movement() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x25),
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
    assert!(player.facing_left);

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x27),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(!player.facing_left);
}

#[test]
fn core_initializes_and_clears_jump_state_on_room_reset() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();

    let initial_player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        initial_player.jump,
        crate::types::RuntimeJumpState::default()
    );

    let room = core.current_room.as_mut().unwrap();
    let player = room
        .instances
        .iter_mut()
        .find(|instance| instance.player_candidate)
        .unwrap();
    player.jump.active = true;
    player.jump.hold_frames = 5;
    player.jump.cut_applied = true;
    player.jump.grounded_last_tick = false;

    core.reset_player_to_spawn();

    let reset_player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(reset_player.jump, crate::types::RuntimeJumpState::default());
}

#[test]
fn core_tap_jump_reaches_lower_apex_than_held_jump() {
    fn run_jump_sequence(held_frames: usize) -> f64 {
        let mut package = sample_package();
        package.rooms[0].transition_targets.clear();
        let mut core = RuntimeCore::load(package).unwrap();
        let mut host = host();
        let mut min_y = f64::INFINITY;

        for frame in 0..12 {
            let pressed = frame < held_frames;
            host.input.replace_button_states([(
                RuntimeButton::Keyboard(0x20),
                ButtonState {
                    pressed,
                    just_pressed: frame == 0,
                    just_released: frame == held_frames,
                },
            )]);
            core.tick(&mut host).unwrap();
            let player = core
                .current_room()
                .unwrap()
                .instances
                .iter()
                .find(|instance| instance.player_candidate)
                .unwrap();
            min_y = min_y.min(player.y);
            host.input.clear_transitions();
        }

        min_y
    }

    let tap_apex = run_jump_sequence(1);
    let held_apex = run_jump_sequence(4);
    assert!(held_apex < tap_apex);
}

#[test]
fn core_ceiling_hit_clears_upward_jump_phase() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 99,
            object_id: 2,
            x: 12,
            y: 0,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: true,
            is_hazard: false,
            is_checkpoint: false,
        });

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

    for _ in 0..6 {
        core.tick(&mut host).unwrap();
        host.input.clear_transitions();
    }

    let player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(!player.jump.active);
    assert!(player.vspeed >= 0.0);
}

#[test]
fn core_jump_release_marks_cut_state_during_upward_motion() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
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
    host.input.clear_transitions();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x20),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
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
    assert!(player.jump.cut_applied);
}

#[test]
fn core_jump_trace_distinguishes_release_cut_and_landing_reset() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
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
    host.input.clear_transitions();

    host.input.set_button_state(
        RuntimeButton::Keyboard(0x20),
        ButtonState {
            pressed: false,
            just_pressed: false,
            just_released: true,
        },
    );
    core.tick(&mut host).unwrap();
    let after_cut = capture_jump_trace(&core);
    assert!(after_cut.jump_cut_applied);

    for _ in 0..32 {
        host.input.clear_transitions();
        core.tick(&mut host).unwrap();
        let trace = capture_jump_trace(&core);
        if trace.grounded {
            assert!(!trace.jump_active);
            assert_eq!(trace.jump_hold_frames, 0);
            return;
        }
    }

    panic!("player did not land within 32 ticks");
}

#[test]
fn core_snapshot_exposes_player_jump_trace_state() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
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

    let snapshot = core.snapshot();
    let player = snapshot.player.expect("expected player snapshot");
    assert!(player.jump.active);
    assert_eq!(player.jump.hold_frames, 1);
    assert!(!player.jump.cut_applied);
    assert!(!player.jump.grounded);
}

fn filled_mask(width: u32, height: u32) -> SpriteCollisionMask {
    SpriteCollisionMask {
        width,
        height,
        bbox_left: 0,
        bbox_right: width - 1,
        bbox_top: 0,
        bbox_bottom: height - 1,
        data: vec![true; (width * height) as usize],
    }
}

fn single_pixel_mask(width: u32, height: u32, x: u32, y: u32) -> SpriteCollisionMask {
    let mut data = vec![false; (width * height) as usize];
    data[(y * width + x) as usize] = true;
    SpriteCollisionMask {
        width,
        height,
        bbox_left: x,
        bbox_right: x,
        bbox_top: y,
        bbox_bottom: y,
        data,
    }
}

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
        .insert("speed".into(), crate::RuntimeValue::Number(3.0));
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
    assert!((dir - 306.86989764584405).abs() < 1e-6, "direction={}", dir);
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
        .insert("friction".into(), crate::RuntimeValue::Number(1.0));
    crate::movement::apply_gm_motion_vars(&mut instance);
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
        .insert("friction".into(), crate::RuntimeValue::Number(1.0));
    crate::movement::apply_gm_motion_vars(&mut instance);
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
        .insert("friction".into(), crate::RuntimeValue::Number(1.0));
    instance
        .vars
        .insert("gravity".into(), crate::RuntimeValue::Number(0.5));
    instance.vars.insert(
        "gravity_direction".into(),
        crate::RuntimeValue::Number(270.0),
    );
    crate::movement::apply_gm_motion_vars(&mut instance);
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

    let collided = crate::helpers::move_instance_axis(
        &mut mover,
        &[solid],
        Some(mover_runtime_id),
        crate::helpers::Axis::Horizontal,
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

fn make_test_instance() -> crate::RuntimeInstance {
    crate::RuntimeInstance {
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
        jump: crate::RuntimeJumpState::default(),
        vars: std::collections::HashMap::new(),
    }
}

fn filled_runtime_mask(size: u32, height: u32) -> crate::RuntimeCollisionMask {
    crate::RuntimeCollisionMask {
        width: size,
        height,
        bbox_left: 0,
        bbox_top: 0,
        bbox_right: size as i32 - 1,
        bbox_bottom: height as i32 - 1,
        data: vec![true; (size * height) as usize],
    }
}

fn as_number(value: &crate::RuntimeValue) -> Option<f64> {
    match value {
        crate::RuntimeValue::Number(n) => Some(*n),
        crate::RuntimeValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        crate::RuntimeValue::Text(t) => t.parse().ok(),
    }
}
