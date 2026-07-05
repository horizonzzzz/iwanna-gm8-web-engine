use iwm_runtime_host::{ButtonState, RuntimeButton};
use iwm_runtime_model::RoomInstancePlacement;

use crate::{RuntimeCore, RuntimeStatus, RuntimeValue};

use super::support::{capture_jump_trace, host, sample_package};

#[test]
fn core_ticks_and_submits_a_frame() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert_eq!(core.status(), RuntimeStatus::Running);
    assert_eq!(core.tick_count(), 1);
    assert_eq!(host.renderer.submitted_frames.len(), 1);
    assert!(host.renderer.submitted_frames[0]
        .commands
        .iter()
        .any(|command| matches!(command, iwm_runtime_host::RuntimeDrawCommand::Present)));
}

#[test]
fn core_emits_idle_diagnostic_when_no_input_is_active() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert!(core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-idle"
            && matches!(
                diagnostic.level,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info
            )));
}

#[test]
fn core_records_idle_diagnostics_in_the_host_sink() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    assert!(host
        .diagnostics
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-idle"
            && matches!(
                diagnostic.level,
                iwm_runtime_host::RuntimeDiagnosticLevel::Info
            )));
}

#[test]
fn core_keeps_runtime_diagnostics_bounded_over_many_idle_ticks() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    for _ in 0..300 {
        core.tick(&mut host).unwrap();
    }

    assert!(core.diagnostics().len() <= 64);
    assert!(host.diagnostics.diagnostics.len() <= 64);
    assert!(core
        .diagnostics()
        .last()
        .map(|diagnostic| diagnostic.message.contains("tick 300"))
        .unwrap_or(false));
}

#[test]
fn core_resets_player_back_to_spawn() {
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

    host.input.replace_button_states([(
        RuntimeButton::Restart,
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
    assert_eq!((player.x, player.y), (12.0, 24.0));
}

#[test]
fn core_reset_clears_player_speed_direction() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let room = core.current_room.as_mut().unwrap();
    let player = room
        .instances
        .iter_mut()
        .find(|instance| instance.player_candidate)
        .unwrap();
    player.set_direction(45.0);
    player.set_speed(6.0);

    core.reset_player_to_spawn();

    let reset_player = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(reset_player.hspeed, 0.0);
    assert_eq!(reset_player.vspeed, 0.0);
    assert_eq!(
        reset_player.vars.get("speed"),
        Some(&RuntimeValue::Number(0.0))
    );
    assert_eq!(
        reset_player.vars.get("direction"),
        Some(&RuntimeValue::Number(0.0))
    );
}

#[test]
fn core_transitions_to_target_room_when_requested() {
    let mut package = sample_package();
    package.rooms[0].transition_targets = vec![1];
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.request_room_transition(9);
    core.tick(&mut host).unwrap();
    assert_eq!(core.snapshot().room_id, Some(9));
}

#[test]
fn core_replaces_room_player_with_persistent_player_on_transition() {
    let mut package = sample_package();
    package.objects[0].persistent = true;
    package.rooms[1].instances.push(RoomInstancePlacement {
        instance_id: 90,
        object_id: 0,
        x: 144,
        y: 96,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: false,
        is_checkpoint: false,
    });
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let player = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| crate::helpers::is_player_instance(instance))
        .unwrap();
    player.x = 72.0;
    player.y = 80.0;

    core.request_room_transition(9);
    core.apply_pending_room_change(&mut host).unwrap();

    let players = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .filter(|instance| crate::helpers::is_player_instance(instance))
        .collect::<Vec<_>>();
    assert_eq!(players.len(), 1, "room should have a single live player");
    assert_eq!(players[0].instance_id, 11);
    assert_eq!((players[0].x, players[0].y), (72.0, 80.0));
}

#[test]
fn core_only_restarts_on_restart_press_edge() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    host.input.replace_button_states([(
        RuntimeButton::Restart,
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
    assert_eq!((player.x, player.y), (12.0, 24.0));

    host.input.replace_button_states([
        (
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: true,
                just_pressed: false,
                just_released: false,
            },
        ),
        (
            RuntimeButton::Restart,
            ButtonState {
                pressed: true,
                just_pressed: false,
                just_released: false,
            },
        ),
    ]);
    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(player.x > 12.0);
}

#[test]
fn core_emits_restart_request_diagnostic_on_restart_press_edge() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();

    host.input.replace_button_states([(
        RuntimeButton::Restart,
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    assert!(core
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime-room-restart-requested"));
}

#[test]
fn raw_r_does_not_restart_without_runtime_binding() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let player = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.player_candidate)
        .unwrap();
    player.x = 72.0;
    player.previous_x = 72.0;
    let mut host = host();

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x52),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
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
    assert_eq!(player.x, 72.0);
    assert!(core
        .diagnostics()
        .iter()
        .all(|diagnostic| diagnostic.code != "runtime-room-restart-requested"));
}

#[test]
fn core_uses_runtime_bound_restart_key_without_default_r() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    core.globals.insert(
        "global.restartbutton".into(),
        RuntimeValue::Number(0x53 as f64),
    );
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

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x52),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    let player_after_r = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert!(
        player_after_r.x > 12.0,
        "raw R should not reset when global.restartbutton is bound elsewhere"
    );

    host.input.replace_button_states([(
        RuntimeButton::Keyboard(0x53),
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    let player_after_bound_key = core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!(
        (player_after_bound_key.x, player_after_bound_key.y),
        (12.0, 24.0)
    );
}

#[test]
fn core_reset_clears_previous_movement_and_input_effects() {
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

    host.input.replace_button_states([(
        RuntimeButton::Restart,
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    host.input.clear_transitions();
    core.tick(&mut host).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!((player.previous_x, player.previous_y), (12.0, 24.0));
}

#[test]
fn core_restart_resets_jump_state_before_the_next_jump() {
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

    host.input.replace_button_states([(
        RuntimeButton::Restart,
        ButtonState {
            pressed: true,
            just_pressed: true,
            just_released: false,
        },
    )]);
    core.tick(&mut host).unwrap();

    let trace = capture_jump_trace(&core);
    assert!(!trace.jump_active);
    assert_eq!(trace.jump_hold_frames, 0);
    assert!(!trace.jump_cut_applied);
}

#[test]
fn core_spawn_adjusts_explicit_player_out_of_checkpoint_solid() {
    let mut package = sample_package();
    package.rooms[0].instances[0].is_checkpoint = false;
    package.rooms[0].instances[1].is_checkpoint = false;
    package.rooms[0].instances[2].is_checkpoint = true;

    let mut core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();

    assert_eq!(room.spawn_point, Some((12, 40)));
    assert_eq!((player.x, player.y), (12.0, 24.0));

    core.request_room_transition(7);
    core.tick(&mut host()).unwrap();
    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.player_candidate)
        .unwrap();
    assert_eq!((player.x, player.y), (12.0, 24.0));
}
