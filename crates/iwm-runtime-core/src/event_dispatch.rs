use std::collections::HashMap;

use crate::helpers::bounds_at;
use iwm_runtime_model::ObjectDefinition;

use crate::{RuntimeInstance, RuntimePackage, RuntimeRoomState};

const COLLISION_SPATIAL_CELL_SIZE: i32 = 64;

#[derive(Clone)]
pub(crate) enum RuntimeEventSelector {
    Step,
    Draw,
    Destroy,
    Alarm(u32),
    KeyboardHeld(u16),
    KeyboardPressed(u16),
    KeyboardReleased(u16),
    OtherAnimationEnd,
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
    let (event_type, sub_event, wanted) = event_selector_parts(&selector);

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

        current_object_id = object_parent_id(&package.objects, id);
    }

    block_ids
}

pub(crate) fn event_owner_id_for_block_id(
    objects: &[ObjectDefinition],
    block_id: &str,
) -> Option<usize> {
    objects.iter().find_map(|object| {
        object
            .events
            .iter()
            .any(|event| event.block_id == block_id)
            .then_some(object.id)
    })
}

pub(crate) fn inherited_event_block_id(
    objects: &[ObjectDefinition],
    owner_object_id: usize,
    selector: &RuntimeEventSelector,
) -> Option<(usize, String)> {
    let (event_type, sub_event, wanted) = event_selector_parts(selector);
    let mut current_object_id = object_parent_id(objects, owner_object_id);
    while let Some(id) = current_object_id {
        let object = objects.iter().find(|object| object.id == id)?;
        if let Some(event) = object.events.iter().find(|event| {
            event.event_type == event_type
                && event.sub_event == sub_event
                && event.event_tag == wanted
        }) {
            return Some((object.id, event.block_id.clone()));
        }
        current_object_id = object_parent_id(objects, object.id);
    }

    None
}

fn event_selector_parts(selector: &RuntimeEventSelector) -> (usize, u32, String) {
    match selector {
        RuntimeEventSelector::Step => (3usize, 0u32, "step".to_string()),
        RuntimeEventSelector::Draw => (8usize, 0u32, "draw".to_string()),
        RuntimeEventSelector::Destroy => (1usize, 0u32, "destroy".to_string()),
        RuntimeEventSelector::Alarm(slot) => (2usize, *slot, format!("alarm:{slot}")),
        RuntimeEventSelector::KeyboardHeld(key) => (
            5usize,
            *key as u32,
            format!("keyboard:{}", format_key_name(*key)),
        ),
        RuntimeEventSelector::KeyboardPressed(key) => (
            9usize,
            *key as u32,
            format!("keypress:{}", format_key_name(*key)),
        ),
        RuntimeEventSelector::KeyboardReleased(key) => (
            10usize,
            *key as u32,
            format!("keyrelease:{}", format_key_name(*key)),
        ),
        RuntimeEventSelector::OtherAnimationEnd => {
            (7usize, 7u32, "other:animation-end".to_string())
        }
        RuntimeEventSelector::Collision { target_object_id } => {
            (4usize, *target_object_id as u32, "collision".to_string())
        }
    }
}

fn object_parent_id(objects: &[ObjectDefinition], object_id: usize) -> Option<usize> {
    objects
        .iter()
        .find(|object| object.id == object_id)
        .and_then(|object| object.parent_index.try_into().ok())
}

pub(crate) fn runtime_instance_indices_by_object_id(
    room: &RuntimeRoomState,
) -> HashMap<usize, Vec<usize>> {
    runtime_instance_indices_by_object_id_from_instances(&room.instances)
}

