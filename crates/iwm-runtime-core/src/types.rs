use std::collections::HashMap;

use iwm_runtime_host::{RuntimeDiagnostic, RuntimeHostError};
use iwm_runtime_model::{
    AnalysisReport, ObjectDefinition, ResourceIndex, RoomDefinition, RuntimeManifest, ScriptIrFile,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredLogicFile {
    pub format: String,
    pub entries: Vec<LoweredLogicEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweredLogicEntry {
    pub block_id: String,
    pub statements: Vec<LoweredLogicStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum LoweredLogicExpr {
    Identifier(String),
    LiteralNumber(f64),
    LiteralBool(bool),
    LiteralText(String),
    UnaryExpr {
        op: String,
        child: Box<LoweredLogicExpr>,
    },
    Call {
        name: String,
        args: Vec<LoweredLogicExpr>,
    },
    MemberAccess {
        target: Box<LoweredLogicExpr>,
        member: String,
    },
    IndexAccess {
        target: Box<LoweredLogicExpr>,
        index: Box<LoweredLogicExpr>,
    },
    BinaryExpr {
        op: String,
        left: Box<LoweredLogicExpr>,
        right: Box<LoweredLogicExpr>,
    },
    Raw {
        source: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LoweredLogicStatement {
    Assignment {
        target: LoweredLogicExpr,
        value: LoweredLogicExpr,
    },
    VariableDeclaration {
        names: Vec<String>,
    },
    Return {
        value: Option<LoweredLogicExpr>,
    },
    FunctionCall {
        name: String,
        args: Vec<LoweredLogicExpr>,
    },
    Conditional {
        condition: LoweredLogicExpr,
        then_branch: Vec<LoweredLogicStatement>,
        else_branch: Vec<LoweredLogicStatement>,
    },
    With {
        target: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    Repeat {
        count: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    While {
        condition: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    For {
        init: LoweredLogicExpr,
        condition: LoweredLogicExpr,
        step: LoweredLogicExpr,
        body: Vec<LoweredLogicStatement>,
    },
    Raw {
        source: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Number(f64),
    Bool(bool),
    Text(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeJumpState {
    pub active: bool,
    pub hold_frames: u32,
    pub cut_applied: bool,
    pub grounded_last_tick: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeJumpSnapshot {
    pub grounded: bool,
    pub active: bool,
    pub hold_frames: u32,
    pub cut_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePackage {
    pub manifest: RuntimeManifest,
    pub rooms: Vec<RoomDefinition>,
    pub objects: Vec<ObjectDefinition>,
    pub scripts: ScriptIrFile,
    #[serde(default, rename = "loweredLogic")]
    pub lowered_logic: Option<LoweredLogicFile>,
    pub resources: ResourceIndex,
    pub analysis: AnalysisReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatus {
    Idle,
    Ready,
    Running,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuntimeInstance {
    pub runtime_id: usize,
    pub instance_id: i32,
    pub object_id: usize,
    pub object_name: String,
    pub x: f64,
    pub y: f64,
    pub previous_x: f64,
    pub previous_y: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub width: i32,
    pub height: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_top: i32,
    pub bbox_bottom: i32,
    pub collision_masks: Vec<RuntimeCollisionMask>,
    pub per_frame_collision_masks: bool,
    pub facing_left: bool,
    pub alive: bool,
    pub solid: bool,
    pub hazard: bool,
    pub checkpoint: bool,
    pub player_candidate: bool,
    pub jump: RuntimeJumpState,
    pub vars: HashMap<String, RuntimeValue>,
}

#[derive(Debug, Clone)]
pub struct RuntimeCollisionMask {
    pub width: u32,
    pub height: u32,
    pub bbox_left: i32,
    pub bbox_right: i32,
    pub bbox_top: i32,
    pub bbox_bottom: i32,
    pub data: Vec<bool>,
}

#[derive(Debug, Clone)]
pub struct RuntimeRoomState {
    pub room_id: usize,
    pub room_name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub playable: bool,
    pub transition_targets: Vec<usize>,
    pub spawn_point: Option<(i32, i32)>,
    pub instances: Vec<RuntimeInstance>,
}

#[derive(Debug, Clone)]
pub struct RuntimePlayerSnapshot {
    pub x: f64,
    pub y: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub facing_left: bool,
    pub jump: RuntimeJumpSnapshot,
}

#[derive(Debug, Clone)]
pub struct RuntimeInputTraceSnapshot {
    pub jump_button_key: u16,
    pub jump_pressed: bool,
    pub jump_just_pressed: bool,
    pub jump_just_released: bool,
    pub active_keys: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub tick: u64,
    pub room_id: Option<usize>,
    pub room_name: Option<String>,
    pub instance_count: usize,
    pub player: Option<RuntimePlayerSnapshot>,
    pub input_trace: RuntimeInputTraceSnapshot,
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

#[derive(Debug)]
pub enum RuntimeCoreError {
    NoRooms,
    RoomMissing(usize),
    Host(RuntimeHostError),
}

impl From<RuntimeHostError> for RuntimeCoreError {
    fn from(value: RuntimeHostError) -> Self {
        Self::Host(value)
    }
}
