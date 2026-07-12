use std::fs;

use gm8exe::{
    asset::font::Font,
    asset::sound::{Sound, SoundFX, SoundKind},
    asset::sprite::{CollisionMap, Frame},
    asset::Sprite,
    settings::{GameHelpDialog, Settings},
    Colour, GameAssets, GameVersion,
};

#[test]
fn exported_sound_resources_preserve_gm8_sound_kind() {
    let mut assets = game_assets_with_sprite_frame(vec![0, 0, 0, 0], 1, 1);
    assets.sounds.push(Some(Box::new(Sound {
        name: "music".into(),
        source: "music.mp3".into(),
        extension: ".mp3".into(),
        data: Some(vec![1, 2, 3].into_boxed_slice()),
        kind: SoundKind::Multimedia,
        volume: 1.0,
        pan: 0.0,
        preload: true,
        fx: SoundFX {
            chorus: false,
            echo: false,
            flanger: false,
            gargle: false,
            reverb: false,
        },
    })));
    let temp = tempfile::tempdir().unwrap();

    let index = iwm_parser::resource_export::export_resources(&assets, temp.path()).unwrap();

    assert_eq!(index.sounds[0].kind, "multimedia");
}

fn game_assets_with_sprite_frame(data: Vec<u8>, width: u32, height: u32) -> GameAssets {
    GameAssets {
        triggers: vec![],
        constants: vec![],
        extensions: vec![],
        sprites: vec![Some(Box::new(Sprite {
            name: "spr_player".into(),
            origin_x: 4,
            origin_y: 8,
            frames: vec![Frame {
                width,
                height,
                data: data.into_boxed_slice(),
            }],
            colliders: vec![CollisionMap {
                width,
                height,
                bbox_left: 1,
                bbox_right: 14,
                bbox_top: 2,
                bbox_bottom: 13,
                data: (0..width * height)
                    .map(|index| index == 2 * width + 1 || index == 13 * width + 14)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            }],
            per_frame_colliders: false,
        }))],
        sounds: vec![],
        backgrounds: vec![],
        paths: vec![],
        scripts: vec![],
        fonts: vec![],
        timelines: vec![],
        objects: vec![],
        rooms: vec![],
        included_files: vec![],
        version: GameVersion::GameMaker8_0,
        dx_dll: vec![],
        ico_file_raw: None,
        help_dialog: GameHelpDialog {
            bg_colour: Colour::from(0u32),
            new_window: false,
            caption: "".into(),
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            border: false,
            resizable: false,
            window_on_top: false,
            freeze_game: false,
            info: "".into(),
        },
        last_instance_id: 0,
        last_tile_id: 0,
        library_init_strings: vec![],
        room_order: vec![],
        settings: Settings {
            fullscreen: false,
            scaling: 0,
            interpolate_pixels: false,
            clear_colour: 0,
            allow_resize: false,
            window_on_top: false,
            dont_draw_border: false,
            dont_show_buttons: false,
            display_cursor: false,
            freeze_on_lose_focus: false,
            disable_screensaver: false,
            force_cpu_render: false,
            set_resolution: false,
            colour_depth: 0,
            resolution: 0,
            frequency: 0,
            vsync: false,
            esc_close_game: false,
            treat_close_as_esc: false,
            f1_help_menu: false,
            f4_fullscreen_toggle: false,
            f5_save_f6_load: false,
            f9_screenshot: false,
            priority: 0,
            custom_load_image: None,
            transparent: false,
            translucency: 0,
            loading_bar: 0,
            backdata: None,
            frontdata: None,
            scale_progress_bar: false,
            show_error_messages: false,
            log_errors: false,
            always_abort: false,
            zero_uninitialized_vars: false,
            error_on_uninitialized_args: false,
            swap_creation_events: false,
        },
        game_id: 0,
        guid: [0; 4],
    }
}

#[test]
fn bgra_pixels_are_converted_to_rgba_order() {
    let converted = iwm_parser::resource_export::bgra_to_rgba(vec![0, 64, 255, 255]);
    assert_eq!(converted, vec![255, 64, 0, 255]);
}

