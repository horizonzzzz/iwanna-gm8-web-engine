use iwm_runtime_web::{BridgeDrawCommand, BridgeFrameSnapshot, BridgeRgba8};
use serde_json::json;

#[test]
fn bridge_frame_json_preserves_draw_command_wire_shape() {
    let value = serde_json::to_value(BridgeFrameSnapshot {
        tick: 1,
        room_id: Some(87),
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
    })
    .unwrap();

    assert_eq!(value.get("tick"), Some(&json!(1)));
    assert_eq!(value.get("roomId"), Some(&json!(87)));
    assert_eq!(value.get("width"), Some(&json!(320)));
    assert_eq!(value.get("height"), Some(&json!(240)));
    assert!(value.get("room_id").is_none());

    let commands = value
        .get("commands")
        .and_then(serde_json::Value::as_array)
        .expect("frame snapshot should encode commands as an array");

    assert_eq!(commands[0].get("kind"), Some(&json!("clear")));
    assert_eq!(commands[0].get("colour"), Some(&json!([1, 2, 3, 4])));

    assert_eq!(commands[1].get("kind"), Some(&json!("drawBackground")));
    assert_eq!(commands[1].get("backgroundId"), Some(&json!(44)));
    assert_eq!(commands[1].get("tileHorz"), Some(&json!(false)));
    assert_eq!(commands[1].get("tileVert"), Some(&json!(true)));
    assert_eq!(commands[1].get("isForeground"), Some(&json!(false)));
    assert!(commands[1].get("background_id").is_none());
    assert!(commands[1].get("tile_horz").is_none());

    assert_eq!(commands[2].get("kind"), Some(&json!("drawTile")));
    assert_eq!(commands[2].get("backgroundId"), Some(&json!(55)));
    assert_eq!(commands[2].get("tileX"), Some(&json!(8)));
    assert_eq!(commands[2].get("tileY"), Some(&json!(16)));
    assert_eq!(commands[2].get("xscale"), Some(&json!(1.5)));
    assert_eq!(commands[2].get("yscale"), Some(&json!(0.5)));
    assert!(commands[2].get("background_id").is_none());
    assert!(commands[2].get("tile_x").is_none());

    assert_eq!(commands[3].get("kind"), Some(&json!("drawSprite")));
    assert_eq!(commands[3].get("spriteId"), Some(&json!(7)));
    assert_eq!(commands[3].get("frameIndex"), Some(&json!(0)));
    assert_eq!(commands[3].get("originX"), Some(&json!(16)));
    assert_eq!(commands[3].get("originY"), Some(&json!(24)));
    assert_eq!(commands[3].get("alpha"), Some(&json!(0.5)));
    assert_eq!(commands[3].get("angleDegrees"), Some(&json!(0.0)));
    assert!(commands[3].get("sprite_id").is_none());
    assert!(commands[3].get("frame_index").is_none());
    assert!(commands[3].get("origin_x").is_none());
    assert!(commands[3].get("angle_degrees").is_none());

    assert_eq!(commands[4].get("kind"), Some(&json!("fillRect")));
    assert_eq!(
        commands[4].get("colour"),
        Some(&json!([250, 251, 252, 253]))
    );
    assert_eq!(commands[4].get("width"), Some(&json!(3)));
    assert_eq!(commands[4].get("height"), Some(&json!(4)));

    assert_eq!(commands[5].get("kind"), Some(&json!("drawText")));
    assert_eq!(commands[5].get("text"), Some(&json!("GAME OVER")));
    assert_eq!(commands[5].get("x"), Some(&json!(160)));
    assert_eq!(commands[5].get("y"), Some(&json!(88)));
    assert_eq!(commands[5].get("size"), Some(&json!(32)));
    assert_eq!(commands[5].get("align"), Some(&json!("center")));
    assert_eq!(commands[5].get("colour"), Some(&json!([232, 36, 48, 220])));

    assert_eq!(commands[6], json!({ "kind": "present" }));
}
