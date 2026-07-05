use iwm_runtime_core::{RuntimeCore, RuntimePackage};
use iwm_runtime_host::{
    ButtonState, ExternalSignature, ExternalValue, HeadlessHost, RuntimeAudioHost, RuntimeButton,
    RuntimeDiagnostic, RuntimeDiagnosticsHost, RuntimeExternalHost, RuntimeFileHost,
    RuntimeHostError, RuntimeInputHost, RuntimeRenderFrame, RuntimeRenderHost, RuntimeSoundMode,
    RuntimeTimeHost,
};
use std::path::{Path, PathBuf};

use crate::translate::{bridge_snapshot, format_core_error};
use crate::{BridgeFrameSnapshot, BridgeSnapshot, BridgeStepResult, WebAudioHost, WebInputState};

const DEFAULT_GM_ROOM_SPEED_HZ: u32 = 30;

#[derive(Debug)]
struct WebRuntimeHostBoundary {
    headless: HeadlessHost,
    audio: WebAudioHost,
}

impl WebRuntimeHostBoundary {
    fn new() -> Self {
        Self {
            headless: HeadlessHost::new("runtime-web"),
            audio: WebAudioHost::default(),
        }
    }
}

#[derive(Debug)]
pub struct WebRuntimeHost {
    host: WebRuntimeHostBoundary,
    core: Option<RuntimeCore>,
    package: Option<RuntimePackage>,
    previous_left: bool,
    previous_right: bool,
    previous_restart: bool,
}

impl WebRuntimeHost {
    pub fn new() -> Self {
        Self {
            host: WebRuntimeHostBoundary::new(),
            core: None,
            package: None,
            previous_left: false,
            previous_right: false,
            previous_restart: false,
        }
    }

    pub fn boot(&mut self, package: RuntimePackage) -> Result<BridgeSnapshot, String> {
        let mut core = RuntimeCore::load(package.clone()).map_err(format_core_error)?;
        let mut host = WebRuntimeHostBoundary::new();
        sync_host_tick_rate_from_core(&mut host, &core);
        core.render(&mut host).map_err(format_core_error)?;
        let snapshot = bridge_snapshot(core.snapshot());
        self.core = Some(core);
        self.package = Some(package);
        self.host = host;
        self.previous_left = false;
        self.previous_right = false;
        self.previous_restart = false;
        Ok(snapshot)
    }

    pub fn boot_from_json(&mut self, package_json: &str) -> Result<BridgeSnapshot, String> {
        let package = serde_json::from_str::<RuntimePackage>(package_json)
            .map_err(|error| error.to_string())?;
        self.boot(package)
    }

    pub fn set_input(&mut self, input: WebInputState) {
        let left_just_pressed = input.left && !self.previous_left;
        let left_just_released = !input.left && self.previous_left;
        let right_just_pressed = input.right && !self.previous_right;
        let right_just_released = !input.right && self.previous_right;
        let restart_just_pressed = input.restart && !self.previous_restart;
        let restart_just_released = !input.restart && self.previous_restart;
        let mut states = input
            .keys_held
            .iter()
            .copied()
            .map(|key| {
                (
                    RuntimeButton::Keyboard(key),
                    ButtonState {
                        pressed: true,
                        just_pressed: input.keys_pressed.contains(&key),
                        just_released: false,
                    },
                )
            })
            .collect::<std::collections::HashMap<_, _>>();

        for key in &input.keys_pressed {
            states
                .entry(RuntimeButton::Keyboard(*key))
                .and_modify(|state| state.just_pressed = true)
                .or_insert(ButtonState {
                    pressed: false,
                    just_pressed: true,
                    just_released: false,
                });
        }

        for key in &input.keys_released {
            states
                .entry(RuntimeButton::Keyboard(*key))
                .and_modify(|state| state.just_released = true)
                .or_insert(ButtonState {
                    pressed: false,
                    just_pressed: false,
                    just_released: true,
                });
        }

        merge_semantic_button_state(
            &mut states,
            RuntimeButton::Keyboard(0x25),
            ButtonState {
                pressed: input.left,
                just_pressed: left_just_pressed,
                just_released: left_just_released,
            },
        );
        merge_semantic_button_state(
            &mut states,
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: input.right,
                just_pressed: right_just_pressed,
                just_released: right_just_released,
            },
        );
        merge_semantic_button_state(
            &mut states,
            RuntimeButton::Restart,
            ButtonState {
                pressed: input.restart,
                just_pressed: restart_just_pressed,
                just_released: restart_just_released,
            },
        );

