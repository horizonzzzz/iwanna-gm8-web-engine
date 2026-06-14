use iwm_runtime_host::RuntimeDrawCommand;
use iwm_runtime_model::{ObjectDefinition, ObjectEventEntry, RoomInstancePlacement};

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore};

use super::support::{add_step_block, append_lowered_entry, host, sample_package};

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
fn runtime_core_skips_dead_instances_when_rendering_sprites() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();
    let room = core.current_room.as_mut().unwrap();
    let sparse_sprite = room
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 705)
        .unwrap();
    sparse_sprite.alive = false;

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, RuntimeDrawCommand::DrawSprite { sprite_id: 1, .. })));
}

#[test]
fn runtime_core_uses_instance_visible_flag_when_rendering_sprites() {
    let mut package = sample_package();
    package.objects[1].sprite_index = 1;
    package.objects[1].visible = false;
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker.visible = true;

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 1,
            x: 48,
            y: 64,
            ..
        }
    )));
}

#[test]
fn runtime_core_uses_instance_sprite_and_image_index_when_rendering_sprites() {
    let mut core = RuntimeCore::load(sample_package()).unwrap();
    let mut host = host();
    let marker = core
        .current_room
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.object_id == 1)
        .unwrap();
    marker
        .vars
        .insert("sprite_index".into(), crate::RuntimeValue::Number(1.0));
    marker
        .vars
        .insert("image_index".into(), crate::RuntimeValue::Number(2.0));

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 1,
            frame_index: 2,
            ..
        }
    )));
}

#[test]
fn runtime_core_does_not_render_custom_death_feedback_after_direct_hazard_death() {
    let mut package = sample_package();
    package.rooms[0].instances.push(RoomInstancePlacement {
        instance_id: 99,
        object_id: 1,
        x: 12,
        y: 24,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: true,
        is_checkpoint: false,
    });
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(!frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::FillRect {
            colour,
            ..
        } if colour.r >= 160 && colour.g <= 40 && colour.b <= 40
    )));
    assert!(!frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawText {
            text,
            ..
        } if text == "GAME OVER"
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
            x,
            y,
            ..
        } if *x >= 0 && *x <= 32 && *y >= 0 && *y <= 32
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

#[test]
fn runtime_core_does_not_render_new_room_pre_step_transients_after_room_goto() {
    let mut package = sample_package();
    package.objects.push(ObjectDefinition {
        id: 8,
        name: "obj_transient".into(),
        sprite_index: 1,
        parent_index: -1,
        depth: 0,
        persistent: false,
        visible: true,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(false),
        is_player: false,
        events: vec![ObjectEventEntry {
            event_type: 3,
            sub_event: 0,
            event_tag: "step".into(),
            block_id: "object:8:event:3:0".into(),
            action_count: 0,
        }],
    });
    package.rooms[1].instances.push(RoomInstancePlacement {
        instance_id: 99,
        object_id: 8,
        x: 32,
        y: 32,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: false,
        is_checkpoint: false,
    });
    add_step_block(
        &mut package,
        vec![LoweredLogicStatement::FunctionCall {
            name: "room_goto".into(),
            args: vec![LoweredLogicExpr::LiteralNumber(9.0)],
        }],
    );
    append_lowered_entry(
        &mut package,
        "object:8:event:3:0".into(),
        vec![LoweredLogicStatement::FunctionCall {
            name: "instance_destroy".into(),
            args: vec![],
        }],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.tick(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!(frame.room_id, Some(9));
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, RuntimeDrawCommand::DrawSprite { sprite_id: 1, .. })));
}
