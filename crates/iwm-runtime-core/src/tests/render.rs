use iwm_runtime_host::RuntimeDrawCommand;

use crate::RuntimeCore;

use super::support::{host, sample_package};

#[test]
fn runtime_core_emits_browser_consumable_draw_commands() {
    let package = sample_package();
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!(frame.room_id, Some(7));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawBackground {
            background_id: 0,
            ..
        }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawTile {
            background_id: 0,
            width: 32,
            height: 32,
            ..
        }
    )));
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, RuntimeDrawCommand::DrawSprite { sprite_id: 0, .. })));
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, RuntimeDrawCommand::DrawSprite { sprite_id: 1, .. })));
}

#[test]
fn runtime_core_mirrors_player_sprite_when_facing_left() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();
    let room = core.current_room.as_mut().unwrap();
    let player = room
        .instances
        .iter_mut()
        .find(|instance| instance.player_candidate)
        .unwrap();
    player.facing_left = true;

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 0,
            xscale,
            ..
        } if *xscale < 0.0
    )));
}

#[test]
fn runtime_core_renders_visible_view_as_canvas_frame() {
    let mut package = sample_package();
    package.rooms[0].width = 2400;
    package.rooms[0].height = 1824;
    package.rooms[0].views_enabled = true;
    package.rooms[0].views[0].visible = true;
    package.rooms[0].views[0].source_x = 800;
    package.rooms[0].views[0].source_y = 608;
    package.rooms[0].views[0].source_w = 800;
    package.rooms[0].views[0].source_h = 600;
    package.rooms[0].views[0].port_x = 0;
    package.rooms[0].views[0].port_y = 0;
    package.rooms[0].views[0].port_w = 800;
    package.rooms[0].views[0].port_h = 600;
    package.rooms[0].tiles[0].x = 832;
    package.rooms[0].tiles[0].y = 640;
    package.rooms[0].instances[0].x = 812;
    package.rooms[0].instances[0].y = 624;
    for instance in &mut package.rooms[0].instances {
        instance.is_checkpoint = false;
    }
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 99,
            object_id: 0,
            x: 64,
            y: 64,
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

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!(frame.width, 800);
    assert_eq!(frame.height, 600);
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, RuntimeDrawCommand::DrawTile { x: 32, y: 32, .. })));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 0,
            x: 12,
            y: 16,
            ..
        }
    )));
    assert!(!frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 0,
            x: -736,
            y: -544,
            ..
        }
    )));
}
