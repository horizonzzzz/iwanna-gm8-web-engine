use iwm_runtime_model::{
    read_runtime_package_dir, validate_runtime_package, AnalysisReport, CompatibilityLevel,
    LogicBlock, LoweredLogicEntry, LoweredLogicFile, ObjectDefinition, ObjectEventEntry,
    RawCodeAction, RawLogicEventBinding, RawLogicFile, RawLogicOwner, RawLogicOwnerKind,
    ResourceIndex, RoomBackgroundLayer, RoomDefinition, RoomInstancePlacement, RoomTilePlacement,
    RuntimeDisplaySource, RuntimeManifest, RuntimePackageContract, RuntimePackageValidationError,
    ScriptIrFile, SpriteResource,
};

fn valid_sparse_package() -> RuntimePackageContract {
    RuntimePackageContract {
        manifest: RuntimeManifest {
            format_version: 1,
            package_kind: "runtime-v1".into(),
            source_name: "synthetic".into(),
            source_hash: "synthetic-hash".into(),
            engine_family: "gm8".into(),
            compatibility: CompatibilityLevel::Partial,
            default_room_id: Some(300),
            room_order: vec![300],
            room_count: 1,
            object_count: 1,
            script_block_count: 3,
            sprite_count: 1,
            background_count: 1,
            sound_count: 0,
            resource_index_path: "resources/index.json".into(),
            warnings: vec![],
            display_source: None,
            display_width: None,
            display_height: None,
            zero_uninitialized_vars: false,
        },
        rooms: vec![RoomDefinition {
            id: 300,
            name: "room_sparse".into(),
            width: 640,
            height: 480,
            speed: 50,
            persistent: false,
            background_colour: 0,
            clear_screen: true,
            backgrounds: vec![RoomBackgroundLayer {
                visible_on_start: true,
                is_foreground: false,
                source_bg: 900,
                xoffset: 0,
                yoffset: 0,
                tile_horz: false,
                tile_vert: false,
                hspeed: 0,
                vspeed: 0,
                stretch: false,
            }],
            views_enabled: false,
            views: vec![],
            tiles: vec![RoomTilePlacement {
                tile_id: 1,
                source_bg: 900,
                x: 0,
                y: 0,
                tile_x: 0,
                tile_y: 0,
                width: 16,
                height: 16,
                depth: 100,
                xscale: 1.0,
                yscale: 1.0,
                blend: 0x00ff_ffff,
            }],
            instances: vec![RoomInstancePlacement {
                instance_id: 1234,
                object_id: 705,
                x: 32,
                y: 64,
                xscale: 1.0,
                yscale: 1.0,
                angle: 0.0,
                blend: 0x00ff_ffff,
                creation_block_id: Some("room:300:instance:1234:create".into()),
                is_solid: false,
                is_hazard: false,
                is_checkpoint: false,
            }],
            creation_block_id: Some("room:300:create".into()),
            playable: true,
            transition_targets: vec![],
        }],
        objects: vec![ObjectDefinition {
            id: 705,
            name: "obj_sparse_player".into(),
            sprite_index: 42,
            parent_index: -1,
            depth: 0,
            persistent: false,
            visible: true,
            solid: false,
            mask_index: -1,
            is_hazard: Some(false),
            is_checkpoint: Some(false),
            is_player: true,
            events: vec![ObjectEventEntry {
                event_type: 3,
                sub_event: 0,
                event_tag: "step".into(),
                block_id: "object:705:event:3:0".into(),
                action_count: 1,
            }],
        }],
        scripts: ScriptIrFile {
            format: "iwm-script-ir-v1".into(),
            blocks: vec![
                LogicBlock {
                    id: "room:300:create".into(),
                    name: "room_sparse creation".into(),
                    kind: "room-creation".into(),
                    support: "source-only".into(),
                    executable_action_count: 0,
                    ops: vec![],
                },
                LogicBlock {
                    id: "room:300:instance:1234:create".into(),
                    name: "room_sparse instance 1234 creation".into(),
                    kind: "instance-creation".into(),
                    support: "source-only".into(),
                    executable_action_count: 0,
                    ops: vec![],
                },
                LogicBlock {
                    id: "object:705:event:3:0".into(),
                    name: "obj_sparse_player step".into(),
                    kind: "object-event".into(),
                    support: "source-only".into(),
                    executable_action_count: 0,
                    ops: vec![],
                },
            ],
        },
        raw_logic: RawLogicFile {
            format: "iwm-raw-logic-v1".into(),
            room_creation_codes: vec![RawLogicOwner {
                owner_kind: RawLogicOwnerKind::Room,
                owner_id: 300,
                owner_name: "room_sparse".into(),
                event_type: None,
                sub_event: None,
                collision_object_id: None,
                block_id: "room:300:create".into(),
                gml_source: "global.ready = true;".into(),
            }],
            instance_creation_codes: vec![RawLogicOwner {
                owner_kind: RawLogicOwnerKind::RoomInstance,
                owner_id: 1234,
                owner_name: "705".into(),
                event_type: None,
                sub_event: None,
                collision_object_id: None,
                block_id: "room:300:instance:1234:create".into(),
                gml_source: "image_speed = 0;".into(),
            }],
            object_events: vec![RawLogicEventBinding {
                object_id: 705,
                object_name: "obj_sparse_player".into(),
                event_type: 3,
                sub_event: 0,
                event_tag: "step".into(),
                collision_object_id: None,
                block_id: "object:705:event:3:0".into(),
                actions: vec![RawCodeAction {
                    action_id: 603,
                    lib_id: 1,
                    action_kind: 7,
                    execution_type: 2,
                    applies_to: -1,
                    is_condition: false,
                    invert_condition: false,
                    is_relative: false,
                    fn_name: "execute_code".into(),
                    fn_code: "x += 1;".into(),
                    args: vec![],
                }],
            }],
            scripts: vec![],
            triggers: vec![],
            timelines: vec![],
        },
        lowered_logic: LoweredLogicFile {
            format: "iwm-lowered-logic-v1".into(),
            entries: vec![
                LoweredLogicEntry {
                    block_id: "room:300:create".into(),
                    statements: vec![],
                },
                LoweredLogicEntry {
                    block_id: "room:300:instance:1234:create".into(),
                    statements: vec![],
                },
                LoweredLogicEntry {
                    block_id: "object:705:event:3:0".into(),
                    statements: vec![],
                },
            ],
        },
        analysis: AnalysisReport {
            dlls: vec![],
            included_files: vec![],
            warnings: vec![],
            unsupported_features: vec![],
        },
        resources: ResourceIndex {
            sprites: vec![SpriteResource {
                id: 42,
                name: "spr_sparse_player".into(),
                origin_x: 0,
                origin_y: 0,
                frame_paths: vec!["resources/sprites/spr_sparse_player/0.png".into()],
                width: 16,
                height: 16,
                bbox_left: 0,
                bbox_right: 15,
                bbox_top: 0,
                bbox_bottom: 15,
                collision_masks: vec![],
                per_frame_collision_masks: false,
            }],
            backgrounds: vec![iwm_runtime_model::BackgroundResource {
                id: 900,
                name: "bg_sparse".into(),
                width: 16,
                height: 16,
                image_path: "resources/backgrounds/bg_sparse.png".into(),
            }],
            sounds: vec![],
            fonts: vec![],
            paths: vec![],
        },
    }
}

