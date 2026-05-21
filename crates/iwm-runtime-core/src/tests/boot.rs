use crate::{RuntimeCore, RuntimeCoreError, RuntimeStatus};

use super::support::sample_package;

#[test]
fn core_loads_default_room_and_instances() {
    let core = RuntimeCore::load(sample_package()).unwrap();

    assert_eq!(core.status(), RuntimeStatus::Ready);
    assert_eq!(core.current_room().map(|room| room.room_id), Some(7));
    assert_eq!(core.current_room().map(|room| room.instances.len()), Some(4));
    assert!(core.current_room().unwrap().instances[0].player_candidate);
}

#[test]
fn core_reports_missing_room() {
    let mut package = sample_package();
    package.manifest.default_room_id = Some(99);

    let error = RuntimeCore::load(package).unwrap_err();
    assert!(matches!(error, RuntimeCoreError::RoomMissing(99)));
}

#[test]
fn core_spawns_a_fallback_player_when_room_has_checkpoint_but_no_player_instance() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .retain(|instance| instance.object_id != 0);
    package.rooms[0].instances[0].is_checkpoint = true;

    let core = RuntimeCore::load(package).unwrap();
    let room = core.current_room().unwrap();

    assert!(room.instances.iter().any(|instance| instance.player_candidate));
    assert!(room
        .instances
        .iter()
        .any(|instance| instance.player_candidate && instance.instance_id == -1));
}

#[test]
fn core_ignores_player_start_markers_when_deciding_whether_a_room_has_a_player() {
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

    assert!(room.instances.iter().any(|instance| {
        instance.instance_id == -1 && instance.object_id == 0 && instance.player_candidate
    }));
}
