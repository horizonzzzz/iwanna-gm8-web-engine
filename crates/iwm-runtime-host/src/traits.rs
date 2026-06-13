use std::path::{Path, PathBuf};

use crate::types::{
    ButtonState, ExternalSignature, ExternalValue, RuntimeButton, RuntimeDiagnostic,
    RuntimeHostError, RuntimeRenderFrame, RuntimeSoundMode, DEFAULT_TICK_RATE_HZ,
};

pub trait RuntimeTimeHost {
    fn now_nanos(&self) -> u128;

    fn diagnostic_now_nanos(&self) -> Option<u128> {
        None
    }

    fn tick_rate_hz(&self) -> u32 {
        DEFAULT_TICK_RATE_HZ
    }
}

pub trait RuntimeInputHost {
    fn button_state(&self, button: RuntimeButton) -> ButtonState;

    fn keyboard_numlock(&self) -> bool {
        false
    }

    fn set_keyboard_numlock(&mut self, _state: bool) {}

    fn active_buttons(&self) -> Vec<(RuntimeButton, ButtonState)> {
        Vec::new()
    }

    fn mouse_position(&self) -> (i32, i32) {
        (0, 0)
    }
}

pub trait RuntimeRenderHost {
    fn submit_frame(&mut self, frame: RuntimeRenderFrame) -> Result<(), RuntimeHostError>;
}

pub trait RuntimeAudioHost {
    fn play_sound(&mut self, sound_id: i32, mode: RuntimeSoundMode)
        -> Result<(), RuntimeHostError>;
    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError>;
    fn stop_all_sounds(&mut self) -> Result<(), RuntimeHostError>;
    fn is_sound_playing(&self, sound_id: i32) -> Result<bool, RuntimeHostError>;
}

pub trait RuntimeFileHost {
    fn read(&self, path: &Path) -> Result<Vec<u8>, RuntimeHostError>;
    fn write_temp(
        &mut self,
        relative_path: &Path,
        bytes: &[u8],
    ) -> Result<PathBuf, RuntimeHostError>;
    fn remove_temp(&mut self, relative_path: &Path) -> Result<(), RuntimeHostError>;
}

pub trait RuntimeExternalHost {
    fn define(&mut self, signature: ExternalSignature) -> Result<u32, RuntimeHostError>;
    fn call(
        &mut self,
        handle: u32,
        args: &[ExternalValue],
    ) -> Result<ExternalValue, RuntimeHostError>;
    fn free_library(&mut self, library: &str) -> Result<(), RuntimeHostError>;
}

pub trait RuntimeDiagnosticsHost {
    fn record(&mut self, diagnostic: RuntimeDiagnostic);
}

pub trait RuntimeHost:
    RuntimeTimeHost
    + RuntimeInputHost
    + RuntimeRenderHost
    + RuntimeAudioHost
    + RuntimeFileHost
    + RuntimeExternalHost
    + RuntimeDiagnosticsHost
{
}

impl<T> RuntimeHost for T where
    T: RuntimeTimeHost
        + RuntimeInputHost
        + RuntimeRenderHost
        + RuntimeAudioHost
        + RuntimeFileHost
        + RuntimeExternalHost
        + RuntimeDiagnosticsHost
{
}
