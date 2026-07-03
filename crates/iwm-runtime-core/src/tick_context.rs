use crate::event_dispatch::{RuntimeCollisionScratch, RuntimeCollisionSpatialIndex};
use crate::RuntimeRoomState;

#[derive(Debug, Default)]
pub(crate) struct RuntimeTickContext {
    pub(crate) collision_spatial_index: RuntimeCollisionSpatialIndex,
    pub(crate) collision_scratch: RuntimeCollisionScratch,
    pub(crate) collision_hits: Vec<RuntimeCollisionHit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeCollisionHit {
    pub(crate) instance_idx: usize,
    pub(crate) target_object_id: usize,
    pub(crate) other_idx: usize,
    pub(crate) solid_collision: bool,
}

impl RuntimeTickContext {
    pub(crate) fn rebuild_collision_spatial_index(&mut self, room: &RuntimeRoomState) {
        self.collision_spatial_index.rebuild(room);
    }

    pub(crate) fn clear_collision_hits(&mut self) {
        self.collision_hits.clear();
    }
}
