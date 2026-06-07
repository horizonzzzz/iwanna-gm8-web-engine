use crate::event_tags::normalize_event_tag;
use crate::logic_export::{
    event_block_id, instance_creation_block_id, room_creation_block_id, take_action_args,
};
use crate::models::{
    RawCodeAction, RawLogicEventBinding, RawLogicFile, RawLogicOwner, RawLogicOwnerKind,
    RawLogicScript, RawLogicTimelineMoment, RawLogicTrigger,
};
use gm8exe::{asset::CodeAction, GameAssets};

pub fn export_raw_logic(assets: &GameAssets) -> RawLogicFile {
    let mut room_creation_codes = Vec::new();
    let mut instance_creation_codes = Vec::new();
    let mut object_events = Vec::new();
    let mut scripts = Vec::new();
    let mut triggers = Vec::new();
    let mut timelines = Vec::new();

    for (room_id, room) in assets
        .rooms
        .iter()
        .enumerate()
        .filter_map(|(id, room)| room.as_ref().map(|room| (id, room)))
    {
        if !room.creation_code.0.is_empty() {
            room_creation_codes.push(RawLogicOwner {
                owner_kind: RawLogicOwnerKind::Room,
                owner_id: room_id as i32,
                owner_name: room.name.to_string(),
                event_type: None,
                sub_event: None,
                collision_object_id: None,
                block_id: room_creation_block_id(room_id),
                gml_source: room.creation_code.to_string(),
            });
        }

        for instance in &room.instances {
            if !instance.creation_code.0.is_empty() {
                instance_creation_codes.push(RawLogicOwner {
                    owner_kind: RawLogicOwnerKind::RoomInstance,
                    owner_id: instance.id,
                    owner_name: instance.object.to_string(),
                    event_type: None,
                    sub_event: None,
                    collision_object_id: None,
                    block_id: instance_creation_block_id(room_id, instance.id),
                    gml_source: instance.creation_code.to_string(),
                });
            }
        }
    }

    for (object_id, object) in assets
        .objects
        .iter()
        .enumerate()
        .filter_map(|(id, object)| object.as_ref().map(|object| (id, object)))
    {
        for (event_type, sub_events) in object.events.iter().enumerate() {
            for (sub_event, actions) in sub_events {
                let event_tag = normalize_event_tag(event_type, *sub_event);
                let collision_object_id = if event_type == 4 {
                    Some(*sub_event as i32)
                } else {
                    None
                };

                object_events.push(RawLogicEventBinding {
                    object_id,
                    object_name: object.name.to_string(),
                    event_type,
                    sub_event: *sub_event,
                    event_tag,
                    collision_object_id,
                    block_id: event_block_id(object_id, event_type, *sub_event),
                    actions: actions.iter().map(raw_action).collect(),
                });
            }
        }
    }

    for (script_id, script) in assets
        .scripts
        .iter()
        .enumerate()
        .filter_map(|(id, script)| script.as_ref().map(|script| (id, script)))
    {
        scripts.push(RawLogicScript {
            script_id,
            script_name: script.name.to_string(),
            gml_source: script.source.to_string(),
        });
    }

    for (trigger_id, trigger) in assets
        .triggers
        .iter()
        .enumerate()
        .filter_map(|(id, trigger)| trigger.as_ref().map(|trigger| (id, trigger)))
    {
        triggers.push(RawLogicTrigger {
            trigger_id,
            trigger_name: trigger.name.to_string(),
            constant_name: trigger.constant_name.to_string(),
            moment: trigger.moment.to_string(),
            condition_gml: trigger.condition.to_string(),
        });
    }

    for (timeline_id, timeline) in assets
        .timelines
        .iter()
        .enumerate()
        .filter_map(|(id, timeline)| timeline.as_ref().map(|timeline| (id, timeline)))
    {
        for (moment, actions) in &timeline.moments {
            timelines.push(RawLogicTimelineMoment {
                timeline_id,
                timeline_name: timeline.name.to_string(),
                moment: *moment,
                actions: actions.iter().map(raw_action).collect(),
            });
        }
    }

    RawLogicFile {
        format: "iwm-raw-logic-v1".into(),
        room_creation_codes,
        instance_creation_codes,
        object_events,
        scripts,
        triggers,
        timelines,
    }
}

fn raw_action(action: &CodeAction) -> RawCodeAction {
    RawCodeAction {
        action_id: action.id,
        lib_id: action.lib_id,
        action_kind: action.action_kind,
        execution_type: action.execution_type,
        fn_name: action.fn_name.to_string(),
        fn_code: action.fn_code.to_string(),
        args: take_action_args(
            action.param_count,
            std::array::from_fn(|index| action.param_strings[index].to_string()),
        ),
    }
}
