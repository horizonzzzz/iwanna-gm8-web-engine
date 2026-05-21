//! Host-boundary contracts for the WASM-first runtime path.
//!
//! This crate intentionally stays small. It defines the narrow host traits and
//! headless helpers needed for the first OpenGMK feasibility spike without
//! mirroring the full `gm8emulator` surface area.

use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_TICK_RATE_HZ: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeButton {
    Keyboard(u16),
    Mouse(u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonState {
    pub pressed: bool,
    pub just_pressed: bool,
    pub just_released: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeDrawCommand {
    Clear { colour: Rgba8 },
    DrawBackground {
        background_id: usize,
        x: i32,
        y: i32,
        stretch: bool,
        tile_horz: bool,
        tile_vert: bool,
        is_foreground: bool,
    },
    DrawTile {
        background_id: usize,
        x: i32,
        y: i32,
        tile_x: u32,
        tile_y: u32,
        width: u32,
        height: u32,
        xscale: f64,
        yscale: f64,
    },
    DrawSprite {
        sprite_id: usize,
        frame_index: usize,
        x: i32,
        y: i32,
        origin_x: i32,
        origin_y: i32,
        xscale: f64,
        yscale: f64,
        angle_degrees: f64,
    },
    FillRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        colour: Rgba8,
    },
    Present,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRenderFrame {
    pub tick: u64,
    pub room_id: Option<usize>,
    pub width: u32,
    pub height: u32,
    pub commands: Vec<RuntimeDrawCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSoundMode {
    Once,
    Loop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSignature {
    pub library: String,
    pub symbol: String,
    pub arg_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExternalValue {
    Real(f64),
    Str(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeDiagnosticLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDiagnostic {
    pub level: RuntimeDiagnosticLevel,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeHostErrorKind {
    Unsupported,
    NotFound,
    InvalidInput,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHostError {
    kind: RuntimeHostErrorKind,
    message: String,
}

impl RuntimeHostError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            kind: RuntimeHostErrorKind::Unsupported,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: RuntimeHostErrorKind::NotFound,
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            kind: RuntimeHostErrorKind::InvalidInput,
            message: message.into(),
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self {
            kind: RuntimeHostErrorKind::Io,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> RuntimeHostErrorKind {
        self.kind
    }
}

impl Display for RuntimeHostError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for RuntimeHostError {}

pub trait RuntimeTimeHost {
    fn now_nanos(&self) -> u128;

    fn tick_rate_hz(&self) -> u32 {
        DEFAULT_TICK_RATE_HZ
    }
}

pub trait RuntimeInputHost {
    fn button_state(&self, button: RuntimeButton) -> ButtonState;

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

#[derive(Debug, Clone, Copy)]
pub struct DeterministicClock {
    now_nanos: u128,
    tick_rate_hz: u32,
}

impl DeterministicClock {
    pub fn new(start_nanos: u128, tick_rate_hz: u32) -> Self {
        Self {
            now_nanos: start_nanos,
            tick_rate_hz,
        }
    }

    pub fn advance_frames(&mut self, frames: u64) {
        if self.tick_rate_hz == 0 {
            return;
        }

        let frame_nanos = 1_000_000_000u128 / u128::from(self.tick_rate_hz);
        self.now_nanos += frame_nanos.saturating_mul(u128::from(frames));
    }
}

impl Default for DeterministicClock {
    fn default() -> Self {
        Self::new(0, DEFAULT_TICK_RATE_HZ)
    }
}

impl RuntimeTimeHost for DeterministicClock {
    fn now_nanos(&self) -> u128 {
        self.now_nanos
    }

    fn tick_rate_hz(&self) -> u32 {
        self.tick_rate_hz
    }
}

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

    fn mouse_position(&self) -> (i32, i32) {
        self.mouse_position
    }
}

#[derive(Debug, Default)]
pub struct NullRenderHost {
    pub submitted_frames: Vec<RuntimeRenderFrame>,
}

impl RuntimeRenderHost for NullRenderHost {
    fn submit_frame(&mut self, frame: RuntimeRenderFrame) -> Result<(), RuntimeHostError> {
        self.submitted_frames.push(frame);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NoopAudioHost {
    pub played: Vec<(i32, RuntimeSoundMode)>,
    pub stopped: Vec<i32>,
}

impl RuntimeAudioHost for NoopAudioHost {
    fn play_sound(
        &mut self,
        sound_id: i32,
        mode: RuntimeSoundMode,
    ) -> Result<(), RuntimeHostError> {
        self.played.push((sound_id, mode));
        Ok(())
    }

    fn stop_sound(&mut self, sound_id: i32) -> Result<(), RuntimeHostError> {
        self.stopped.push(sound_id);
        Ok(())
    }
}

#[derive(Debug)]
pub struct MemoryFileHost {
    root: PathBuf,
    files: HashMap<PathBuf, Vec<u8>>,
}

impl MemoryFileHost {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            files: HashMap::new(),
        }
    }

    fn resolve_read_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    fn resolve_relative_path(&self, relative_path: &Path) -> Result<PathBuf, RuntimeHostError> {
        if relative_path.is_absolute() {
            return Err(RuntimeHostError::invalid_input(
                "absolute temp paths are not allowed",
            ));
        }

        for component in relative_path.components() {
            match component {
                Component::CurDir | Component::Normal(_) => {}
                Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                    return Err(RuntimeHostError::invalid_input(
                        "temp paths may not escape the configured host root",
                    ));
                }
            }
        }

        Ok(self.root.join(relative_path))
    }
}

impl RuntimeFileHost for MemoryFileHost {
    fn read(&self, path: &Path) -> Result<Vec<u8>, RuntimeHostError> {
        let resolved = self.resolve_read_path(path);
        self.files.get(&resolved).cloned().ok_or_else(|| {
            RuntimeHostError::not_found(format!("missing host file {}", resolved.display()))
        })
    }

    fn write_temp(
        &mut self,
        relative_path: &Path,
        bytes: &[u8],
    ) -> Result<PathBuf, RuntimeHostError> {
        let resolved = self.resolve_relative_path(relative_path)?;
        self.files.insert(resolved.clone(), bytes.to_vec());
        Ok(resolved)
    }

    fn remove_temp(&mut self, relative_path: &Path) -> Result<(), RuntimeHostError> {
        let resolved = self.resolve_relative_path(relative_path)?;
        self.files.remove(&resolved).map(|_| ()).ok_or_else(|| {
            RuntimeHostError::not_found(format!("missing host file {}", resolved.display()))
        })
    }
}

#[derive(Debug, Default)]
pub struct RejectingExternalHost {
    pub attempted_definitions: Vec<ExternalSignature>,
}

impl RuntimeExternalHost for RejectingExternalHost {
    fn define(&mut self, signature: ExternalSignature) -> Result<u32, RuntimeHostError> {
        self.attempted_definitions.push(signature.clone());
        Err(RuntimeHostError::unsupported(format!(
            "external host is disabled for {}!{}",
            signature.library, signature.symbol
        )))
    }

    fn call(
        &mut self,
        handle: u32,
        _args: &[ExternalValue],
    ) -> Result<ExternalValue, RuntimeHostError> {
        Err(RuntimeHostError::unsupported(format!(
            "external host is disabled for handle {}",
            handle
        )))
    }

    fn free_library(&mut self, library: &str) -> Result<(), RuntimeHostError> {
        Err(RuntimeHostError::unsupported(format!(
            "external host is disabled for library {}",
            library
        )))
    }
}

#[derive(Debug, Default)]
pub struct VecDiagnosticsHost {
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

impl RuntimeDiagnosticsHost for VecDiagnosticsHost {
    fn record(&mut self, diagnostic: RuntimeDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

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
    fn deterministic_clock_advances_by_frame_count() {
        let mut clock = DeterministicClock::new(0, 50);
        clock.advance_frames(3);

        assert_eq!(clock.now_nanos(), 60_000_000);
        assert_eq!(clock.tick_rate_hz(), 50);
    }

    #[test]
    fn memory_file_host_rejects_parent_paths() {
        let mut files = MemoryFileHost::new("sandbox");
        let error = files
            .write_temp(Path::new("../escape.bin"), b"data")
            .unwrap_err();

        assert_eq!(error.kind(), RuntimeHostErrorKind::InvalidInput);
    }

    #[test]
    fn memory_file_host_reads_writes_and_removes_temp_paths() {
        let mut files = MemoryFileHost::new("sandbox");
        let written = files
            .write_temp(Path::new("included/game.dat"), b"payload")
            .unwrap();

        assert_eq!(
            written,
            PathBuf::from("sandbox").join("included").join("game.dat")
        );
        assert_eq!(
            files.read(Path::new("included/game.dat")).unwrap(),
            b"payload"
        );

        files.remove_temp(Path::new("included/game.dat")).unwrap();
        let error = files.read(Path::new("included/game.dat")).unwrap_err();
        assert_eq!(error.kind(), RuntimeHostErrorKind::NotFound);
    }

    #[test]
    fn null_render_host_collects_frames() {
        let mut renderer = NullRenderHost::default();
        renderer
            .submit_frame(RuntimeRenderFrame {
                tick: 0,
                room_id: None,
                width: 320,
                height: 240,
                commands: vec![
                    RuntimeDrawCommand::Clear {
                        colour: Rgba8 {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 255,
                        },
                    },
                    RuntimeDrawCommand::Present,
                ],
            })
            .unwrap();

        assert_eq!(renderer.submitted_frames.len(), 1);
        assert_eq!(renderer.submitted_frames[0].commands.len(), 2);
    }

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

    #[test]
    fn rejecting_external_host_is_explicit() {
        let mut externals = RejectingExternalHost::default();
        let error = externals
            .define(ExternalSignature {
                library: "gmfmodsimple.dll".into(),
                symbol: "FMODSoundAdd".into(),
                arg_count: 2,
            })
            .unwrap_err();

        assert_eq!(error.kind(), RuntimeHostErrorKind::Unsupported);
        assert_eq!(externals.attempted_definitions.len(), 1);
    }

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
            level: RuntimeDiagnosticLevel::Info,
            code: "runtime-start".into(),
            message: "headless host booted".into(),
        });
        host.play_sound(7, RuntimeSoundMode::Once).unwrap();
        host.submit_frame(RuntimeRenderFrame {
            tick: 1,
            room_id: Some(7),
            width: 1,
            height: 1,
            commands: vec![RuntimeDrawCommand::Present],
        })
        .unwrap();

        assert!(host.now_nanos() > 0);
        assert!(host.button_state(RuntimeButton::Keyboard(0x25)).pressed);
        assert_eq!(host.audio.played, vec![(7, RuntimeSoundMode::Once)]);
        assert_eq!(host.renderer.submitted_frames.len(), 1);
        assert_eq!(host.diagnostics.diagnostics.len(), 1);
    }
}
