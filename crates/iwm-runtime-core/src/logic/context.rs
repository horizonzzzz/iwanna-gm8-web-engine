use std::collections::{HashMap, HashSet};

use iwm_runtime_host::RuntimeButton;

use crate::{RuntimeInstance, RuntimeValue};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepExecutionResult {
    pub interrupted: bool,
    pub player_motion_changed: bool,
    pub player_jump_owned_by_script: bool,
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeExecutionScope {
    locals: HashMap<String, Option<RuntimeValue>>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeInstanceCreateRequest {
    pub(super) object_id: usize,
    pub(super) x: f64,
    pub(super) y: f64,
}

pub(crate) struct RuntimeRoomInstanceOverlay<'a> {
    committed_updates: Option<&'a HashMap<usize, RuntimeInstance>>,
    pending_updates: Vec<(usize, RuntimeInstance)>,
    current_instance: Option<(usize, RuntimeInstance)>,
}

impl<'a> RuntimeRoomInstanceOverlay<'a> {
    pub(crate) fn empty() -> Self {
        Self {
            committed_updates: None,
            pending_updates: Vec::new(),
            current_instance: None,
        }
    }

    pub(crate) fn with_current(
        committed_updates: &'a HashMap<usize, RuntimeInstance>,
        pending_updates: &[(usize, RuntimeInstance)],
        current_index: usize,
        current_instance: &RuntimeInstance,
    ) -> Self {
        Self {
            committed_updates: Some(committed_updates),
            pending_updates: pending_updates.to_vec(),
            current_instance: Some((current_index, current_instance.clone())),
        }
    }

    pub(super) fn merge_pending_current(
        &self,
        pending_updates: &[(usize, RuntimeInstance)],
        current_index: usize,
        current_instance: &RuntimeInstance,
    ) -> Self {
        let mut merged_pending = self.pending_updates.clone();
        merged_pending.extend(pending_updates.iter().cloned());
        Self {
            committed_updates: self.committed_updates,
            pending_updates: merged_pending,
            current_instance: Some((current_index, current_instance.clone())),
        }
    }

    fn instance_at<'b>(&'b self, index: usize, fallback: &'b RuntimeInstance) -> &'b RuntimeInstance
    where
        'a: 'b,
    {
        if let Some((current_index, instance)) = &self.current_instance {
            if *current_index == index {
                return instance;
            }
        }
        if let Some((_, instance)) = self
            .pending_updates
            .iter()
            .rev()
            .find(|(update_index, _)| *update_index == index)
        {
            return instance;
        }
        if let Some(instance) = self
            .committed_updates
            .and_then(|updates| updates.get(&index))
        {
            return instance;
        }
        fallback
    }
}

impl RuntimeExecutionScope {
    pub(super) fn declare(&mut self, name: &str) {
        self.locals.entry(name.to_string()).or_insert(None);
    }

    pub(super) fn get(&self, key: &str) -> Option<RuntimeValue> {
        self.locals.get(key).and_then(Clone::clone)
    }

    pub(super) fn assign(&mut self, key: &str, value: RuntimeValue) -> bool {
        if self.is_local_key(key) {
            self.locals.insert(key.to_string(), Some(value));
            true
        } else {
            false
        }
    }

    pub(super) fn is_local_key(&self, key: &str) -> bool {
        self.locals.contains_key(key)
            || key
                .split_once('[')
                .map(|(base, _)| self.locals.contains_key(base))
                .unwrap_or(false)
    }
}

pub(crate) struct RuntimeEvalContext<'a> {
    pub current_room_id: usize,
    pub button_states: &'a HashMap<RuntimeButton, iwm_runtime_host::ButtonState>,
    pub room_instances: &'a [RuntimeInstance],
    pub room_instance_indices_by_object_id: &'a HashMap<usize, Vec<usize>>,
    pub room_instance_overlay: RuntimeRoomInstanceOverlay<'a>,
    pub room_order: &'a [usize],
    pub known_files: &'a HashSet<String>,
    pub other_instance: Option<&'a RuntimeInstance>,
    pub other_runtime_id: Option<usize>,
    pub place_target_ids_by_name: &'a HashMap<String, Vec<usize>>,
}

impl<'a> RuntimeEvalContext<'a> {
    pub(super) fn with_other_and_overlay<'b>(
        &'b self,
        other_instance: &'b RuntimeInstance,
        room_instance_overlay: RuntimeRoomInstanceOverlay<'b>,
    ) -> RuntimeEvalContext<'b>
    where
        'a: 'b,
    {
        RuntimeEvalContext {
            current_room_id: self.current_room_id,
            button_states: self.button_states,
            room_instances: self.room_instances,
            room_instance_indices_by_object_id: self.room_instance_indices_by_object_id,
            room_instance_overlay,
            room_order: self.room_order,
            known_files: self.known_files,
            other_instance: Some(other_instance),
            other_runtime_id: Some(other_instance.runtime_id),
            place_target_ids_by_name: self.place_target_ids_by_name,
        }
    }

    pub(crate) fn other_instance(&self) -> Option<&RuntimeInstance> {
        let runtime_id = self
            .other_runtime_id
            .or_else(|| self.other_instance.map(|instance| instance.runtime_id))?;
        self.room_instances_iter()
            .find(|(_, instance)| instance.runtime_id == runtime_id)
            .map(|(_, instance)| instance)
    }

    pub(crate) fn room_instance(&self, index: usize) -> Option<&RuntimeInstance> {
        self.room_instances
            .get(index)
            .map(|fallback| self.room_instance_overlay.instance_at(index, fallback))
    }

    pub(crate) fn room_instances_iter(
        &self,
    ) -> impl Iterator<Item = (usize, &RuntimeInstance)> + '_ {
        self.room_instances
            .iter()
            .enumerate()
            .map(|(index, instance)| {
                (
                    index,
                    self.room_instance_overlay.instance_at(index, instance),
                )
            })
    }

    pub(crate) fn room_instances_matching_object_ids<'b>(
        &'b self,
        target_object_ids: &[usize],
    ) -> impl Iterator<Item = (usize, &'b RuntimeInstance)> + 'b {
        self.room_instance_indices_matching_object_ids(target_object_ids)
            .into_iter()
            .filter_map(|index| self.room_instance(index).map(|instance| (index, instance)))
    }

    fn room_instance_indices_matching_object_ids(&self, target_object_ids: &[usize]) -> Vec<usize> {
        if self.room_instance_indices_by_object_id.is_empty() {
            return self
                .room_instances
                .iter()
                .enumerate()
                .filter(|(_, instance)| target_object_ids.contains(&instance.object_id))
                .map(|(index, _)| index)
                .collect();
        }

        target_object_ids
            .iter()
            .filter_map(|object_id| self.room_instance_indices_by_object_id.get(object_id))
            .flat_map(|indices| indices.iter().copied())
            .collect()
    }
}