        self.host.headless.input.replace_button_states(states);
        self.previous_left = input.left;
        self.previous_right = input.right;
        self.previous_restart = input.restart;
    }

    pub fn tick(&mut self, frames: u32) -> Result<BridgeSnapshot, String> {
        let Some(core) = self.core.as_mut() else {
            return Err("runtime core is not booted".into());
        };

        let frame_count = frames.max(1);
        for _ in 0..frame_count {
            self.host.headless.clock.advance_frames(1);
            core.tick(&mut self.host).map_err(format_core_error)?;
            sync_host_tick_rate_from_core(&mut self.host, core);
            self.host.headless.input.clear_transitions();
        }

        Ok(bridge_snapshot(core.snapshot()))
    }

    pub fn step(&mut self, input: WebInputState) -> Result<BridgeStepResult, String> {
        self.set_input(input);
        let snapshot = self.tick(1)?;
        let frame = self.frame_snapshot()?.clone();
        Ok(BridgeStepResult { snapshot, frame })
    }

    pub fn reset(&mut self) -> Result<BridgeSnapshot, String> {
        let Some(package) = self.package.clone() else {
            return Err("runtime core is not booted".into());
        };

        let mut host = WebRuntimeHostBoundary::new();
        let mut core = RuntimeCore::load(package).map_err(format_core_error)?;
        sync_host_tick_rate_from_core(&mut host, &core);
        core.render(&mut host).map_err(format_core_error)?;
        let snapshot = bridge_snapshot(core.snapshot());
        self.host = host;
        self.core = Some(core);
        self.previous_restart = false;
        self.previous_left = false;
        self.previous_right = false;
        Ok(snapshot)
    }

    pub fn select_room(&mut self, room_id: usize) -> Result<BridgeSnapshot, String> {
        let Some(core) = self.core.as_mut() else {
            return Err("runtime core is not booted".into());
        };

        core.reload_room(room_id).map_err(format_core_error)?;
        sync_host_tick_rate_from_core(&mut self.host, core);
        core.render(&mut self.host).map_err(format_core_error)?;
        Ok(bridge_snapshot(core.snapshot()))
    }

    pub fn snapshot(&self) -> Option<BridgeSnapshot> {
        self.core
            .as_ref()
            .map(|core| bridge_snapshot(core.snapshot()))
    }

    pub fn diagnostics(&self) -> Vec<String> {
        self.snapshot()
            .map(|snapshot| snapshot.diagnostics)
            .unwrap_or_default()
    }

    pub fn frame_snapshot(&self) -> Result<&BridgeFrameSnapshot, String> {
        self.host
            .headless
            .renderer
            .submitted_frames
            .last()
            .ok_or_else(|| "runtime has not submitted a frame yet".to_string())
    }

    pub fn host_frame_count(&self) -> usize {
        self.host.headless.renderer.submitted_frames.len()
    }

    pub fn audio_events(&self) -> &[String] {
        self.host.audio.events()
    }
}

fn sync_host_tick_rate_from_core(host: &mut WebRuntimeHostBoundary, core: &RuntimeCore) {
    let tick_rate = core
        .current_room_speed()
        .filter(|speed| *speed > 0)
        .unwrap_or(DEFAULT_GM_ROOM_SPEED_HZ);
    host.headless.clock.set_tick_rate_hz(tick_rate);
}