pub(crate) fn runtime_instance_indices_by_object_id_from_instances(
    instances: &[RuntimeInstance],
) -> HashMap<usize, Vec<usize>> {
    let mut indices = HashMap::new();
    for (index, instance) in instances.iter().enumerate() {
        if instance.alive {
            indices
                .entry(instance.object_id)
                .or_insert_with(Vec::new)
                .push(index);
        }
    }
    indices
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeCollisionSpatialIndex {
    cells_by_object_id: HashMap<usize, HashMap<(i32, i32), Vec<usize>>>,
    solid_cells: HashMap<(i32, i32), Vec<usize>>,
}

impl RuntimeCollisionSpatialIndex {
    pub(crate) fn candidate_indices(
        &self,
        object_id: usize,
        instance: &RuntimeInstance,
        x: f64,
        y: f64,
    ) -> Vec<usize> {
        let Some(cells) = self.cells_by_object_id.get(&object_id) else {
            return Vec::new();
        };
        candidate_indices_from_cells(cells, instance, x, y)
    }

    pub(crate) fn solid_candidate_indices(
        &self,
        instance: &RuntimeInstance,
        x: f64,
        y: f64,
    ) -> Vec<usize> {
        candidate_indices_from_cells(&self.solid_cells, instance, x, y)
    }
}

pub(crate) fn runtime_collision_spatial_index(
    room: &RuntimeRoomState,
) -> RuntimeCollisionSpatialIndex {
    let mut index = RuntimeCollisionSpatialIndex::default();
    for (instance_index, instance) in room.instances.iter().enumerate() {
        if !instance.alive {
            continue;
        }
        let (left, top, right, bottom) = bounds_at(instance, instance.x, instance.y);
        let object_cells = index
            .cells_by_object_id
            .entry(instance.object_id)
            .or_default();
        for cell in cells_for_bounds(left, top, right, bottom) {
            object_cells.entry(cell).or_default().push(instance_index);
            if instance.solid {
                index
                    .solid_cells
                    .entry(cell)
                    .or_default()
                    .push(instance_index);
            }
        }
    }
    index
}

fn candidate_indices_from_cells(
    cells: &HashMap<(i32, i32), Vec<usize>>,
    instance: &RuntimeInstance,
    x: f64,
    y: f64,
) -> Vec<usize> {
    let mut candidates = Vec::new();
    let (left, top, right, bottom) = bounds_at(instance, x, y);
    for cell in cells_for_bounds(left, top, right, bottom) {
        let Some(indices) = cells.get(&cell) else {
            continue;
        };
        for index in indices {
            if !candidates.contains(index) {
                candidates.push(*index);
            }
        }
    }
    candidates
}

fn cells_for_bounds(left: i32, top: i32, right: i32, bottom: i32) -> Vec<(i32, i32)> {
    let right = (right - 1).max(left);
    let bottom = (bottom - 1).max(top);
    let left_cell = left.div_euclid(COLLISION_SPATIAL_CELL_SIZE);
    let right_cell = right.div_euclid(COLLISION_SPATIAL_CELL_SIZE);
    let top_cell = top.div_euclid(COLLISION_SPATIAL_CELL_SIZE);
    let bottom_cell = bottom.div_euclid(COLLISION_SPATIAL_CELL_SIZE);

    let mut cells = Vec::new();
    for y in top_cell..=bottom_cell {
        for x in left_cell..=right_cell {
            cells.push((x, y));
        }
    }
    cells
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

pub(crate) fn object_ids_matching_or_inheriting_from(
    package: &RuntimePackage,
    wanted_object_id: usize,
) -> Vec<usize> {
    package
        .objects
        .iter()
        .filter(|object| object_matches_or_inherits_from(package, object.id, wanted_object_id))
        .map(|object| object.id)
        .collect()
}

fn object_matches_or_inherits_from(
    package: &RuntimePackage,
    object_id: usize,
    wanted_object_id: usize,
) -> bool {
    let mut current_id = Some(object_id);
    while let Some(id) = current_id {
        if id == wanted_object_id {
            return true;
        }
        current_id = package
            .objects
            .iter()
            .find(|object| object.id == id)
            .and_then(|object| object.parent_index.try_into().ok());
    }
    false
}

fn format_key_name(sub_event: u16) -> String {
    let key = sub_event as u8 as char;
    if key.is_ascii_alphanumeric() {
        key.to_ascii_lowercase().to_string()
    } else {
        format!("0x{:02x}", sub_event as u8)
    }
}
