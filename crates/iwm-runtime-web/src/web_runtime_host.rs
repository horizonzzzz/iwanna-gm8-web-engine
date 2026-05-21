use iwm_runtime_core::{RuntimeCore, RuntimePackage};
use iwm_runtime_host::{ButtonState, HeadlessHost, RuntimeButton};

use crate::{
    BridgeFrameSnapshot, BridgeSnapshot, WebInputState,
};
use crate::translate::{
    bridge_draw_command, bridge_frame_snapshot, bridge_snapshot, format_core_error,
};

#[derive(Debug)]
pub struct WebRuntimeHost {
    host: HeadlessHost,
    core: Option<RuntimeCore>,
    package: Option<RuntimePackage>,
}

impl WebRuntimeHost {
    pub fn new() -> Self {
        Self {
            host: HeadlessHost::new("runtime-web"),
            core: None,
            package: None,
        }
    }

    pub fn boot(&mut self, package: RuntimePackage) -> Result<BridgeSnapshot, String> {
        let mut core = RuntimeCore::load(package.clone()).map_err(format_core_error)?;
        let mut host = HeadlessHost::new("runtime-web");
        core.render(&mut host).map_err(format_core_error)?;
        let snapshot = bridge_snapshot(core.snapshot());
        self.core = Some(core);
        self.package = Some(package);
        self.host = host;
        Ok(snapshot)
    }

    pub fn boot_from_json(&mut self, package_json: &str) -> Result<BridgeSnapshot, String> {
        let package =
            serde_json::from_str::<RuntimePackage>(package_json).map_err(|error| error.to_string())?;
        self.boot(package)
    }

    pub fn set_input(&mut self, input: WebInputState) {
        self.host.input.replace_button_states([
            (
                RuntimeButton::Keyboard(0x25),
                ButtonState {
                    pressed: input.left,
                    just_pressed: input.left,
                    just_released: false,
                },
            ),
            (
                RuntimeButton::Keyboard(0x27),
                ButtonState {
                    pressed: input.right,
                    just_pressed: input.right,
                    just_released: false,
                },
            ),
            (
                RuntimeButton::Keyboard(0x20),
                ButtonState {
                    pressed: input.jump,
                    just_pressed: input.jump_pressed,
                    just_released: input.jump_released,
                },
            ),
            (
                RuntimeButton::Keyboard(0x52),
                ButtonState {
                    pressed: input.restart,
                    just_pressed: input.restart,
                    just_released: false,
                },
            ),
        ]);
    }

    pub fn tick(&mut self, frames: u32) -> Result<BridgeSnapshot, String> {
        let Some(core) = self.core.as_mut() else {
            return Err("runtime core is not booted".into());
        };

        let frame_count = frames.max(1);
        for _ in 0..frame_count {
            self.host.clock.advance_frames(1);
            core.tick(&mut self.host).map_err(format_core_error)?;
        }

        Ok(bridge_snapshot(core.snapshot()))
    }

    pub fn reset(&mut self) -> Result<BridgeSnapshot, String> {
        let Some(package) = self.package.clone() else {
            return Err("runtime core is not booted".into());
        };

        let mut host = HeadlessHost::new("runtime-web");
        let mut core = RuntimeCore::load(package).map_err(format_core_error)?;
        core.render(&mut host).map_err(format_core_error)?;
        let snapshot = bridge_snapshot(core.snapshot());
        self.host = host;
        self.core = Some(core);
        Ok(snapshot)
    }

    pub fn select_room(&mut self, room_id: usize) -> Result<BridgeSnapshot, String> {
        let Some(core) = self.core.as_mut() else {
            return Err("runtime core is not booted".into());
        };

        core.reload_room(room_id).map_err(format_core_error)?;
        core.render(&mut self.host).map_err(format_core_error)?;
        Ok(bridge_snapshot(core.snapshot()))
    }

    pub fn snapshot(&self) -> Option<BridgeSnapshot> {
        self.core.as_ref().map(|core| bridge_snapshot(core.snapshot()))
    }

    pub fn diagnostics(&self) -> Vec<String> {
        self.snapshot()
            .map(|snapshot| snapshot.diagnostics)
            .unwrap_or_default()
    }

    pub fn frame_snapshot(&self) -> Result<BridgeFrameSnapshot, String> {
        let frame = self
            .host
            .renderer
            .submitted_frames
            .last()
            .ok_or_else(|| "runtime has not submitted a frame yet".to_string())?;

        Ok(bridge_frame_snapshot(
            frame.tick,
            frame.room_id,
            frame.width,
            frame.height,
            frame.commands.iter().map(bridge_draw_command).collect(),
        ))
    }

    pub fn host_frame_count(&self) -> usize {
        self.host.renderer.submitted_frames.len()
    }
}

impl Default for WebRuntimeHost {
    fn default() -> Self {
        Self::new()
    }
}
