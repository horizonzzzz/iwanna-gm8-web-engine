use super::*;

#[test]
fn core_does_not_inject_player_into_rooms_without_spawn_logic() {
    let mut package = sample_package();
    package.manifest.default_room_id = Some(11);
    package.rooms.push(iwm_runtime_model::RoomDefinition {
        id: 11,
        name: "menu_like_room".into(),
        width: 320,
        height: 240,
        speed: 60,
        persistent: false,
        backgrounds: vec![],
        views_enabled: false,
        views: vec![],
        tiles: vec![],
        instances: vec![iwm_runtime_model::RoomInstancePlacement {
            instance_id: 99,
            object_id: 1,
            x: 120,
            y: 80,
            xscale: 1.0,
            yscale: 1.0,
            angle: 0.0,
            blend: 0x00ff_ffff,
            creation_block_id: None,
            is_solid: false,
            is_hazard: false,
            is_checkpoint: false,
        }],
        creation_block_id: None,
        playable: false,
        transition_targets: vec![],
    });

    let core = RuntimeCore::load(package).unwrap();

    assert!(core.snapshot().player.is_none());
}

#[test]
fn core_dispatches_room_start_events_after_room_build() {
    let mut package = sample_package();
    package.rooms[0]
        .instances
        .retain(|instance| instance.object_id != 0);
    package.objects[1]
        .events
        .push(iwm_runtime_model::ObjectEventEntry {
            event_type: 7,
            sub_event: 4,
            event_tag: "other:room-start".into(),
            block_id: "object:1:event:7:4".into(),
            action_count: 1,
        });
    package.scripts.blocks.push(iwm_runtime_model::LogicBlock {
        id: "object:1:event:7:4".into(),
        name: "marker room start".into(),
        kind: "object-event".into(),
        support: "source-only".into(),
        executable_action_count: 0,
        ops: vec![],
    });
    package.lowered_logic = Some(crate::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![crate::LoweredLogicEntry {
            block_id: "object:1:event:7:4".into(),
            statements: vec![LoweredLogicStatement::FunctionCall {
                name: "instance_create".into(),
                args: vec![
                    LoweredLogicExpr::Identifier("x".into()),
                    LoweredLogicExpr::Identifier("y".into()),
                    LoweredLogicExpr::Identifier("obj_player".into()),
                ],
            }],
        }],
    });

    let core = RuntimeCore::load(package).unwrap();

    let room = core.current_room().unwrap();
    let player = room
        .instances
        .iter()
        .find(|instance| instance.object_name == "obj_player")
        .expect("room-start event should create player");
    assert_eq!((player.x, player.y), (48.0, 64.0));
}

#[test]
fn room_start_assignment_updates_runtime_room_speed() {
    let mut package = sample_package();
    package.objects[1]
        .events
        .push(iwm_runtime_model::ObjectEventEntry {
            event_type: 7,
            sub_event: 4,
            event_tag: "other:room-start".into(),
            block_id: "object:1:event:7:4".into(),
            action_count: 1,
        });
    package.lowered_logic = Some(crate::LoweredLogicFile {
        format: "iwm-lowered-logic-v1".into(),
        entries: vec![crate::LoweredLogicEntry {
            block_id: "object:1:event:7:4".into(),
            statements: vec![LoweredLogicStatement::Assignment {
                target: LoweredLogicExpr::Identifier("room_speed".into()),
                value: LoweredLogicExpr::LiteralNumber(50.0),
            }],
        }],
    });

    let core = RuntimeCore::load(package).unwrap();

    assert_eq!(core.current_room_speed(), Some(50));
    assert_eq!(core.snapshot().room_speed, Some(50));
    assert!(core
        .current_room()
        .unwrap()
        .instances
        .iter()
        .all(|instance| { !instance.vars.contains_key("room_speed") }));
}
