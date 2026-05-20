use crate::models::{
    LogicBlock, LogicOp, ObjectDefinition, ObjectEventEntry, RoomBackgroundLayer, RoomDefinition,
    RoomInstancePlacement, RoomView, ScriptIrFile,
};
use gm8exe::{
    asset::{CodeAction, Object, Room},
    AssetList,
};

pub fn export_rooms_and_logic(
    rooms: &AssetList<Room>,
    objects: &AssetList<Object>,
) -> (Vec<RoomDefinition>, Vec<ObjectDefinition>, ScriptIrFile) {
    let mut blocks = Vec::new();

    let room_defs = rooms
        .iter()
        .enumerate()
        .filter_map(|(room_id, room)| room.as_ref().map(|room| (room_id, room)))
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
                    ops: vec![LogicOp::SourceSnippet {
                        code: room.creation_code.to_string(),
                    }],
                });
                Some(id)
            };

            let instances = room
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
                            ops: vec![LogicOp::SourceSnippet {
                                code: instance.creation_code.to_string(),
                            }],
                        });
                        Some(id)
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
                    }
                })
                .collect();

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
                    })
                    .collect(),
                instances,
                creation_block_id,
            }
        })
        .collect();

    let object_defs = objects
        .iter()
        .enumerate()
        .filter_map(|(object_id, object)| object.as_ref().map(|object| (object_id, object)))
        .map(|(object_id, object)| {
            let mut events = Vec::new();

            for (event_type, sub_events) in object.events.iter().enumerate() {
                for (sub_event, actions) in sub_events {
                    let block_id = event_block_id(object_id, event_type, *sub_event);
                    blocks.push(LogicBlock {
                        id: block_id.clone(),
                        name: format!("object {} event {}:{}", object.name, event_type, sub_event),
                        kind: "object-event".into(),
                        support: detect_support(actions),
                        ops: actions.iter().map(action_to_logic_op).collect(),
                    });
                    events.push(ObjectEventEntry {
                        event_type,
                        sub_event: *sub_event,
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
                events,
            }
        })
        .collect();

    (
        room_defs,
        object_defs,
        ScriptIrFile {
            format: "iwm-script-ir-v1".into(),
            blocks,
        },
    )
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

fn detect_support(actions: &[CodeAction]) -> String {
    if actions.iter().any(|action| !action.fn_code.0.is_empty()) {
        "source-only".into()
    } else {
        "action-list".into()
    }
}
