use std::collections::HashMap;

use crate::event_dispatch::{RuntimeCollisionScratch, RuntimeCollisionSpatialIndex};
use crate::RuntimeRoomState;

#[derive(Debug, Default)]
pub(crate) struct RuntimeTickContext {
    pub(crate) collision_spatial_index: RuntimeCollisionSpatialIndex,
    pub(crate) collision_scratch: RuntimeCollisionScratch,
    pub(crate) collision_hits: Vec<RuntimeCollisionHit>,
    pub(crate) object_index: RuntimeObjectIndex,
    pub(crate) object_query_scratch: RuntimeObjectQueryScratch,
    pub(crate) dispatch_owners: Vec<(usize, usize)>,
    pub(crate) with_target_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeCollisionHit {
    pub(crate) instance_idx: usize,
    pub(crate) target_object_id: usize,
    pub(crate) other_idx: usize,
    pub(crate) solid_collision: bool,
    pub(crate) contact_y: Option<i32>,
}

impl RuntimeTickContext {
    pub(crate) fn rebuild_collision_spatial_index(&mut self, room: &RuntimeRoomState) {
        self.collision_spatial_index.rebuild(room);
    }

    pub(crate) fn rebuild_object_index(&mut self, room: &RuntimeRoomState) {
        self.object_index.rebuild(room);
    }

    pub(crate) fn clear_collision_hits(&mut self) {
        self.collision_hits.clear();
    }

    pub(crate) fn clear_dispatch_owners(&mut self) {
        self.dispatch_owners.clear();
    }
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeObjectIndex {
    indices_by_object_id: HashMap<usize, Vec<usize>>,
}

impl RuntimeObjectIndex {
    pub(crate) fn rebuild(&mut self, room: &RuntimeRoomState) {
        for indices in self.indices_by_object_id.values_mut() {
            indices.clear();
        }

        for (index, instance) in room.instances.iter().enumerate() {
            if !instance.alive {
                continue;
            }
            self.indices_by_object_id
                .entry(instance.object_id)
                .or_default()
                .push(index);
        }

        self.indices_by_object_id
            .retain(|_, indices| !indices.is_empty());
    }

    pub(crate) fn indices_for_object_id(&self, object_id: usize) -> &[usize] {
        self.indices_by_object_id
            .get(&object_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeObjectQueryScratch {
    candidates: Vec<usize>,
    candidate_marks: Vec<u32>,
    mark_epoch: u32,
}

impl RuntimeObjectQueryScratch {
    pub(crate) fn begin_query(&mut self, room_instance_len: usize) {
        self.candidates.clear();
        if self.candidate_marks.len() < room_instance_len {
            self.candidate_marks.resize(room_instance_len, 0);
        }
        self.mark_epoch = self.mark_epoch.wrapping_add(1);
        if self.mark_epoch == 0 {
            self.candidate_marks.fill(0);
            self.mark_epoch = 1;
        }
    }

    pub(crate) fn push_candidate(&mut self, index: usize) {
        if index >= self.candidate_marks.len() {
            self.candidate_marks.resize(index + 1, 0);
        }
        if self.candidate_marks[index] == self.mark_epoch {
            return;
        }
        self.candidate_marks[index] = self.mark_epoch;
        self.candidates.push(index);
    }

    pub(crate) fn candidates(&self) -> &[usize] {
        &self.candidates
    }
}
