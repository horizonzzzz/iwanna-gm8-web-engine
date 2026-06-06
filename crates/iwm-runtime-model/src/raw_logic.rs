use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogicFile {
    pub format: String,
    pub room_creation_codes: Vec<RawLogicOwner>,
    pub instance_creation_codes: Vec<RawLogicOwner>,
    pub object_events: Vec<RawLogicEventBinding>,
    pub scripts: Vec<RawLogicScript>,
    pub triggers: Vec<RawLogicTrigger>,
    pub timelines: Vec<RawLogicTimelineMoment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RawLogicOwnerKind {
    Room,
    RoomInstance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogicOwner {
    pub owner_kind: RawLogicOwnerKind,
    pub owner_id: i32,
    pub owner_name: String,
    pub event_type: Option<usize>,
    pub sub_event: Option<u32>,
    pub collision_object_id: Option<i32>,
    pub block_id: String,
    pub gml_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogicEventBinding {
    pub object_id: usize,
    pub object_name: String,
    pub event_type: usize,
    pub sub_event: u32,
    pub event_tag: String,
    pub collision_object_id: Option<i32>,
    pub block_id: String,
    pub actions: Vec<RawCodeAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCodeAction {
    pub action_id: u32,
    pub lib_id: u32,
    pub action_kind: u32,
    pub execution_type: u32,
    pub fn_name: String,
    pub fn_code: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogicScript {
    pub script_id: usize,
    pub script_name: String,
    pub gml_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogicTrigger {
    pub trigger_id: usize,
    pub trigger_name: String,
    pub constant_name: String,
    pub moment: String,
    pub condition_gml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogicTimelineMoment {
    pub timeline_id: usize,
    pub timeline_name: String,
    pub moment: u32,
    pub actions: Vec<RawCodeAction>,
}
