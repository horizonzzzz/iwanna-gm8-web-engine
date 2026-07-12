use iwm_runtime_host::RuntimeDrawCommand;
use iwm_runtime_model::{
    ObjectDefinition, ObjectEventEntry, RoomInstancePlacement, RuntimeDisplaySource,
};

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore};

#[cfg(feature = "local-sample-tests")]
use super::support::real_sample_package;
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
fn runtime_core_orders_instance_sprites_by_gm_depth() {
    let mut package = sample_package();
    package
        .objects
        .iter_mut()
        .find(|object| object.id == 0)
        .unwrap()
        .depth = -999_999_999;
    package
        .objects
        .iter_mut()
        .find(|object| object.id == 705)
        .unwrap()
        .depth = 100;
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.render(&mut host).unwrap();

    let sprite_ids = host
        .renderer
        .submitted_frames
        .last()
        .unwrap()
        .commands
        .iter()
        .filter_map(|command| match command {
            RuntimeDrawCommand::DrawSprite { sprite_id, .. } => Some(*sprite_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sprite_ids, vec![1, 0]);
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
fn runtime_core_uses_instance_image_alpha_when_rendering_sprites() {
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
        .insert("image_alpha".into(), crate::RuntimeValue::Number(0.7));

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 1,
            alpha,
            ..
        } if (*alpha - 0.7).abs() < f64::EPSILON
    )));
}

#[test]
fn runtime_core_uses_floored_image_index_when_rendering_sprites() {
    let mut package = sample_package();
    package.resources.sprites[1].frame_paths = vec![
        "resources/sprites/1-0.png".into(),
        "resources/sprites/1-1.png".into(),
        "resources/sprites/1-2.png".into(),
    ];
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
    marker
        .vars
        .insert("sprite_index".into(), crate::RuntimeValue::Number(1.0));
    marker
        .vars
        .insert("image_index".into(), crate::RuntimeValue::Number(1.9));

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 1,
            frame_index: 1,
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
fn runtime_core_uses_exe_display_size_for_room_without_active_view() {
    let mut package = sample_package();
    package.manifest.display_source = Some(RuntimeDisplaySource::ExeResolution);
    package.manifest.display_width = Some(640);
    package.manifest.display_height = Some(480);
    package.rooms[0].width = 320;
    package.rooms[0].height = 240;
    package.rooms[0].views_enabled = false;

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!((frame.width, frame.height), (640, 480));
}

#[test]
fn runtime_core_executes_draw_events_for_text_commands() {
    let mut package = sample_package();
    package.objects.push(ObjectDefinition {
        id: 8,
        name: "obj_label".into(),
        sprite_index: -1,
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
            event_type: 8,
            sub_event: 0,
            event_tag: "draw".into(),
            block_id: "object:8:event:8:0".into(),
            action_count: 0,
        }],
    });
    package.rooms[0].instances.push(RoomInstancePlacement {
        instance_id: 99,
        object_id: 8,
        x: 40,
        y: 50,
        xscale: 1.0,
        yscale: 1.0,
        angle: 0.0,
        blend: 0x00ff_ffff,
        creation_block_id: None,
        is_solid: false,
        is_hazard: false,
        is_checkpoint: false,
    });
    append_lowered_entry(
        &mut package,
        "object:8:event:8:0".into(),
        vec![
            LoweredLogicStatement::FunctionCall {
                name: "draw_set_color".into(),
                args: vec![LoweredLogicExpr::Identifier("c_red".into())],
            },
            LoweredLogicStatement::FunctionCall {
                name: "draw_set_halign".into(),
                args: vec![LoweredLogicExpr::Identifier("fa_center".into())],
            },
            LoweredLogicStatement::FunctionCall {
                name: "draw_set_font".into(),
                args: vec![LoweredLogicExpr::Identifier("font40".into())],
            },
            LoweredLogicStatement::FunctionCall {
                name: "draw_text".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("x".into()),
                    LoweredLogicExpr::Identifier("y".into()),
                    LoweredLogicExpr::LiteralText("Data1".into()),
                ],
            },
            LoweredLogicStatement::FunctionCall {
                name: "draw_sprite".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("spr_sparse".into()),
                    LoweredLogicExpr::LiteralNumber(0.0),
                    LoweredLogicExpr::LiteralNumber(64.0),
                    LoweredLogicExpr::LiteralNumber(72.0),
                ],
            },
        ],
    );

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawText {
            text,
            x: 40,
            y: 50,
            size: 40,
            colour,
            align,
            ..
        } if text == "Data1"
            && colour.r == 255
            && colour.g == 0
            && colour.b == 0
            && colour.a == 255
            && align == "center"
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id: 1,
            x: 64,
            y: 72,
            ..
        }
    )));
}

#[test]
#[cfg(feature = "local-sample-tests")]
fn real_sample_menu_draws_slot_text_and_cursor_sprite() {
    let Some(package) = real_sample_package() else {
        return;
    };
    let menu_room_id = package
        .rooms
        .iter()
        .find(|room| room.name.eq_ignore_ascii_case("rMenu"))
        .map(|room| room.id)
        .expect("sample package should include rMenu");
    let cursor_sprite_id = package
        .resources
        .sprites
        .iter()
        .find(|sprite| sprite.name.eq_ignore_ascii_case("sprPlayerRunning"))
        .map(|sprite| sprite.id)
        .expect("sample package should include sprPlayerRunning");
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.reload_room(menu_room_id).unwrap();
    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    let texts = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            RuntimeDrawCommand::DrawText { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(texts.contains(&"Data1"), "rMenu texts: {texts:?}");
    assert!(texts.contains(&"Data2"), "rMenu texts: {texts:?}");
    assert!(texts.contains(&"Data3"), "rMenu texts: {texts:?}");
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite {
            sprite_id,
            ..
        } if *sprite_id == cursor_sprite_id
    )));
}

#[test]
fn runtime_core_prefers_active_view_over_exe_display_size() {
    let mut package = sample_package();
    package.manifest.display_source = Some(RuntimeDisplaySource::ExeResolution);
    package.manifest.display_width = Some(640);
    package.manifest.display_height = Some(480);
    package.rooms[0].views_enabled = true;
    package.rooms[0].views[0].visible = true;
    package.rooms[0].views[0].port_w = 800;
    package.rooms[0].views[0].port_h = 600;

    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = host();

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!((frame.width, frame.height), (800, 600));
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
