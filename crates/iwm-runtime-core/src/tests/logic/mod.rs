use std::cell::Cell;
use std::path::{Path, PathBuf};

use iwm_runtime_host::{
    ButtonState, ExternalSignature, ExternalValue, HeadlessHost, RuntimeAudioHost, RuntimeButton,
    RuntimeDiagnostic, RuntimeDiagnosticsHost, RuntimeExternalHost, RuntimeFileHost,
    RuntimeHostError, RuntimeInputHost, RuntimeRenderFrame, RuntimeRenderHost, RuntimeSoundMode,
    RuntimeTimeHost,
};

use crate::{LoweredLogicExpr, LoweredLogicStatement, RuntimeCore, RuntimeValue};

use super::support::{
    add_alarm_block, add_create_block, add_destroy_block, add_keyboard_block,
    add_room_create_block, add_script_block, add_step_block, append_lowered_entry,
    assert_no_runtime_blockers, host, player, player_mut, player_var, real_sample_package,
    sample_package,
};
use iwm_runtime_model::{ObjectDefinition, ObjectEventEntry};

mod audio;
mod bootstrap;
mod bootstrap_world;
mod diagnostics;
mod events;
mod expressions;
mod file_sampling;
mod instances;
mod real_sample;
mod room_start;
mod script_jump;
mod step;

struct ReadCountingHost {
    inner: HeadlessHost,
    read_count: Cell<usize>,
}

impl ReadCountingHost {
    fn new() -> Self {
        Self {
            inner: host(),
            read_count: Cell::new(0),
        }
    }
}

impl RuntimeTimeHost for ReadCountingHost {
    fn now_nanos(&self) -> u128 {
        self.inner.now_nanos()
    }

    fn diagnostic_now_nanos(&self) -> Option<u128> {
        self.inner.diagnostic_now_nanos()
    }

    fn tick_rate_hz(&self) -> u32 {
        self.inner.tick_rate_hz()
    }
}

impl RuntimeInputHost for ReadCountingHost {
    fn button_state(&self, button: RuntimeButton) -> ButtonState {
        self.inner.button_state(button)
    }

    fn active_buttons(&self) -> Vec<(RuntimeButton, ButtonState)> {
        self.inner.active_buttons()
    }

    fn mouse_position(&self) -> (i32, i32) {
        self.inner.mouse_position()
    }
}

impl RuntimeRenderHost for ReadCountingHost {
    fn submit_frame(&mut self, frame: RuntimeRenderFrame) -> Result<(), RuntimeHostError> {
        self.inner.submit_frame(frame)
    }
}

impl RuntimeAudioHost for ReadCountingHost {
    fn play_sound(
        &mut self,
        sound_id: i32,
        mode: RuntimeSoundMode,
    ) -> Result<(), RuntimeHostError> {
        self.inner.play_sound(sound_id, mode)
    }

    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError> {
        self.inner.stop_sound(sound_id)
    }

    fn stop_all_sounds(&mut self) -> Result<(), RuntimeHostError> {
        self.inner.stop_all_sounds()
    }

    fn is_sound_playing(&self, sound_id: i32) -> Result<bool, RuntimeHostError> {
        self.inner.is_sound_playing(sound_id)
    }
}

impl RuntimeFileHost for ReadCountingHost {
    fn read(&self, path: &Path) -> Result<Vec<u8>, RuntimeHostError> {
        self.read_count.set(self.read_count.get() + 1);
        self.inner.read(path)
    }

    fn write_temp(
        &mut self,
        relative_path: &Path,
        bytes: &[u8],
    ) -> Result<PathBuf, RuntimeHostError> {
        self.inner.write_temp(relative_path, bytes)
    }

    fn remove_temp(&mut self, relative_path: &Path) -> Result<(), RuntimeHostError> {
        self.inner.remove_temp(relative_path)
    }
}

impl RuntimeExternalHost for ReadCountingHost {
    fn define(&mut self, signature: ExternalSignature) -> Result<u32, RuntimeHostError> {
        self.inner.define(signature)
    }

    fn call(
        &mut self,
        handle: u32,
        args: &[ExternalValue],
    ) -> Result<ExternalValue, RuntimeHostError> {
        self.inner.call(handle, args)
    }

    fn free_library(&mut self, library: &str) -> Result<(), RuntimeHostError> {
        self.inner.free_library(library)
    }
}

impl RuntimeDiagnosticsHost for ReadCountingHost {
    fn record(&mut self, diagnostic: RuntimeDiagnostic) {
        self.inner.record(diagnostic);
    }
}
