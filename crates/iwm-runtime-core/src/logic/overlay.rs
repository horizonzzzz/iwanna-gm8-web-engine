use crate::RuntimeInstance;

#[derive(Debug, Default)]
pub(crate) struct RuntimeSparseInstanceOverlay {
    slots: Vec<Option<RuntimeInstance>>,
    dirty_indices: Vec<usize>,
    dirty_marks: Vec<bool>,
}

impl RuntimeSparseInstanceOverlay {
    pub(crate) fn get(&self, index: usize) -> Option<&RuntimeInstance> {
        self.slots.get(index).and_then(Option::as_ref)
    }

    pub(crate) fn set(&mut self, index: usize, instance: RuntimeInstance) {
        self.ensure_capacity(index);
        if !self.dirty_marks[index] {
            self.dirty_marks[index] = true;
            self.dirty_indices.push(index);
        }
        self.slots[index] = Some(instance);
    }

    pub(crate) fn take(&mut self, index: usize) -> Option<RuntimeInstance> {
        self.slots.get_mut(index).and_then(Option::take)
    }

    pub(crate) fn dirty_indices(&self) -> &[usize] {
        &self.dirty_indices
    }

    pub(crate) fn snapshot(&self) -> Self {
        let mut snapshot = Self::default();
        snapshot.extend_from_overlay(self);
        snapshot
    }

    pub(crate) fn extend_from_overlay(&mut self, overlay: &Self) {
        for &index in overlay.dirty_indices() {
            if let Some(instance) = overlay.get(index) {
                self.set(index, instance.clone());
            }
        }
    }

    pub(crate) fn drain_dirty_updates(&mut self) -> Vec<(usize, RuntimeInstance)> {
        let mut updates = Vec::with_capacity(self.dirty_indices.len());
        for index in self.dirty_indices.drain(..) {
            if let Some(instance) = self.slots.get_mut(index).and_then(Option::take) {
                updates.push((index, instance));
            }
            if let Some(mark) = self.dirty_marks.get_mut(index) {
                *mark = false;
            }
        }
        updates
    }

    #[cfg(test)]
    pub(crate) fn clear_dirty(&mut self) {
        for index in self.dirty_indices.drain(..) {
            if let Some(slot) = self.slots.get_mut(index) {
                *slot = None;
            }
            if let Some(mark) = self.dirty_marks.get_mut(index) {
                *mark = false;
            }
        }
    }

    fn ensure_capacity(&mut self, index: usize) {
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }
        if self.dirty_marks.len() <= index {
            self.dirty_marks.resize(index + 1, false);
        }
    }
}
