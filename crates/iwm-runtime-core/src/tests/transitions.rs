use iwm_runtime_host::{ButtonState, RuntimeButton};

use crate::{RuntimeCore, RuntimeStatus};

use super::support::{host, sample_package};

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
        RuntimeButton::Keyboard(0x52),
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
    assert_eq!((player.x, player.y), (12, 24));
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
