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
    pub room_instance_overlay: &'a [(usize, RuntimeInstance)],
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
        room_instance_overlay: &'b [(usize, RuntimeInstance)],
    ) -> RuntimeEvalContext<'b>
    where
        'a: 'b,
    {
        RuntimeEvalContext {
            current_room_id: self.current_room_id,
            button_states: self.button_states,
            room_instances: self.room_instances,
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
        self.room_instance_overlay
            .iter()
            .rev()
            .find(|(update_index, _)| *update_index == index)
            .map(|(_, instance)| instance)
            .or_else(|| self.room_instances.get(index))
    }

    pub(crate) fn room_instances_iter(
        &self,
    ) -> impl Iterator<Item = (usize, &RuntimeInstance)> + '_ {
        self.room_instances
            .iter()
            .enumerate()
            .map(|(index, instance)| {
                let instance = self
                    .room_instance_overlay
                    .iter()
                    .rev()
                    .find(|(update_index, _)| *update_index == index)
                    .map(|(_, instance)| instance)
                    .unwrap_or(instance);
                (index, instance)
            })
    }
}
