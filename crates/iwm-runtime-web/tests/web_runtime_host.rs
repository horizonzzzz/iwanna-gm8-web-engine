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
    assert_eq!(host.host_frame_count(), 1);
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
        Some((32.0, 64.0))
    );
    assert_eq!(
        boot.player.as_ref().map(|player| (
            player.jump.grounded,
            player.jump.active,
            player.jump.hold_frames,
            player.jump.cut_applied
        )),
        Some((true, false, 0, false))
    );

    host.set_input(WebInputState {
        left: false,
        right: true,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: false,
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
    });

    let after_tick = host.tick(1).unwrap();
    assert_eq!(after_tick.tick, 1);
    assert!(after_tick.player.as_ref().map(|player| player.x).unwrap() > 32.0);
    assert_eq!(
        after_tick.player.as_ref().map(|player| (
            player.jump.grounded,
            player.jump.active,
            player.jump.hold_frames,
            player.jump.cut_applied
        )),
        Some((true, false, 0, false))
    );

    let switched = host.select_room(1).unwrap();
    assert_eq!(switched.room_id, Some(1));
    assert_eq!(
        switched.player.as_ref().map(|player| (player.x, player.y)),
        Some((0.0, 0.0))
    );

    let reset = host.reset().unwrap();
    assert_eq!(reset.room_id, Some(0));
    assert_eq!(
        reset.player.as_ref().map(|player| (player.x, player.y)),
        Some((32.0, 64.0))
    );
    assert_eq!(
        reset.player.as_ref().map(|player| (
            player.jump.grounded,
            player.jump.active,
            player.jump.hold_frames,
            player.jump.cut_applied
        )),
        Some((true, false, 0, false))
    );
}

#[test]
fn web_runtime_host_snapshot_exposes_jump_trace_after_jump_press() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: false,
        right: false,
        jump: true,
        jump_pressed: true,
        jump_released: false,
        restart: false,
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
    });

    let after_tick = host.tick(1).unwrap();
    assert_eq!(
        after_tick.player.as_ref().map(|player| (
            player.jump.grounded,
            player.jump.active,
            player.jump.hold_frames,
            player.jump.cut_applied
        )),
        Some((false, true, 1, false))
    );
}

#[test]
fn web_runtime_host_accepts_raw_virtual_key_input() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: false,
        right: false,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: false,
        keys_held: vec![0x10],
        keys_pressed: vec![0x10],
        keys_released: vec![],
    });

    let snapshot = host.tick(1).unwrap();
    assert_eq!(snapshot.tick, 1);
}

#[test]
fn web_runtime_host_preserves_raw_key_edges_when_semantic_jump_is_false() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: false,
        right: false,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: false,
        keys_held: vec![0x20],
        keys_pressed: vec![0x20],
        keys_released: vec![],
    });

    let after_press = host.tick(1).unwrap();
    assert_eq!(
        after_press.player.as_ref().map(|player| (
            player.jump.grounded,
            player.jump.active,
            player.jump.hold_frames,
            player.jump.cut_applied
        )),
        Some((false, true, 1, false))
    );

    host.set_input(WebInputState {
        left: false,
        right: false,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: false,
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![0x20],
    });

    let after_release = host.tick(1).unwrap();
    assert!(after_release
        .player
        .as_ref()
        .map(|player| player.jump.cut_applied)
        .unwrap_or(false));
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
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
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
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
    });
    let reset = host.tick(1).unwrap();
    assert_eq!(
        reset.player.as_ref().map(|player| (player.x, player.y)),
        Some((32.0, 64.0))
    );

    host.set_input(WebInputState {
        left: false,
        right: true,
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: true,
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
    });
    let after_hold = host.tick(1).unwrap();
    assert!(after_hold.player.as_ref().map(|player| player.x).unwrap() > 32.0);
}

#[test]
fn web_runtime_host_clears_input_edge_bits_after_each_tick() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: false,
        right: true,
        jump: true,
        jump_pressed: true,
        jump_released: false,
        restart: true,
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
    });

    host.tick(1).unwrap();
    let after_first = host.snapshot().unwrap();
    assert_eq!(after_first.tick, 1);
    host.set_input(WebInputState {
        left: false,
        right: true,
        jump: true,
        jump_pressed: false,
        jump_released: false,
        restart: true,
        keys_held: vec![],
        keys_pressed: vec![],
        keys_released: vec![],
    });
    host.tick(1).unwrap();

    let after_second = host.snapshot().unwrap();
    assert_eq!(after_second.tick, 2);
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


