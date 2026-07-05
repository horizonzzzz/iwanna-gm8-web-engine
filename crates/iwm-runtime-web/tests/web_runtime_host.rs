mod support;

use std::path::Path;

use iwm_runtime_host::{RuntimeAudioHost, RuntimeSoundMode};
use iwm_runtime_web::{BridgeDrawCommand, WebAudioHost, WebInputState, WebRuntimeHost};
use serde_json::json;

use support::sample_package;

fn load_real_sample_package() -> Option<iwm_runtime_core::RuntimePackage> {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("runtime")
        .join("public")
        .join("packages")
        .join("sample");

    let manifest_path = package_root.join("manifest.json");
    if !manifest_path.exists() {
        return None;
    }

    let manifest = serde_json::from_slice(&std::fs::read(manifest_path).ok()?).ok()?;
    let rooms =
        serde_json::from_slice(&std::fs::read(package_root.join("rooms.json")).ok()?).ok()?;
    let objects =
        serde_json::from_slice(&std::fs::read(package_root.join("objects.json")).ok()?).ok()?;
    let scripts =
        serde_json::from_slice(&std::fs::read(package_root.join("scripts.ir.json")).ok()?).ok()?;
    let analysis =
        serde_json::from_slice(&std::fs::read(package_root.join("analysis.json")).ok()?).ok()?;
    let resources = serde_json::from_slice(
        &std::fs::read(package_root.join("resources").join("index.json")).ok()?,
    )
    .ok()?;
    let lowered_logic =
        serde_json::from_slice(&std::fs::read(package_root.join("logic.lowered.json")).ok()?)
            .ok()?;

    Some(iwm_runtime_core::RuntimePackage {
        manifest,
        rooms,
        objects,
        scripts,
        lowered_logic: Some(lowered_logic),
        resources,
        analysis,
    })
}

#[test]
fn web_runtime_host_boots_and_ticks_headless_runtime() {
    let mut host = WebRuntimeHost::new();

    let boot = host.boot(sample_package()).unwrap();
    assert_eq!(boot.room_id, Some(0));
    assert_eq!(serde_json::to_value(&boot).unwrap()["roomSpeed"], 60);
    assert_eq!(boot.status, "ready");
    assert_eq!(host.host_frame_count(), 1);

    let after_tick = host.tick(2).unwrap();
    assert_eq!(after_tick.tick, 2);
    assert_eq!(after_tick.room_id, Some(0));
    assert!(after_tick.tick_phases.step_events_nanos > 0);
    assert!(after_tick.tick_phases.render_submit_nanos > 0);
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
    assert_eq!(serde_json::to_value(&selected).unwrap()["roomSpeed"], 30);
    assert_eq!(selected.room_name.as_deref(), Some("room1"));

    let reset = host.reset().unwrap();
    assert_eq!(reset.tick, 0);
    assert_eq!(reset.room_id, Some(0));
    assert_eq!(serde_json::to_value(&reset).unwrap()["roomSpeed"], 60);
    assert_eq!(reset.room_name.as_deref(), Some("room0"));
}

#[test]
fn web_runtime_host_can_select_real_sample_room_directly() {
    let Some(package) = load_real_sample_package() else {
        return;
    };
    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();

    let selected = host.select_room(147).unwrap();

    assert_eq!(selected.room_id, Some(147));
    assert!(
        selected.instance_count > 0,
        "manual room selection should remain available for diagnostics"
    );
    assert!(
        selected
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.contains("runtime-unsupported")),
        "manual room selection should not introduce runtime diagnostics: {:?}",
        selected.diagnostics
    );
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
    assert!(switched.player.is_none());

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
        jump: false,
        jump_pressed: false,
        jump_released: false,
        restart: false,
        keys_held: vec![0x20],
        keys_pressed: vec![0x20],
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
fn web_runtime_host_step_returns_snapshot_and_frame_together() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    let step = host
        .step(WebInputState {
            left: false,
            right: true,
            jump: false,
            jump_pressed: false,
            jump_released: false,
            restart: false,
            keys_held: vec![],
            keys_pressed: vec![],
            keys_released: vec![],
        })
        .unwrap();

    assert_eq!(step.snapshot.tick, 1);
    assert_eq!(step.frame.tick, 1);
    assert_eq!(step.frame.width, 320);
    assert!(step
        .frame
        .commands
        .iter()
        .any(|command| matches!(command, BridgeDrawCommand::Present)));
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
fn web_runtime_host_does_not_map_semantic_jump_to_space_without_raw_space_input() {
    let mut package = sample_package();
    package.rooms[0].creation_block_id = Some("room:7:create".into());
    package.lowered_logic = Some(iwm_runtime_core::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![iwm_runtime_core::LoweredLogicEntry {
            block_id: "room:7:create".into(),
            statements: vec![iwm_runtime_core::LoweredLogicStatement::Assignment {
                target: iwm_runtime_core::LoweredLogicExpr::MemberAccess {
                    target: Box::new(iwm_runtime_core::LoweredLogicExpr::Identifier(
                        "global".into(),
                    )),
                    member: "jumpbutton".into(),
                },
                value: iwm_runtime_core::LoweredLogicExpr::LiteralNumber(0x10 as f64),
            }],
        }],
    });

    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();

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
        Some((true, false, 0, false))
    );
}

