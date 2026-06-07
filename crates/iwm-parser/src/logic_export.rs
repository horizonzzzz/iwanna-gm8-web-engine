use crate::event_tags::normalize_event_tag;
use crate::models::{
    LogicBlock, LogicOp, ObjectDefinition, ObjectEventEntry, RoomBackgroundLayer, RoomDefinition,
    RoomInstancePlacement, RoomTilePlacement, RoomView, ScriptIrFile,
};
use gm8exe::{
    asset::{CodeAction, Object, Room, Script},
    AssetList,
};

pub fn export_rooms_and_logic(
    rooms: &AssetList<Room>,
    objects: &AssetList<Object>,
    scripts: &AssetList<Script>,
) -> (Vec<RoomDefinition>, Vec<ObjectDefinition>, ScriptIrFile) {
    let mut blocks = Vec::new();

    let mut room_defs: Vec<RoomDefinition> = rooms
        .iter()
        .enumerate()
        .filter_map(|ri| ri.1.as_ref().map(|r| (ri.0, r)))
        .map(|(room_id, room)| {
            let creation_block_id = if room.creation_code.0.is_empty() {
                None
            } else {
                let id = room_creation_block_id(room_id);
                blocks.push(LogicBlock {
                    id: id.clone(),
                    name: format!("room {} creation", room.name),
                    kind: "room-creation".into(),
                    support: "source-only".into(),
                    executable_action_count: 0,
                    ops: vec![LogicOp::SourceSnippet {
                        code: room.creation_code.to_string(),
                    }],
                });
                Some(id)
            };

            let instances: Vec<RoomInstancePlacement> = room
                .instances
                .iter()
                .map(|instance| {
                    let creation_block_id = if instance.creation_code.0.is_empty() {
                        None
                    } else {
                        let id = instance_creation_block_id(room_id, instance.id);
                        blocks.push(LogicBlock {
                            id: id.clone(),
                            name: format!("room {room_id} instance {} creation", instance.id),
                            kind: "instance-creation".into(),
                            support: "source-only".into(),
                            executable_action_count: 0,
                            ops: vec![LogicOp::SourceSnippet {
                                code: instance.creation_code.to_string(),
                            }],
                        });
                        Some(id)
                    };

                    // Look up the object to get solid/hazard/checkpoint hints
                    let (is_solid, is_hazard, is_checkpoint) =
                        match objects.get(instance.object as usize) {
                            Some(Some(obj)) => {
                                let name_str = String::from_utf8_lossy(&obj.name.0);
                                (
                                    obj.solid,
                                    detect_hazard(&name_str),
                                    detect_checkpoint(&name_str),
                                )
                            }
                            _ => (false, false, false),
                        };

                    RoomInstancePlacement {
                        instance_id: instance.id,
                        object_id: instance.object,
                        x: instance.x,
                        y: instance.y,
                        xscale: instance.xscale,
                        yscale: instance.yscale,
                        angle: instance.angle,
                        blend: instance.blend,
                        creation_block_id,
                        is_solid,
                        is_hazard,
                        is_checkpoint,
                    }
                })
                .collect();

            // Detect if room is playable (has instances with sprites that aren't decorative)
            let playable = instances
                .iter()
                .any(|i| match objects.get(i.object_id as usize) {
                    Some(Some(obj)) => {
                        let name_str = String::from_utf8_lossy(&obj.name.0);
                        obj.sprite_index >= 0 && !is_decorative_object(&name_str)
                    }
                    _ => false,
                });

            RoomDefinition {
                id: room_id,
                name: room.name.to_string(),
                width: room.width,
                height: room.height,
                speed: room.speed,
                persistent: room.persistent,
                backgrounds: room
                    .backgrounds
                    .iter()
                    .map(|bg| RoomBackgroundLayer {
                        visible_on_start: bg.visible_on_start,
                        is_foreground: bg.is_foreground,
                        source_bg: bg.source_bg,
                        xoffset: bg.xoffset,
                        yoffset: bg.yoffset,
                        tile_horz: bg.tile_horz,
                        tile_vert: bg.tile_vert,
                        hspeed: bg.hspeed,
                        vspeed: bg.vspeed,
                        stretch: bg.stretch,
                    })
                    .collect(),
                views_enabled: room.views_enabled,
                views: room
                    .views
                    .iter()
                    .map(|view| RoomView {
                        visible: view.visible,
                        source_x: view.source_x,
                        source_y: view.source_y,
                        source_w: view.source_w,
                        source_h: view.source_h,
                        port_x: view.port_x,
                        port_y: view.port_y,
                        port_w: view.port_w,
                        port_h: view.port_h,
                        target: view.following.target,
                        hborder: view.following.hborder,
                        vborder: view.following.vborder,
                        hspeed: view.following.hspeed,
                        vspeed: view.following.vspeed,
                    })
                    .collect(),
                tiles: room
                    .tiles
                    .iter()
                    .map(|tile| RoomTilePlacement {
                        tile_id: tile.id,
                        source_bg: tile.source_bg,
                        x: tile.x,
                        y: tile.y,
                        tile_x: tile.tile_x,
                        tile_y: tile.tile_y,
                        width: tile.width,
                        height: tile.height,
                        depth: tile.depth,
                        xscale: tile.xscale,
                        yscale: tile.yscale,
                        blend: tile.blend,
                    })
                    .collect(),
                instances,
                creation_block_id,
                playable,
                // Transition targets will be populated by analyzing room_goto actions
                transition_targets: Vec::new(),
            }
        })
        .collect();

    // Collect room transition targets from object events by object id first.
    let mut object_transition_map: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for (object_id, object) in objects.iter().enumerate() {
        if let Some(obj) = object.as_ref() {
            let mut targets = Vec::new();
            for sub_events in &obj.events {
                for (_sub_event, actions) in sub_events {
                    for action in actions {
                        if let Some(target_room) = detect_room_goto_target(action) {
                            targets.push(target_room);
                        }
                    }
                }
            }

            if !targets.is_empty() {
                targets.sort_unstable();
                targets.dedup();
                object_transition_map.insert(object_id, targets);
            }
        }
    }

    let object_defs = objects
        .iter()
        .enumerate()
        .filter_map(|(object_id, object)| object.as_ref().map(|o| (object_id, o)))
        .map(|(object_id, object)| {
            let mut events = Vec::new();

            for (event_type, sub_events) in object.events.iter().enumerate() {
                for (sub_event, actions) in sub_events {
                    let block_id = event_block_id(object_id, event_type, *sub_event);
                    let executable_count = count_executable_actions(actions);
                    let support = if executable_count > 0 {
                        "action-list"
                    } else {
                        "source-only"
                    };

                    blocks.push(LogicBlock {
                        id: block_id.clone(),
                        name: format!("object {} event {}:{}", object.name, event_type, sub_event),
                        kind: "object-event".into(),
                        support: support.into(),
                        executable_action_count: executable_count,
                        ops: actions.iter().map(action_to_logic_op).collect(),
                    });

                    events.push(ObjectEventEntry {
                        event_type,
                        sub_event: *sub_event,
                        event_tag: normalize_event_tag(event_type, *sub_event),
                        block_id,
                        action_count: actions.len(),
                    });
                }
            }

            ObjectDefinition {
                id: object_id,
                name: object.name.to_string(),
                sprite_index: object.sprite_index,
                parent_index: object.parent_index,
                depth: object.depth,
                persistent: object.persistent,
                visible: object.visible,
                solid: object.solid,
                mask_index: object.mask_index,
                is_hazard: detect_hazard(&String::from_utf8_lossy(object.name.0.as_ref()))
                    .then_some(true),
                is_checkpoint: detect_checkpoint(&String::from_utf8_lossy(object.name.0.as_ref()))
                    .then_some(true),
                is_player: detect_player(&String::from_utf8_lossy(object.name.0.as_ref())),
                events,
            }
        })
        .collect();

    // Update room transition_targets based on discovered targets for the objects present in each room.
    for room in &mut room_defs.iter_mut() {
        let mut transition_targets = room
            .instances
            .iter()
            .filter_map(|instance| object_transition_map.get(&(instance.object_id as usize)))
            .flat_map(|targets| targets.iter().copied())
            .collect::<Vec<_>>();
        transition_targets.sort_unstable();
        transition_targets.dedup();
        room.transition_targets = transition_targets;
    }

    for (script_id, script) in scripts
        .iter()
        .enumerate()
        .filter_map(|(id, script)| script.as_ref().map(|script| (id, script)))
    {
        blocks.push(LogicBlock {
            id: format!("script:{script_id}"),
            name: script.name.to_string(),
            kind: "script".into(),
            support: "source-only".into(),
            executable_action_count: 0,
            ops: vec![LogicOp::SourceSnippet {
                code: script.source.to_string(),
            }],
        });
    }

    (
        room_defs,
        object_defs,
        ScriptIrFile {
            format: "iwm-script-ir-v1".into(),
            blocks,
        },
    )
}

