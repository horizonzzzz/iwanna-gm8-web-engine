pub mod detect;
pub mod models;
pub mod package;
pub mod signatures;

pub use detect::{detect_input, detect_package};
pub use models::{DetectionReport, DetectionVerdict, EngineFamily, PackageInputKind};
pub use package::{load_package, selected_executable, LoadedPackage};