impl RuntimeTimeHost for WebRuntimeHostBoundary {
    fn now_nanos(&self) -> u128 {
        self.headless.now_nanos()
    }

    fn diagnostic_now_nanos(&self) -> Option<u128> {
        self.headless.diagnostic_now_nanos()
    }

    fn tick_rate_hz(&self) -> u32 {
        self.headless.tick_rate_hz()
    }
}

impl RuntimeInputHost for WebRuntimeHostBoundary {
    fn button_state(&self, button: RuntimeButton) -> ButtonState {
        self.headless.button_state(button)
    }

    fn keyboard_numlock(&self) -> bool {
        self.headless.keyboard_numlock()
    }

    fn set_keyboard_numlock(&mut self, state: bool) {
        self.headless.set_keyboard_numlock(state);
    }

    fn active_buttons(&self) -> Vec<(RuntimeButton, iwm_runtime_host::ButtonState)> {
        self.headless.active_buttons()
    }

    fn mouse_position(&self) -> (i32, i32) {
        self.headless.mouse_position()
    }
}

impl RuntimeRenderHost for WebRuntimeHostBoundary {
    fn submit_frame(&mut self, frame: RuntimeRenderFrame) -> Result<(), RuntimeHostError> {
        self.headless.submit_frame(frame)
    }
}

impl RuntimeAudioHost for WebRuntimeHostBoundary {
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

    fn stop_all_sounds(&mut self) -> Result<(), RuntimeHostError> {
        self.audio.stop_all_sounds()
    }

    fn is_sound_playing(&self, sound_id: i32) -> Result<bool, RuntimeHostError> {
        self.audio.is_sound_playing(sound_id)
    }
}

impl RuntimeFileHost for WebRuntimeHostBoundary {
    fn read(&self, path: &Path) -> Result<Vec<u8>, RuntimeHostError> {
        if let Some(bytes) = crate::file_host::read_file(path)? {
            return Ok(bytes);
        }
        self.headless.read(path)
    }

    fn write_temp(
        &mut self,
        relative_path: &Path,
        bytes: &[u8],
    ) -> Result<PathBuf, RuntimeHostError> {
        let written = self.headless.write_temp(relative_path, bytes)?;
        crate::file_host::write_file(relative_path, bytes)?;
        Ok(written)
    }

    fn remove_temp(&mut self, relative_path: &Path) -> Result<(), RuntimeHostError> {
        let removed_from_browser = crate::file_host::remove_file(relative_path)?;
        match self.headless.remove_temp(relative_path) {
            Ok(()) => Ok(()),
            Err(_) if removed_from_browser => Ok(()),
            Err(error) => Err(error),
        }
    }
}

impl RuntimeExternalHost for WebRuntimeHostBoundary {
    fn define(&mut self, signature: ExternalSignature) -> Result<u32, RuntimeHostError> {
        self.headless.define(signature)
    }

    fn call(
        &mut self,
        handle: u32,
        args: &[ExternalValue],
    ) -> Result<ExternalValue, RuntimeHostError> {
        self.headless.call(handle, args)
    }

    fn free_library(&mut self, library: &str) -> Result<(), RuntimeHostError> {
        self.headless.free_library(library)
    }
}

impl RuntimeDiagnosticsHost for WebRuntimeHostBoundary {
    fn record(&mut self, diagnostic: RuntimeDiagnostic) {
        self.headless.record(diagnostic);
    }
}

fn merge_semantic_button_state(
    states: &mut std::collections::HashMap<RuntimeButton, ButtonState>,
    button: RuntimeButton,
    semantic: ButtonState,
) {
    states
        .entry(button)
        .and_modify(|state| {
            state.pressed |= semantic.pressed;
            state.just_pressed |= semantic.just_pressed;
            state.just_released |= semantic.just_released;
        })
        .or_insert(semantic);
}

impl Default for WebRuntimeHost {
    fn default() -> Self {
        Self::new()
    }
}
