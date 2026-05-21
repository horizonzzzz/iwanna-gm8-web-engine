use std::path::{Path, PathBuf};

use crate::{
    ButtonState, DeterministicClock, ExternalSignature, ExternalValue, MemoryFileHost,
    NoopAudioHost, NullRenderHost, RejectingExternalHost, RuntimeAudioHost,
    RuntimeButton, RuntimeDiagnostic, RuntimeDiagnosticsHost, RuntimeExternalHost,
    RuntimeFileHost, RuntimeHostError, RuntimeInputHost, RuntimeRenderFrame,
    RuntimeRenderHost, RuntimeSoundMode, RuntimeTimeHost, SnapshotInputHost,
    VecDiagnosticsHost,
};

#[derive(Debug)]
pub struct HeadlessHost {
    pub clock: DeterministicClock,
    pub input: SnapshotInputHost,
    pub renderer: NullRenderHost,
    pub audio: NoopAudioHost,
    pub files: MemoryFileHost,
    pub externals: RejectingExternalHost,
    pub diagnostics: VecDiagnosticsHost,
}

impl HeadlessHost {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            clock: DeterministicClock::default(),
            input: SnapshotInputHost::default(),
            renderer: NullRenderHost::default(),
            audio: NoopAudioHost::default(),
            files: MemoryFileHost::new(root),
            externals: RejectingExternalHost::default(),
            diagnostics: VecDiagnosticsHost::default(),
        }
    }
}

impl RuntimeTimeHost for HeadlessHost {
    fn now_nanos(&self) -> u128 {
        self.clock.now_nanos()
    }

    fn tick_rate_hz(&self) -> u32 {
        self.clock.tick_rate_hz()
    }
}

impl RuntimeInputHost for HeadlessHost {
    fn button_state(&self, button: RuntimeButton) -> ButtonState {
        self.input.button_state(button)
    }

    fn mouse_position(&self) -> (i32, i32) {
        self.input.mouse_position()
    }
}

impl RuntimeRenderHost for HeadlessHost {
    fn submit_frame(&mut self, frame: RuntimeRenderFrame) -> Result<(), RuntimeHostError> {
        self.renderer.submit_frame(frame)
    }
}

impl RuntimeAudioHost for HeadlessHost {
    fn play_sound(
        &mut self,
        sound_id: i32,
        mode: RuntimeSoundMode,
    ) -> Result<(), RuntimeHostError> {
        self.audio.play_sound(sound_id, mode)
    }

    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError> {
        self.audio.stop_sound(sound_id)
    }
}

impl RuntimeFileHost for HeadlessHost {
    fn read(&self, path: &Path) -> Result<Vec<u8>, RuntimeHostError> {
        self.files.read(path)
    }

    fn write_temp(
        &mut self,
        relative_path: &Path,
        bytes: &[u8],
    ) -> Result<PathBuf, RuntimeHostError> {
        self.files.write_temp(relative_path, bytes)
    }

    fn remove_temp(&mut self, relative_path: &Path) -> Result<(), RuntimeHostError> {
        self.files.remove_temp(relative_path)
    }
}

impl RuntimeExternalHost for HeadlessHost {
    fn define(&mut self, signature: ExternalSignature) -> Result<u32, RuntimeHostError> {
        self.externals.define(signature)
    }

    fn call(
        &mut self,
        handle: u32,
        args: &[ExternalValue],
    ) -> Result<ExternalValue, RuntimeHostError> {
        self.externals.call(handle, args)
    }

    fn free_library(&mut self, library: &str) -> Result<(), RuntimeHostError> {
        self.externals.free_library(library)
    }
}

impl RuntimeDiagnosticsHost for HeadlessHost {
    fn record(&mut self, diagnostic: RuntimeDiagnostic) {
        self.diagnostics.record(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_host_composes_all_minimal_hosts() {
        let mut host = HeadlessHost::new("sandbox");
        host.clock.advance_frames(1);
        host.input.set_button_state(
            RuntimeButton::Keyboard(0x25),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        );
        host.record(RuntimeDiagnostic {
            level: crate::RuntimeDiagnosticLevel::Info,
            code: "runtime-start".into(),
            message: "headless host booted".into(),
        });
        host.play_sound(7, RuntimeSoundMode::Once).unwrap();
        host.submit_frame(RuntimeRenderFrame {
            tick: 1,
            room_id: Some(7),
            width: 1,
            height: 1,
            commands: vec![crate::RuntimeDrawCommand::Present],
        })
        .unwrap();

        assert!(host.now_nanos() > 0);
        assert!(host.button_state(RuntimeButton::Keyboard(0x25)).pressed);
        assert_eq!(host.audio.played, vec![(7, RuntimeSoundMode::Once)]);
        assert_eq!(host.renderer.submitted_frames.len(), 1);
        assert_eq!(host.diagnostics.diagnostics.len(), 1);
    }
}
