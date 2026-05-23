use iwm_runtime_host::{ButtonState, RuntimeButton};

use crate::helpers::collides_at;
use crate::RuntimeCore;

use super::support::{host, sample_package};

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