#[test]
fn web_runtime_host_omits_frame_refresh_diagnostics_for_bridge_consumers() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();
    host.tick(1).unwrap();

    let diagnostics = host.diagnostics();

    assert!(!diagnostics
        .iter()
        .any(|entry| entry.contains("runtime-idle")));
    assert!(!diagnostics
        .iter()
        .any(|entry| entry.contains("runtime-jump-input")));
    assert!(!diagnostics
        .iter()
        .any(|entry| entry.contains("runtime-exec-block-trace")));
    assert!(json!(diagnostics).is_array());
}

#[test]
fn web_runtime_host_snapshot_formats_runtime_unsupported_diagnostics() {
    let mut package = sample_package();
    package.lowered_logic = Some(iwm_runtime_core::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![iwm_runtime_core::LoweredLogicEntry {
            block_id: "object:0:event:3:0".into(),
            statements: vec![iwm_runtime_core::LoweredLogicStatement::FunctionCall {
                name: "instance_position".into(),
                args: vec![
                    iwm_runtime_core::LoweredLogicExpr::Identifier("x".into()),
                    iwm_runtime_core::LoweredLogicExpr::Identifier("y".into()),
                    iwm_runtime_core::LoweredLogicExpr::Identifier("obj_marker".into()),
                ],
            }],
        }],
    });
    package.objects[0]
        .events
        .push(iwm_runtime_model::ObjectEventEntry {
            event_type: 3,
            sub_event: 0,
            event_tag: "step".into(),
            block_id: "object:0:event:3:0".into(),
            action_count: 0,
        });
    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();

    let snapshot = host.tick(1).unwrap();

    assert!(snapshot.diagnostics.iter().any(|entry| {
        entry.contains("runtime-unsupported-function")
            && entry.contains("function=instance_position")
            && entry.contains("block_id=object:0:event:3:0")
            && entry.contains("event_tag=step")
    }));
}

#[test]
fn web_runtime_host_records_audio_events_from_lowered_sound_calls() {
    let mut package = sample_package();
    package.resources.sounds[0].id = 42;
    package.resources.sounds[0].name = "sndJump".into();
    package.lowered_logic = Some(iwm_runtime_core::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![iwm_runtime_core::LoweredLogicEntry {
            block_id: "object:0:event:3:0".into(),
            statements: vec![
                iwm_runtime_core::LoweredLogicStatement::FunctionCall {
                    name: "sound_play".into(),
                    args: vec![iwm_runtime_core::LoweredLogicExpr::Identifier(
                        "sndJump".into(),
                    )],
                },
                iwm_runtime_core::LoweredLogicStatement::FunctionCall {
                    name: "sound_loop".into(),
                    args: vec![iwm_runtime_core::LoweredLogicExpr::Identifier(
                        "sndJump".into(),
                    )],
                },
                iwm_runtime_core::LoweredLogicStatement::FunctionCall {
                    name: "sound_stop".into(),
                    args: vec![iwm_runtime_core::LoweredLogicExpr::Identifier(
                        "sndJump".into(),
                    )],
                },
            ],
        }],
    });
    package.objects[0]
        .events
        .push(iwm_runtime_model::ObjectEventEntry {
            event_type: 3,
            sub_event: 0,
            event_tag: "step".into(),
            block_id: "object:0:event:3:0".into(),
            action_count: 0,
        });

    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();

    assert_eq!(
        host.audio_events(),
        vec!["play:42:once", "play:42:loop", "stop:42"]
    );
}

