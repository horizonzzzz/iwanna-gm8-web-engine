use iwm_runtime_host::{ButtonState, RuntimeButton};

use crate::helpers::collides_at;
use crate::RuntimeCore;

use super::support::{capture_jump_trace, host, sample_package};

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
    let after_right = core.current_room().unwrap();
    let player = after_right
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    let right_x = player.x;
    assert!(right_x > 12);

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x25),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();
    let after_left = core.current_room().unwrap();
    let player = after_left
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(player.x <= right_x);
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

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(player.y <= 24);
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

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(player.x, 12);
}

#[test]
fn core_emits_hazard_diagnostic_and_requests_reset() {
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
    assert_eq!(core.snapshot().status, crate::RuntimeStatus::Ready);
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
    assert_eq!((player.previous_x, player.previous_y), (12, 24));
    assert!(player.x > player.previous_x);
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

    room.instances[player_index].x = 12;
    room.instances[player_index].y = 24;
    room.instances[player_index].width = 32;
    room.instances[player_index].height = 32;
    room.instances[player_index].bbox_left = 8;
    room.instances[player_index].bbox_right = 23;
    room.instances[player_index].bbox_top = 8;
    room.instances[player_index].bbox_bottom = 23;
    room.instances[solid_index].x = 12;
    room.instances[solid_index].y = 48;
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
    assert_eq!(initial_player.jump, crate::types::RuntimeJumpState::default());

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
    fn run_jump_sequence(held_frames: usize) -> i32 {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = host();
        let mut min_y = i32::MAX;

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
    assert!(player.vspeed >= 0);
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
