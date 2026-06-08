use std::collections::HashMap;

use crate::{RuntimePackage, RuntimeRoomState};

#[derive(Clone)]
pub(crate) enum RuntimeEventSelector {
    Destroy,
    Alarm(u32),
    KeyboardHeld(u16),
    KeyboardPressed(u16),
    KeyboardReleased(u16),
    #[cfg_attr(not(test), allow(dead_code))]
    Collision {
        target_object_id: usize,
    },
}

pub(crate) fn object_event_block_ids(
    package: &RuntimePackage,
    object_id: usize,
    selector: RuntimeEventSelector,
) -> Vec<String> {
    let (event_type, sub_event, wanted) = match selector {
        RuntimeEventSelector::Destroy => (1usize, 0u32, "destroy".to_string()),
        RuntimeEventSelector::Alarm(slot) => (2usize, slot, format!("alarm:{slot}")),
        RuntimeEventSelector::KeyboardHeld(key) => (
            5usize,
            key as u32,
            format!("keyboard:{}", format_key_name(key)),
        ),
        RuntimeEventSelector::KeyboardPressed(key) => (
            9usize,
            key as u32,
            format!("keypress:{}", format_key_name(key)),
        ),
        RuntimeEventSelector::KeyboardReleased(key) => (
            10usize,
            key as u32,
            format!("keyrelease:{}", format_key_name(key)),
        ),
        RuntimeEventSelector::Collision { target_object_id } => {
            (4usize, target_object_id as u32, "collision".to_string())
        }
    };

    let mut current_object_id = Some(object_id);
    let mut block_ids = Vec::new();
    while let Some(id) = current_object_id {
        let Some(object) = package.objects.iter().find(|object| object.id == id) else {
            break;
        };

        for event in &object.events {
            if event.event_type == event_type
                && event.sub_event == sub_event
                && event.event_tag == wanted
            {
                block_ids.push(event.block_id.clone());
            }
        }

        if !block_ids.is_empty() {
            break;
        }

        current_object_id = object.parent_index.try_into().ok();
    }

    block_ids
}

pub(crate) fn runtime_instance_indices_by_object_id(
    room: &RuntimeRoomState,
) -> HashMap<usize, Vec<usize>> {
    let mut indices = HashMap::new();
    for (index, instance) in room.instances.iter().enumerate() {
        if instance.alive {
            indices
                .entry(instance.object_id)
                .or_insert_with(Vec::new)
                .push(index);
        }
    }
    indices
}

pub(crate) fn collision_event_target_object_ids(
    package: &RuntimePackage,
    object_id: usize,
) -> Vec<usize> {
    let mut current_object_id = Some(object_id);
    while let Some(id) = current_object_id {
        let Some(object) = package.objects.iter().find(|object| object.id == id) else {
            break;
        };

        let target_ids = object
            .events
            .iter()
            .filter(|event| event.event_type == 4 && event.event_tag == "collision")
            .map(|event| event.sub_event as usize)
            .collect::<Vec<_>>();

        if !target_ids.is_empty() {
            return target_ids;
        }

        current_object_id = object.parent_index.try_into().ok();
    }

    Vec::new()
}

fn format_key_name(sub_event: u16) -> String {
    let key = sub_event as u8 as char;
    if key.is_ascii_alphanumeric() {
        key.to_ascii_lowercase().to_string()
    } else {
        format!("0x{:02x}", sub_event as u8)
    }
}