/// Count how many actions in a list can be executed without GML lowering
fn count_executable_actions(actions: &[CodeAction]) -> usize {
    actions.iter().filter(|a| a.fn_code.0.is_empty()).count()
}

/// Detect if an object name suggests it's a hazard
fn detect_hazard(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let hazard_patterns = [
        "hazard", "spike", "trap", "danger", "killer", "death", "hurt", "enemy", "bad",
    ];
    hazard_patterns.iter().any(|p| name_lower.contains(p))
}

/// Detect if an object name suggests it's a checkpoint
fn detect_checkpoint(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let checkpoint_patterns = ["checkpoint", "cp", "save", "flag", "spawn", "start"];
    checkpoint_patterns.iter().any(|p| name_lower.contains(p))
}

/// Detect if an object name suggests it's player-controlled
fn detect_player(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let player_patterns = ["player", "p1", "p2", "hero", "character", "avatar"];
    player_patterns.iter().any(|p| name_lower.contains(p))
}

/// Check if an object is decorative (not part of gameplay)
fn is_decorative_object(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    let decorative_patterns = ["bg_", "back", "deco", "particle", "effect"];
    decorative_patterns.iter().any(|p| name_lower.contains(p))
}

/// Try to detect room_goto target from an action
fn detect_room_goto_target(action: &CodeAction) -> Option<usize> {
    // room_goto(room) sets first arg to room index
    let fn_name_str = String::from_utf8_lossy(action.fn_name.0.as_ref());
    if fn_name_str.contains("room_goto") && !action.fn_code.0.is_empty() {
        // This is source code - we'd need to parse it to get the room
        None
    } else if fn_name_str.contains("room_goto") {
        // Action call - check args
        if let Some(room_arg) = action.param_strings.first() {
            let room_arg_str = String::from_utf8_lossy(room_arg.0.as_ref());
            room_arg_str.parse().ok()
        } else {
            None
        }
    } else {
        None
    }
}

pub fn event_block_id(object_id: usize, event_type: usize, sub_event: u32) -> String {
    format!("object:{object_id}:event:{event_type}:{sub_event}")
}

pub fn room_creation_block_id(room_id: usize) -> String {
    format!("room:{room_id}:create")
}

pub fn instance_creation_block_id(room_id: usize, instance_id: i32) -> String {
    format!("room:{room_id}:instance:{instance_id}:create")
}

pub fn take_action_args(param_count: usize, args: [String; 8]) -> Vec<String> {
    args.into_iter().take(param_count).collect()
}

fn action_to_logic_op(action: &CodeAction) -> LogicOp {
    let args = action
        .param_strings
        .iter()
        .take(action.param_count)
        .map(ToString::to_string)
        .collect();

    if !action.fn_code.0.is_empty() {
        return LogicOp::SourceSnippet {
            code: action.fn_code.to_string(),
        };
    }

    LogicOp::ActionCall {
        action_id: action.id,
        lib_id: action.lib_id,
        applies_to: action.applies_to,
        is_condition: action.is_condition,
        invert_condition: action.invert_condition,
        is_relative: action.is_relative,
        fn_name: action.fn_name.to_string(),
        fn_code: action.fn_code.to_string(),
        args,
    }
}