#[test]
fn accepts_valid_sparse_identity_references() {
    let report = validate_runtime_package(&valid_sparse_package());

    assert!(report.valid, "expected no errors, got {report:?}");
    assert!(report.errors.is_empty());
}

#[test]
fn accepts_hidden_room_background_layers_that_reference_missing_resources() {
    let mut package = valid_sparse_package();
    package.rooms[0].backgrounds[0].visible_on_start = false;
    package.rooms[0].backgrounds[0].source_bg = 901;

    let report = validate_runtime_package(&package);

    assert!(
        report.valid,
        "expected hidden layer to be ignored, got {report:?}"
    );
    assert!(report.errors.is_empty());
}

#[test]
fn rejects_manifest_display_width_without_height() {
    let mut package = valid_sparse_package();
    package.manifest.display_source = Some(RuntimeDisplaySource::ExeResolution);
    package.manifest.display_width = Some(640);

    let report = validate_runtime_package(&package);

    assert!(!report.valid);
    assert_eq!(
        report.errors,
        vec![RuntimePackageValidationError::IncompleteManifestDisplay {
            display_source: true,
            display_width: true,
            display_height: false,
        }]
    );
}

#[test]
fn rejects_manifest_display_size_without_source() {
    let mut package = valid_sparse_package();
    package.manifest.display_width = Some(640);
    package.manifest.display_height = Some(480);

    let report = validate_runtime_package(&package);

    assert!(!report.valid);
    assert_eq!(
        report.errors,
        vec![RuntimePackageValidationError::IncompleteManifestDisplay {
            display_source: false,
            display_width: true,
            display_height: true,
        }]
    );
}

#[test]
fn rejects_room_order_entries_that_reference_missing_rooms() {
    let mut package = valid_sparse_package();
    package.manifest.room_order = vec![300, 404];

    let report = validate_runtime_package(&package);

    assert!(!report.valid);
    assert_eq!(
        report.errors,
        vec![RuntimePackageValidationError::MissingRoomOrderRoom { room_id: 404 }]
    );
}

#[test]
fn rejects_room_instances_that_reference_missing_objects() {
    let mut package = valid_sparse_package();
    package.rooms[0].instances[0].object_id = 999;

    let report = validate_runtime_package(&package);

    assert!(!report.valid);
    assert!(report
        .errors
        .contains(&RuntimePackageValidationError::MissingRoomInstanceObject {
            room_id: 300,
            instance_id: 1234,
            object_id: 999,
        }));
}

#[test]
fn rejects_objects_that_reference_missing_sprites() {
    let mut package = valid_sparse_package();
    package.objects[0].sprite_index = 43;

    let report = validate_runtime_package(&package);

    assert!(!report.valid);
    assert!(report
        .errors
        .contains(&RuntimePackageValidationError::MissingObjectSprite {
            object_id: 705,
            sprite_index: 43,
        }));
}

#[test]
fn rejects_event_blocks_missing_from_lowered_logic() {
    let mut package = valid_sparse_package();
    package.lowered_logic.entries.clear();

    let report = validate_runtime_package(&package);

    assert!(!report.valid);
    assert!(report
        .errors
        .contains(&RuntimePackageValidationError::MissingLogicBlock {
            owner: "object:705:event:3:0".into(),
            block_id: "object:705:event:3:0".into(),
            missing_from: "logic.lowered.json".into(),
        }));
}

#[test]
fn reads_and_validates_sparse_fixture_package() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sparse-runtime-package");

    let package = read_runtime_package_dir(&fixture).expect("fixture should deserialize");
    let report = validate_runtime_package(&package);

    assert!(report.valid, "expected fixture to validate, got {report:?}");
}
