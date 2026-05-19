pub mod detect;
pub mod models;
pub mod package;
pub mod signatures;

pub use detect::detect_input;
pub use models::{DetectionReport, DetectionVerdict, EngineFamily, PackageInputKind};
