use iwm_runtime_model::ObjectEventEntry;

#[test]
fn object_event_entry_includes_normalized_event_tag() {
    // Event entries should include a human-readable event_tag for runtime dispatch
    let event = ObjectEventEntry {
        event_type: 3, // Step event
        sub_event: 0,  // Step normal
        event_tag: "step".to_string(),
        block_id: "object:0:event:3:0".to_string(),
        action_count: 2,
    };

    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_tag"], "step");
    assert_eq!(json["event_type"], 3);
    assert_eq!(json["sub_event"], 0);
}

#[test]
fn object_event_entry_event_tags_for_all_supported_event_types() {
    // All GM8 event types should have normalized tags
    let test_cases = vec![
        (0, 0, "create"),
        (1, 0, "destroy"),
        (2, 0, "alarm:0"),
        (2, 5, "alarm:5"),
        (3, 0, "step"),
        (3, 1, "step:begin"),
        (3, 2, "step:end"),
        (4, 0, "collision"),   // collision target is dynamic, tag is generic
        (5, 65, "keyboard:a"), // ASCII key code
        (6, 0, "mouse:left"),
        (7, 0, "other:outside"),
        (7, 1, "other:boundary"),
        (8, 0, "draw"),
        (9, 65, "keypress:a"),
        (10, 65, "keyrelease:a"),
    ];

    for (event_type, sub_event, expected_tag) in test_cases {
        let event = ObjectEventEntry {
            event_type,
            sub_event,
            event_tag: expected_tag.to_string(),
            block_id: format!("object:0:event:{event_type}:{sub_event}"),
            action_count: 0,
        };

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json["event_tag"], expected_tag,
            "event_type={}, sub_event={} should have tag '{}'",
            event_type, sub_event, expected_tag
        );
    }
}

#[test]
fn logic_and_raw_exports_share_normalized_event_tags_for_gm8_event_ids() {
    use gm8exe::{
        asset::{object::Object, room::Room, CodeAction},
        settings::{GameHelpDialog, Settings},
        GameAssets, GameVersion,
    };
    use iwm_parser::logic_export::export_rooms_and_logic;
    use iwm_parser::raw_logic_export::export_raw_logic;

    fn sample_assets_for_event(event_type: usize, sub_event: u32) -> GameAssets {
        let mut events: Vec<Vec<(u32, Vec<CodeAction>)>> = (0..12).map(|_| Vec::new()).collect();
        events[event_type].push((sub_event, Vec::new()));

        GameAssets {
            triggers: vec![],
            constants: vec![],
            extensions: vec![],
            sprites: vec![],
            sounds: vec![],
            backgrounds: vec![],
            paths: vec![],
            scripts: vec![],
            fonts: vec![],
            timelines: vec![],
            objects: vec![Some(Box::new(Object {
                name: "obj_event".into(),
                sprite_index: -1,
                solid: false,
                visible: true,
                depth: 0,
                persistent: false,
                parent_index: -1,
                mask_index: -1,
                events,
            }))],
            rooms: vec![Some(Box::new(Room {
                name: "rm_event".into(),
                caption: "".into(),
                width: 320,
                height: 240,
                speed: 30,
                persistent: false,
                bg_colour: 0u32.into(),
                clear_screen: true,
                clear_region: true,
                creation_code: "".into(),
                backgrounds: vec![],
                views_enabled: false,
                views: vec![],
                instances: vec![],
                tiles: vec![],
                uses_810_features: false,
                uses_811_features: false,
            }))],
            included_files: vec![],
            version: GameVersion::GameMaker8_0,
            dx_dll: vec![],
            ico_file_raw: None,
            help_dialog: GameHelpDialog {
                bg_colour: 0u32.into(),
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

    let cases = [
        (0, 0, "create", None),
        (2, 7, "alarm:7", None),
        (3, 0, "step", None),
        (3, 1, "step:begin", None),
        (3, 2, "step:end", None),
        (4, 7, "collision", Some(7)),
        (5, 65, "keyboard:a", None),
        (9, 65, "keypress:a", None),
        (10, 65, "keyrelease:a", None),
    ];

    for (event_type, sub_event, expected_tag, expected_collision_id) in cases {
        let assets = sample_assets_for_event(event_type, sub_event);
        let (room_defs, object_defs, _) =
            export_rooms_and_logic(&assets.rooms, &assets.objects, &assets.scripts);
        assert!(
            room_defs.is_empty()
                || room_defs
                    .iter()
                    .all(|room| room.transition_targets.is_empty())
        );

        let logic_event = &object_defs[0].events[0];
        assert_eq!(logic_event.event_tag, expected_tag);
        assert_eq!(logic_event.sub_event, sub_event);

        let raw_logic = export_raw_logic(&assets);
        let raw_event = &raw_logic.object_events[0];
        assert_eq!(raw_event.event_tag, expected_tag);
        assert_eq!(raw_event.sub_event, sub_event);
        assert_eq!(raw_event.collision_object_id, expected_collision_id);
    }
}

#[test]
fn export_rooms_and_logic_uses_readable_keyboard_event_tags() {
    use gm8exe::{
        asset::{object::Object, room::Room},
        AssetList,
    };
    use iwm_parser::logic_export::export_rooms_and_logic;

    let mut events: Vec<Vec<(u32, Vec<gm8exe::asset::CodeAction>)>> =
        (0..12).map(|_| Vec::new()).collect();
    events[5].push((65, Vec::new()));

    let objects: AssetList<Object> = vec![Some(Box::new(Object {
        name: "obj_keyboard".into(),
        sprite_index: -1,
        solid: false,
        visible: true,
        depth: 0,
        persistent: false,
        parent_index: -1,
        mask_index: -1,
        events,
    }))];
    let rooms: AssetList<Room> = Vec::new();

    let empty_scripts = Vec::new();
    let (_, object_defs, _) = export_rooms_and_logic(&rooms, &objects, &empty_scripts);

    assert_eq!(object_defs[0].events[0].event_tag, "keyboard:a");
}
