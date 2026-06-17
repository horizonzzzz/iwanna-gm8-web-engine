use std::collections::HashMap;

use iwm_runtime_host::{RuntimeDiagnostic, RuntimeHostError};
use iwm_runtime_model::{
    AnalysisReport, ObjectDefinition, ResourceIndex, RoomDefinition, RoomView, RuntimeManifest,
    ScriptIrFile,
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
    pub visible: bool,
    pub alive: bool,
    pub solid: bool,
    pub hazard: bool,
    pub checkpoint: bool,
    pub player_candidate: bool,
    pub jump: RuntimeJumpState,
    pub vars: HashMap<String, RuntimeValue>,
}

const GM_ROUND_TOLERANCE: f64 = 0.0001;

impl RuntimeInstance {
    pub fn set_speed(&mut self, speed: f64) {
        self.vars
            .insert("speed".into(), RuntimeValue::Number(speed));
        self.sync_hvspeed_from_speed_direction();
    }

    pub fn set_direction(&mut self, direction: f64) {
        let normalized = direction.rem_euclid(360.0);
        self.vars
            .insert("direction".into(), RuntimeValue::Number(normalized));
        self.sync_hvspeed_from_speed_direction();
    }

    pub fn set_hspeed(&mut self, hspeed: f64) {
        if (self.hspeed - hspeed).abs() > f64::EPSILON {
            self.hspeed = hspeed;
            self.sync_speed_direction_from_hvspeed();
        }
    }

    pub fn set_vspeed(&mut self, vspeed: f64) {
        if (self.vspeed - vspeed).abs() > f64::EPSILON {
            self.vspeed = vspeed;
            self.sync_speed_direction_from_hvspeed();
        }
    }

    pub fn set_hvspeed(&mut self, hspeed: f64, vspeed: f64) {
        let h_changed = (self.hspeed - hspeed).abs() > f64::EPSILON;
        let v_changed = (self.vspeed - vspeed).abs() > f64::EPSILON;
        if h_changed || v_changed {
            self.hspeed = hspeed;
            self.vspeed = vspeed;
            self.sync_speed_direction_from_hvspeed();
        }
    }

    pub(crate) fn sync_hvspeed_from_speed_direction(&mut self) {
        let speed = self.vars.get("speed").and_then(as_number).unwrap_or(0.0);
        let direction = self
            .vars
            .get("direction")
            .and_then(as_number)
            .unwrap_or(0.0);
        let radians = direction.to_radians();
        let raw_hspeed = radians.cos() * speed;
        let raw_vspeed = -radians.sin() * speed;
        self.hspeed = gm_round(raw_hspeed);
        self.vspeed = gm_round(raw_vspeed);
    }

    fn sync_speed_direction_from_hvspeed(&mut self) {
        let direction = (-self.vspeed)
            .atan2(self.hspeed)
            .to_degrees()
            .rem_euclid(360.0);
        let speed = (self.hspeed * self.hspeed + self.vspeed * self.vspeed).sqrt();
        self.vars
            .insert("direction".into(), RuntimeValue::Number(gm_round(direction)));
        self.vars
            .insert("speed".into(), RuntimeValue::Number(speed));
    }
}

fn as_number(value: &RuntimeValue) -> Option<f64> {
    match value {
        RuntimeValue::Number(n) => Some(*n),
        RuntimeValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        RuntimeValue::Text(t) => t.parse().ok(),
    }
}

fn gm_round(value: f64) -> f64 {
    let rounded = value.round();
    if (rounded - value).abs() < GM_ROUND_TOLERANCE {
        rounded
    } else {
        value
    }
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
pub struct RuntimeRoomView {
    pub visible: bool,
    pub source_x: i32,
    pub source_y: i32,
    pub source_w: u32,
    pub source_h: u32,
    pub port_x: i32,
    pub port_y: i32,
    pub port_w: u32,
    pub port_h: u32,
    pub target: i32,
    pub hborder: i32,
    pub vborder: i32,
    pub hspeed: i32,
    pub vspeed: i32,
}

impl From<&RoomView> for RuntimeRoomView {
    fn from(value: &RoomView) -> Self {
        Self {
            visible: value.visible,
            source_x: value.source_x,
            source_y: value.source_y,
            source_w: value.source_w,
            source_h: value.source_h,
            port_x: value.port_x,
            port_y: value.port_y,
            port_w: value.port_w,
            port_h: value.port_h,
            target: value.target,
            hborder: value.hborder,
            vborder: value.vborder,
            hspeed: value.hspeed,
            vspeed: value.vspeed,
        }
    }
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
    pub views_enabled: bool,
    pub views: Vec<RuntimeRoomView>,
    pub instances: Vec<RuntimeInstance>,
}

#[derive(Debug, Clone)]
pub struct RuntimePlayerSnapshot {
    pub runtime_id: usize,
    pub instance_id: i32,
    pub object_id: usize,
    pub object_name: String,
    pub x: f64,
    pub y: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub facing_left: bool,
    pub alive: bool,
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

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeTickPhaseSnapshot {
    pub input_diag_nanos: u64,
    pub step_events_nanos: u64,
    pub view_sync_nanos: u64,
    pub player_movement_nanos: u64,
    pub collision_events_nanos: u64,
    pub alarms_nanos: u64,
    pub keyboard_events_nanos: u64,
    pub render_submit_nanos: u64,
    pub total_nanos: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub tick: u64,
    pub room_id: Option<usize>,
    pub room_name: Option<String>,
    pub room_speed: Option<u32>,
    pub instance_count: usize,
    pub player: Option<RuntimePlayerSnapshot>,
    pub input_trace: RuntimeInputTraceSnapshot,
    pub tick_phases: RuntimeTickPhaseSnapshot,
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
