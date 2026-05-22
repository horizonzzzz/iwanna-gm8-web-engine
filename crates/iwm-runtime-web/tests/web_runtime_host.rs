mod support;

use iwm_runtime_model::ObjectDefinition;
use iwm_runtime_web::{BridgeDrawCommand, WebInputState, WebRuntimeHost};
use serde_json::json;

use support::sample_package;

#[test]
fn web_runtime_host_boots_and_ticks_headless_runtime() {
    let mut host = WebRuntimeHost::new();

    let boot = host.boot(sample_package()).unwrap();
    assert_eq!(boot.room_id, Some(0));
    assert_eq!(boot.status, "ready");
    assert_eq!(host.host_frame_count(), 1);

    let after_tick = host.tick(2).unwrap();
    assert_eq!(after_tick.tick, 2);
    assert_eq!(after_tick.room_id, Some(0));
    assert_eq!(host.host_frame_count(), 3);
}

#[test]
fn web_runtime_host_requires_boot_before_tick() {
    let mut host = WebRuntimeHost::new();
    let error = host.tick(1).unwrap_err();

    assert!(error.contains("not booted"));
}

#[test]
fn web_runtime_host_boots_from_json_payload() {
    let mut host = WebRuntimeHost::new();
    let package_json = serde_json::to_string(&sample_package()).unwrap();

    let boot = host.boot_from_json(&package_json).unwrap();

    assert_eq!(boot.tick, 0);
    assert_eq!(boot.room_id, Some(0));
    assert_eq!(boot.status, "ready");
}

#[test]
fn web_runtime_host_can_select_room_and_reset() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();
    host.tick(2).unwrap();

    let selected = host.select_room(1).unwrap();
    assert_eq!(selected.room_id, Some(1));
    assert_eq!(selected.room_name.as_deref(), Some("room1"));

    let reset = host.reset().unwrap();
    assert_eq!(reset.tick, 0);
    assert_eq!(reset.room_id, Some(0));
    assert_eq!(reset.room_name.as_deref(), Some("room0"));
}

#[test]
fn web_runtime_host_snapshot_exposes_player_motion_and_reset_state() {
    let mut host = WebRuntimeHost::new();
    let boot = host.boot(sample_package()).unwrap();
    assert_eq!(
        boot.player.as_ref().map(|player| (player.x, player.y)),
        Some((32, 64))
    );

    host.set_input(WebInputState {
        left: false,
        right: true,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: false,
    });

    let after_tick = host.tick(1).unwrap();
    assert_eq!(after_tick.tick, 1);
    assert!(after_tick.player.as_ref().map(|player| player.x).unwrap() > 32);

    let switched = host.select_room(1).unwrap();
    assert_eq!(switched.room_id, Some(1));
    assert_eq!(
        switched.player.as_ref().map(|player| (player.x, player.y)),
        Some((0, 0))
    );

    let reset = host.reset().unwrap();
    assert_eq!(reset.room_id, Some(0));
    assert_eq!(
        reset.player.as_ref().map(|player| (player.x, player.y)),
        Some((32, 64))
    );
}

#[test]
fn web_runtime_host_formats_diagnostics_for_bridge_consumers() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();
    host.tick(1).unwrap();

    let diagnostics = host.diagnostics();

    assert!(diagnostics.iter().any(|entry| entry.contains("runtime-idle")));
    assert!(json!(diagnostics).is_array());
}

#[test]
fn web_runtime_host_accepts_input_and_returns_render_frame_json() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: true,
        right: false,
        jump: true,
        jump_pressed: true,
        jump_released: false,
        restart: false,
    });

    host.tick(1).unwrap();
    let frame = host.frame_snapshot().unwrap();

    assert_eq!(frame.tick, 1);
    assert_eq!(frame.room_id, Some(0));
    assert!(!frame.commands.is_empty());
}

#[test]
fn web_runtime_host_treats_restart_as_a_one_shot_press_edge() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: false,
        right: false,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: true,
    });
    let reset = host.tick(1).unwrap();
    assert_eq!(
        reset.player.as_ref().map(|player| (player.x, player.y)),
        Some((32, 64))
    );

    host.set_input(WebInputState {
        left: false,
        right: true,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: true,
    });
    let after_hold = host.tick(1).unwrap();
    assert!(after_hold.player.as_ref().map(|player| player.x).unwrap() > 32);
}

#[test]
fn web_runtime_host_frame_snapshot_includes_tiles_and_fallback_player_output() {
    let mut package = sample_package();
    package.rooms[0].instances[0].object_id = 1;
    package.rooms[0].instances[0].is_checkpoint = true;
    package.objects.push(ObjectDefinition {
        id: 1,
        name: "checkpoint".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(true),
        is_player: false,
        events: vec![],
    });

    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();
    let frame = host.frame_snapshot().unwrap();

    assert!(frame.commands.iter().any(|command| matches!(
        command,
        BridgeDrawCommand::DrawTile {
            background_id: 0,
            width: 16,
            height: 16,
            ..
        }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        BridgeDrawCommand::DrawSprite { sprite_id: 0, .. }
    )));
}
