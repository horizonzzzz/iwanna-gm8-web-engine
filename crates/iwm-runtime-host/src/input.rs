use std::collections::HashMap;

use crate::{ButtonState, RuntimeButton, RuntimeInputHost};

#[derive(Debug, Default)]
pub struct SnapshotInputHost {
    buttons: HashMap<RuntimeButton, ButtonState>,
    mouse_position: (i32, i32),
}

impl SnapshotInputHost {
    pub fn set_button_state(&mut self, button: RuntimeButton, state: ButtonState) {
        self.buttons.insert(button, state);
    }

    pub fn replace_button_states(
        &mut self,
        states: impl IntoIterator<Item = (RuntimeButton, ButtonState)>,
    ) {
        self.buttons.clear();
        self.buttons.extend(states);
    }

    pub fn clear_transitions(&mut self) {
        for state in self.buttons.values_mut() {
            state.just_pressed = false;
            state.just_released = false;
        }
    }

    pub fn set_mouse_position(&mut self, mouse_position: (i32, i32)) {
        self.mouse_position = mouse_position;
    }
}

impl RuntimeInputHost for SnapshotInputHost {
    fn button_state(&self, button: RuntimeButton) -> ButtonState {
        self.buttons.get(&button).copied().unwrap_or_default()
    }

    fn active_buttons(&self) -> Vec<(RuntimeButton, ButtonState)> {
        self.buttons.iter().map(|(button, state)| (*button, *state)).collect()
    }

    fn mouse_position(&self) -> (i32, i32) {
        self.mouse_position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_input_host_replaces_button_states() {
        let mut input = SnapshotInputHost::default();
        input.replace_button_states([(
            RuntimeButton::Keyboard(0x25),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        )]);

        assert!(input.button_state(RuntimeButton::Keyboard(0x25)).pressed);
        assert!(!input.button_state(RuntimeButton::Keyboard(0x27)).pressed);
    }
}
