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
fn runtime_core_culls_tiles_and_sprites_outside_active_view() {
    let mut package = sample_package();
    package.rooms[0].views_enabled = true;
    package.rooms[0].views[0].visible = true;
    package.rooms[0].views[0].source_x = 0;
    package.rooms[0].views[0].source_y = 0;
    package.rooms[0].views[0].source_w = 160;
    package.rooms[0].views[0].source_h = 160;
    package.rooms[0].tiles.push(iwm_runtime_model::RoomTilePlacement {
        tile_id: 22,
        source_bg: 0,
        x: 200,
        y: 200,
        tile_x: 0,
        tile_y: 0,
        width: 32,
        height: 32,
        depth: 100,
        xscale: 1.0,
        yscale: 1.0,
        blend: 0x00ff_ffff,
    });
    package.rooms[0].instances.push(iwm_runtime_model::RoomInstancePlacement {
        instance_id: 99,
        object_id: 0,
        x: 200,
        y: 200,
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
    let tile_count = frame
        .commands
        .iter()
        .filter(|command| matches!(command, RuntimeDrawCommand::DrawTile { .. }))
        .count();
    let sprite_count = frame
        .commands
        .iter()
        .filter(|command| matches!(command, RuntimeDrawCommand::DrawSprite { .. }))
        .count();

    assert_eq!(tile_count, 1);
    assert_eq!(sprite_count, 2);
}