#[test]
fn web_audio_host_records_stop_all_events() {
    let mut host = WebAudioHost::default();

    host.play_sound(7, RuntimeSoundMode::Loop).unwrap();
    host.stop_all_sounds().unwrap();

    assert_eq!(host.events(), &["play:7:loop", "stop-all"]);
}

#[test]
fn web_audio_host_reports_playing_state_for_native_tests() {
    let mut host = WebAudioHost::default();

    assert!(!host.is_sound_playing(7).unwrap());
    host.play_sound(7, RuntimeSoundMode::Loop).unwrap();
    assert!(host.is_sound_playing(7).unwrap());
    host.stop_sound(7).unwrap();
    assert!(!host.is_sound_playing(7).unwrap());
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
fn web_runtime_host_reemits_raw_press_after_release_cycle() {
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
    let first = host.tick(1).unwrap();
    assert_eq!(
        first.input_trace.active_keys,
        vec!["0x10:p1jp1jr0".to_string()]
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
        keys_released: vec![0x10],
    });
    host.tick(1).unwrap();

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
    let second = host.tick(1).unwrap();
    assert_eq!(
        second.input_trace.active_keys,
        vec!["0x10:p1jp1jr0".to_string()]
    );
}

#[test]
fn web_runtime_host_frame_snapshot_includes_tiles_and_explicit_player_output() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();
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
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, BridgeDrawCommand::DrawSprite { sprite_id: 0, .. })));
}

#[test]
fn real_sample_shift_jump_retriggers_after_landing_in_sampleroom01() {
    let Some(package) = load_real_sample_package() else {
        return;
    };

    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();

    let mut init_snapshot = host.snapshot().unwrap();
    for _ in 0..120 {
        if init_snapshot.input_trace.jump_button_key == 0x10 {
            break;
        }
        host.set_input(WebInputState {
            left: false,
            right: false,
            jump: false,
            jump_pressed: false,
            jump_released: false,
            restart: false,
            keys_held: vec![],
            keys_pressed: vec![],
            keys_released: vec![],
        });
        init_snapshot = host.tick(1).unwrap();
    }
    assert_eq!(
        init_snapshot.input_trace.jump_button_key,
        0x10,
        "sample boot path should initialize jump binding to Shift before room test, snapshot={init_snapshot:?}"
    );

    host.select_room(143).unwrap();

    let mut settled_start = host.snapshot().unwrap();
    for _ in 0..120 {
        if settled_start
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            break;
        }
        host.set_input(WebInputState {
            left: false,
            right: false,
            jump: false,
            jump_pressed: false,
            jump_released: false,
            restart: false,
            keys_held: vec![],
            keys_pressed: vec![],
            keys_released: vec![],
        });
        settled_start = host.tick(1).unwrap();
    }
    assert!(
        settled_start
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false),
        "player should settle to grounded before jump test, snapshot={settled_start:?}"
    );

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
    let first_jump = host.tick(1).unwrap();
    assert!(
        !first_jump
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(true),
        "first jump should leave ground, snapshot={first_jump:?}"
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
        keys_released: vec![0x10],
    });
    host.tick(1).unwrap();

    let mut landed = false;
    let mut last_player = None;
    for _ in 0..180 {
        host.set_input(WebInputState {
            left: false,
            right: false,
            jump: false,
            jump_pressed: false,
            jump_released: false,
            restart: false,
            keys_held: vec![],
            keys_pressed: vec![],
            keys_released: vec![],
        });
        let snapshot = host.tick(1).unwrap();
        last_player = snapshot.player.clone();
        if snapshot
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(false)
        {
            landed = true;
            break;
        }
    }
    assert!(
        landed,
        "player should land again within 180 ticks, last_player={last_player:?}"
    );

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
    let second_jump = host.tick(1).unwrap();

    assert!(
        !second_jump
            .player
            .as_ref()
            .map(|player| player.jump.grounded)
            .unwrap_or(true),
        "second jump should leave ground again, last_player={last_player:?}, second_jump={second_jump:?}"
    );
}

#[test]
fn real_sample_manual_room_select_does_not_render_bow_in_sampleroom03() {
    let Some(package) = load_real_sample_package() else {
        return;
    };

    let mut host = WebRuntimeHost::new();
    host.boot(package).unwrap();

    let snapshot = host.select_room(146).unwrap();
    assert_eq!(snapshot.room_id, Some(146));
    assert_eq!(snapshot.room_name.as_deref(), Some("sampleroom03"));

    let frame = host.frame_snapshot().unwrap();
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, BridgeDrawCommand::DrawSprite { sprite_id: 25, .. })));
}
