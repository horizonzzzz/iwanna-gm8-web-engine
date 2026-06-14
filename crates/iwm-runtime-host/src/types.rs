use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{ser::SerializeTuple, Serialize, Serializer};

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

impl Serialize for Rgba8 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(4)?;
        tuple.serialize_element(&self.r)?;
        tuple.serialize_element(&self.g)?;
        tuple.serialize_element(&self.b)?;
        tuple.serialize_element(&self.a)?;
        tuple.end()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum RuntimeDrawCommand {
    Clear {
        colour: Rgba8,
    },
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
    DrawText {
        text: String,
        x: i32,
        y: i32,
        size: u32,
        colour: Rgba8,
        align: String,
    },
    Present,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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
