use iwm_runtime_web::{
    decode_web_input_state_from_buffer, encode_bridge_step_result_to_buffer, BridgeDrawCommand,
    BridgeFrameSnapshot, BridgeInputTraceSnapshot, BridgeRgba8, BridgeSnapshot, BridgeStepResult,
    BridgeTickPhaseSnapshot,
};

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_keys(bytes: &mut Vec<u8>, values: &[u16]) {
    push_u32(bytes, values.len() as u32);
    for value in values {
        push_u16(bytes, *value);
    }
}

#[test]
fn decode_web_input_state_from_buffer_reads_flags_and_key_edges() {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 0x424d5749);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, 0b0010_1101);
    push_u16(&mut bytes, 0);
    push_keys(&mut bytes, &[0x10, 0x5a]);
    push_keys(&mut bytes, &[0x10]);
    push_keys(&mut bytes, &[0x5a]);

    let input = decode_web_input_state_from_buffer(&bytes).unwrap();

    assert!(input.left);
    assert!(!input.right);
    assert!(input.jump);
    assert!(input.jump_pressed);
    assert!(!input.jump_released);
    assert!(input.restart);
    assert_eq!(input.keys_held, vec![0x10, 0x5a]);
    assert_eq!(input.keys_pressed, vec![0x10]);
    assert_eq!(input.keys_released, vec![0x5a]);
}

#[test]
fn encode_bridge_step_result_to_buffer_writes_header_snapshot_and_present_frame() {
    let step = BridgeStepResult {
        snapshot: BridgeSnapshot {
            status: "ready".into(),
            tick: 7,
            room_id: Some(3),
            room_name: Some("room3".into()),
            room_speed: Some(60),
            instance_count: 2,
            player: None,
            input_trace: BridgeInputTraceSnapshot {
                jump_button_key: 0x10,
                jump_pressed: true,
                jump_just_pressed: true,
                jump_just_released: false,
                active_keys: vec!["0x10:p1jp1jr0".into()],
            },
            tick_phases: BridgeTickPhaseSnapshot {
                input_diag_nanos: 1,
                step_events_nanos: 2,
                view_sync_nanos: 3,
                player_movement_nanos: 4,
                collision_events_nanos: 5,
                alarms_nanos: 6,
                keyboard_events_nanos: 7,
                render_submit_nanos: 8,
                total_nanos: 36,
            },
            diagnostics: vec!["runtime-idle:tick advanced".into()],
        },
        frame: BridgeFrameSnapshot {
            tick: 7,
            room_id: Some(3),
            width: 320,
            height: 240,
            commands: vec![
                BridgeDrawCommand::Clear {
                    colour: BridgeRgba8 {
                        r: 1,
                        g: 2,
                        b: 3,
                        a: 4,
                    },
                },
                BridgeDrawCommand::DrawBackground {
                    background_id: 44,
                    x: -10,
                    y: 20,
                    stretch: true,
                    tile_horz: false,
                    tile_vert: true,
                    is_foreground: false,
                },
                BridgeDrawCommand::DrawTile {
                    background_id: 55,
                    x: 32,
                    y: 64,
                    tile_x: 8,
                    tile_y: 16,
                    width: 32,
                    height: 32,
                    xscale: 1.5,
                    yscale: 0.5,
                },
                BridgeDrawCommand::DrawSprite {
                    sprite_id: 7,
                    frame_index: 0,
                    x: 96,
                    y: 128,
                    origin_x: 16,
                    origin_y: 24,
                    xscale: 1.0,
                    yscale: 1.0,
                    alpha: 0.5,
                    angle_degrees: 0.0,
                },
                BridgeDrawCommand::FillRect {
                    x: 1,
                    y: 2,
                    width: 3,
                    height: 4,
                    colour: BridgeRgba8 {
                        r: 250,
                        g: 251,
                        b: 252,
                        a: 253,
                    },
                },
                BridgeDrawCommand::DrawText {
                    text: "GAME OVER".into(),
                    x: 160,
                    y: 88,
                    size: 32,
                    font_name: Some("font32".into()),
                    font_bold: true,
                    font_italic: false,
                    colour: BridgeRgba8 {
                        r: 232,
                        g: 36,
                        b: 48,
                        a: 220,
                    },
                    align: "center".into(),
                },
                BridgeDrawCommand::Present,
            ],
        },
    };

    let bytes = encode_bridge_step_result_to_buffer(&step).unwrap();

    assert_eq!(&bytes[0..4], &0x424d5749u32.to_le_bytes());
    assert_eq!(&bytes[4..6], &1u16.to_le_bytes());
    assert_eq!(&bytes[6..8], &2u16.to_le_bytes());
    assert!(bytes.ends_with(&[6]));
}
