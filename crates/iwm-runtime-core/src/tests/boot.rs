use crate::{
    LoweredLogicEntry, LoweredLogicExpr, LoweredLogicFile, LoweredLogicStatement, RuntimeCore,
    RuntimeCoreError, RuntimeStatus,
};
use crate::helpers::is_player_instance;

use super::support::sample_package;

#[test]
fn core_loads_default_room_and_instances() {
    let core = RuntimeCore::load(sample_package()).unwrap();

    assert_eq!(core.status(), RuntimeStatus::Ready);
    assert_eq!(core.current_room().map(|room| room.room_id), Some(7));
    assert_eq!(core.current_room().map(|room| room.instances.len()), Some(5));
    assert!(core.current_room().unwrap().instances[0].player_candidate);
}

#[test]
fn core_preserves_sparse_object_ids_in_room_instances() {
    let core = RuntimeCore::load(sample_package()).unwrap();
    let room = core.current_room().unwrap();

    assert!(room.instances.iter().any(|instance| instance.object_id == 705));
    assert!(room
        .instances
        .iter()
        .any(|instance| instance.object_id == 705 && instance.width == 16));
}

#[test]
fn core_reports_missing_room() {
    let mut package = sample_package();
    package.manifest.default_room_id = Some(99);

    let error = RuntimeCore::load(package).unwrap_err();
    assert!(matches!(error, RuntimeCoreError::RoomMissing(99)));
}

#[test]
fn core_does_not_spawn_fallback_player_when_room_has_checkpoint_but_no_spawn_logic() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .retain(|instance| instance.object_id != 0);
    package.rooms[0].instances[0].is_checkpoint = true;

    let core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();

    assert!(!room.instances.iter().any(is_player_instance));
    assert!(!room
        .instances
        .iter()
        .any(|instance| instance.object_id == 0));
}

#[test]
fn core_does_not_treat_player_start_markers_as_spawned_players() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .retain(|instance| instance.object_id != 0);
    package.rooms[0].instances[0].is_checkpoint = true;
    package.rooms[0]
        .instances
        .push(iwm_runtime_model::RoomInstancePlacement {
            instance_id: 99,
            object_id: 2,
            x: 24,
            y: 24,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: false,
            is_checkpoint: true,
        });
    package.objects.push(iwm_runtime_model::ObjectDefinition {
        id: 2,
        name: "playerStart".into(),
        sprite_index: -1,
        parent_index: -1,
        depth: -10,
        persistent: false,
        visible: false,
        solid: false,
        mask_index: -1,
        is_hazard: Some(false),
        is_checkpoint: Some(true),
        is_player: true,
        events: vec![],
    });

    let core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();

    assert!(!room.instances.iter().any(is_player_instance));
    assert!(room.instances.iter().any(|instance| {
        instance.object_name == "playerStart" && instance.player_candidate
    }));
}

#[test]
fn core_loads_structured_lowered_logic_entries() {
    let mut package = sample_package();
    package.lowered_logic = Some(LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![LoweredLogicEntry {
            block_id: "room:7:create".into(),
            statements: vec![
                LoweredLogicStatement::VariableDeclaration {
                    names: vec!["i".into()],
                },
                LoweredLogicStatement::For {
                    init: LoweredLogicExpr::Identifier("i = 0".into()),
                    condition: LoweredLogicExpr::BinaryExpr {
                        op: "<".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(3.0)),
                    },
                    step: LoweredLogicExpr::BinaryExpr {
                        op: "+".into(),
                        left: Box::new(LoweredLogicExpr::Identifier("i".into())),
                        right: Box::new(LoweredLogicExpr::LiteralNumber(1.0)),
                    },
                    body: vec![LoweredLogicStatement::Return {
                        value: Some(LoweredLogicExpr::LiteralBool(false)),
                    }],
                },
            ],
        }],
    });

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(core.current_room().map(|room| room.room_id), Some(7));
    assert!(core
        .current_room()
        .is_some_and(|room| room.instances.iter().any(|instance| instance.player_candidate)));
}
