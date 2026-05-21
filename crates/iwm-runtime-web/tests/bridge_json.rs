use iwm_runtime_web::{BridgeDrawCommand, BridgeFrameSnapshot};
use serde_json::json;

#[test]
fn bridge_draw_command_json_uses_camel_case_fields() {
    let value = serde_json::to_value(BridgeFrameSnapshot {
        tick: 1,
        room_id: Some(87),
        width: 320,
        height: 240,
        commands: vec![
            BridgeDrawCommand::DrawTile {
                background_id: 55,
                x: 32,
                y: 64,
                tile_x: 0,
                tile_y: 0,
                width: 32,
                height: 32,
                xscale: 1.0,
                yscale: 1.0,
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
                angle_degrees: 0.0,
            },
        ],
    })
    .unwrap();

    let commands = value
        .get("commands")
        .and_then(serde_json::Value::as_array)
        .expect("frame snapshot should encode commands as an array");

    assert_eq!(commands[0].get("kind"), Some(&json!("drawTile")));
    assert_eq!(commands[0].get("backgroundId"), Some(&json!(55)));
    assert_eq!(commands[0].get("tileX"), Some(&json!(0)));
    assert_eq!(commands[0].get("tileY"), Some(&json!(0)));
    assert!(commands[0].get("background_id").is_none());
    assert!(commands[0].get("tile_x").is_none());

    assert_eq!(commands[1].get("kind"), Some(&json!("drawSprite")));
    assert_eq!(commands[1].get("spriteId"), Some(&json!(7)));
    assert_eq!(commands[1].get("frameIndex"), Some(&json!(0)));
    assert_eq!(commands[1].get("originX"), Some(&json!(16)));
    assert_eq!(commands[1].get("originY"), Some(&json!(24)));
    assert_eq!(commands[1].get("angleDegrees"), Some(&json!(0.0)));
    assert!(commands[1].get("sprite_id").is_none());
    assert!(commands[1].get("frame_index").is_none());
    assert!(commands[1].get("origin_x").is_none());
    assert!(commands[1].get("angle_degrees").is_none());
}
