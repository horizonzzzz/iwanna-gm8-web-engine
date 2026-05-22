//! Host-boundary contracts for the WASM-first runtime path.
//!
//! This crate intentionally stays small. It defines the narrow host traits and
//! headless helpers needed for the first OpenGMK feasibility spike without
//! mirroring the full `gm8emulator` surface area.
//!
//! These traits and no-op/headless helpers are the only public runtime-host
//! surface that `iwm-runtime-core`, `iwm-runtime-web`, and future
//! OpenGMK-derived extraction work should depend on.

mod audio;
mod clock;
mod diagnostics;
mod externals;
mod files;
mod headless;
mod input;
mod render;
mod traits;
mod types;

pub use audio::NoopAudioHost;
pub use clock::DeterministicClock;
pub use diagnostics::VecDiagnosticsHost;
pub use externals::RejectingExternalHost;
pub use files::MemoryFileHost;
pub use headless::HeadlessHost;
pub use input::SnapshotInputHost;
pub use render::NullRenderHost;
pub use traits::{
    RuntimeAudioHost, RuntimeDiagnosticsHost, RuntimeExternalHost, RuntimeFileHost, RuntimeHost,
    RuntimeInputHost, RuntimeRenderHost, RuntimeTimeHost,
};
pub use types::{
    ButtonState, ExternalSignature, ExternalValue, Rgba8, RuntimeButton, RuntimeDiagnostic,
    RuntimeDiagnosticLevel, RuntimeDrawCommand, RuntimeHostError, RuntimeHostErrorKind,
    RuntimeRenderFrame, RuntimeSoundMode, DEFAULT_TICK_RATE_HZ,
};