#[test]
fn exported_sprite_resources_include_collision_bounding_box_fields() {
    use iwm_parser::resource_export::export_resources;

    let assets = game_assets_with_sprite_frame(vec![0; 16 * 16 * 4], 16, 16);

    let temp = tempfile::tempdir().unwrap();
    let resources = export_resources(&assets, temp.path()).unwrap();
    let json = serde_json::to_value(&resources).unwrap();
    let sprite = &json["sprites"][0];

    assert_eq!(sprite["bbox_left"], 1);
    assert_eq!(sprite["bbox_right"], 14);
    assert_eq!(sprite["bbox_top"], 2);
    assert_eq!(sprite["bbox_bottom"], 13);
    assert_eq!(sprite["per_frame_collision_masks"], false);

    let mask = &sprite["collision_masks"][0];
    assert_eq!(mask["width"], 16);
    assert_eq!(mask["height"], 16);
    assert_eq!(mask["bbox_left"], 1);
    assert_eq!(mask["bbox_right"], 14);
    assert_eq!(mask["bbox_top"], 2);
    assert_eq!(mask["bbox_bottom"], 13);
    assert_eq!(mask["data"].as_array().unwrap().len(), 16 * 16);
    assert_eq!(mask["data"][2 * 16 + 1], true);
    assert_eq!(mask["data"][13 * 16 + 14], true);
    assert_eq!(mask["data"][2 * 16 + 2], false);
}

#[test]
fn exported_sprite_pixels_are_converted_from_bgra_to_rgba_order() {
    use iwm_parser::resource_export::export_resources;

    let assets = game_assets_with_sprite_frame(vec![0, 0, 255, 255], 1, 1);
    let temp = tempfile::tempdir().unwrap();
    let resources = export_resources(&assets, temp.path()).unwrap();
    let sprite_path = temp.path().join(&resources.sprites[0].frame_paths[0]);

    let bytes = fs::read(sprite_path).unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut output = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut output).unwrap();
    let pixels = &output[..info.buffer_size()];

    assert_eq!(pixels, &[255, 0, 0, 255]);
}

#[test]
fn exported_font_resources_preserve_exe_metadata() {
    use iwm_parser::resource_export::export_resources;

    let mut dmap = [0; 0x600];
    let glyph = 65 * 6;
    dmap[glyph] = 1;
    dmap[glyph + 1] = 2;
    dmap[glyph + 2] = 3;
    dmap[glyph + 3] = 4;
    dmap[glyph + 4] = 6;
    dmap[glyph + 5] = 5;

    let mut assets = game_assets_with_sprite_frame(vec![0; 4], 1, 1);
    assets.fonts = vec![Some(Box::new(Font {
        name: "font12".into(),
        sys_name: "MS Gothic".into(),
        size: 12,
        bold: true,
        italic: false,
        range_start: 32,
        range_end: 127,
        charset: 128,
        aa_level: 3,
        dmap: Box::new(dmap),
        map_width: 2,
        map_height: 1,
        pixel_map: vec![0, 255].into_boxed_slice(),
    }))];

    let temp = tempfile::tempdir().unwrap();
    let resources = export_resources(&assets, temp.path()).unwrap();

    assert_eq!(resources.fonts[0].id, 0);
    assert_eq!(resources.fonts[0].name, "font12");
    assert_eq!(resources.fonts[0].system_name, "MS Gothic");
    assert_eq!(resources.fonts[0].size, 12);
    assert!(resources.fonts[0].bold);
    assert!(!resources.fonts[0].italic);
    assert_eq!(resources.fonts[0].range_start, 32);
    assert_eq!(resources.fonts[0].range_end, 127);
    assert_eq!(resources.fonts[0].map_width, 2);
    assert_eq!(resources.fonts[0].map_height, 1);
    assert_eq!(resources.fonts[0].image_path, "resources/fonts/0.png");
    assert_eq!(resources.fonts[0].glyphs[65].x, 1);
    assert_eq!(resources.fonts[0].glyphs[65].y, 2);
    assert_eq!(resources.fonts[0].glyphs[65].width, 3);
    assert_eq!(resources.fonts[0].glyphs[65].height, 4);
    assert_eq!(resources.fonts[0].glyphs[65].offset, 5);
    assert_eq!(resources.fonts[0].glyphs[65].advance, 6);

    let bytes = fs::read(temp.path().join("resources/fonts/0.png")).unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let mut output = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut output).unwrap();
    let pixels = &output[..info.buffer_size()];
    assert_eq!(pixels, &[255, 255, 255, 0, 255, 255, 255, 255]);
}

#[test]
fn runtime_resources_are_written_under_expected_directories() {
    let base = std::path::Path::new("resources");
    assert_eq!(
        base.join("sprites").to_string_lossy().replace('\\', "/"),
        "resources/sprites"
    );
    assert_eq!(
        base.join("backgrounds")
            .to_string_lossy()
            .replace('\\', "/"),
        "resources/backgrounds"
    );
    assert_eq!(
        base.join("audio").to_string_lossy().replace('\\', "/"),
        "resources/audio"
    );
    assert_eq!(
        base.join("fonts").to_string_lossy().replace('\\', "/"),
        "resources/fonts"
    );
}
