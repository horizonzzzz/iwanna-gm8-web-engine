use iwm_runtime_host::{ButtonState, RuntimeButton, RuntimeHost};

use crate::helpers::as_number;
use crate::{RuntimeCore, RuntimeInputTraceSnapshot};

impl RuntimeCore {
    pub(crate) fn update_jump_input_trace<H: RuntimeHost>(
        &mut self,
        host: &mut H,
        jump_state: ButtonState,
    ) {
        let jump_key = self
            .globals
            .get("global.jumpbutton")
            .and_then(as_number)
            .map(|value| value.round() as u16)
            .unwrap_or(0x20);
        let active_keys = host
            .active_buttons()
            .into_iter()
            .filter_map(|(button, state)| match button {
                RuntimeButton::Keyboard(key)
                    if state.pressed || state.just_pressed || state.just_released =>
                {
                    Some(format!(
                        "0x{key:02x}:p{}jp{}jr{}",
                        state.pressed as u8, state.just_pressed as u8, state.just_released as u8
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(",");

        self.last_input_trace = RuntimeInputTraceSnapshot {
            jump_button_key: jump_key,
            jump_pressed: jump_state.pressed,
            jump_just_pressed: jump_state.just_pressed,
            jump_just_released: jump_state.just_released,
            active_keys: active_keys
                .split(',')
                .filter(|entry| !entry.is_empty())
                .map(ToString::to_string)
                .collect(),
        };
    }
}
