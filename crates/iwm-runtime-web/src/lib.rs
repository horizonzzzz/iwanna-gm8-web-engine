use std::sync::{Mutex, OnceLock};

use iwm_runtime_core::{RuntimeCore, RuntimePackage, RuntimeSnapshot, RuntimeStatus};
use iwm_runtime_host::{HeadlessHost, RuntimeDiagnostic};
use serde::Serialize;

#[derive(Debug)]
pub struct WebRuntimeHost {
    host: HeadlessHost,
    core: Option<RuntimeCore>,
    package: Option<RuntimePackage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSnapshot {
    pub status: String,
    pub tick: u64,
    pub room_id: Option<usize>,
    pub room_name: Option<String>,
    pub instance_count: usize,
    pub diagnostics: Vec<String>,
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
        let core = RuntimeCore::load(package.clone()).map_err(format_core_error)?;
        let snapshot = bridge_snapshot(core.snapshot());
        self.core = Some(core);
        self.package = Some(package);
        self.host = HeadlessHost::new("runtime-web");
        Ok(snapshot)
    }

    pub fn boot_from_json(&mut self, package_json: &str) -> Result<BridgeSnapshot, String> {
        let package =
            serde_json::from_str::<RuntimePackage>(package_json).map_err(|error| error.to_string())?;
        self.boot(package)
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

        self.host = HeadlessHost::new("runtime-web");
        let core = RuntimeCore::load(package).map_err(format_core_error)?;
        let snapshot = bridge_snapshot(core.snapshot());
        self.core = Some(core);
        Ok(snapshot)
    }

    pub fn select_room(&mut self, room_id: usize) -> Result<BridgeSnapshot, String> {
        let Some(core) = self.core.as_mut() else {
            return Err("runtime core is not booted".into());
        };

        core.reload_room(room_id).map_err(format_core_error)?;
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

    pub fn host_frame_count(&self) -> usize {
        self.host.renderer.submitted_frames.len()
    }
}

impl Default for WebRuntimeHost {
    fn default() -> Self {
        Self::new()
    }
}

fn bridge_snapshot(snapshot: RuntimeSnapshot) -> BridgeSnapshot {
    BridgeSnapshot {
        status: status_label(snapshot.status).into(),
        tick: snapshot.tick,
        room_id: snapshot.room_id,
        room_name: snapshot.room_name,
        instance_count: snapshot.instance_count,
        diagnostics: format_diagnostics(&snapshot.diagnostics),
    }
}

fn format_diagnostics(diagnostics: &[RuntimeDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|entry| {
            format!(
                "{}:{}:{}",
                diagnostic_level_label(entry),
                entry.code,
                entry.message
            )
        })
        .collect()
}

fn diagnostic_level_label(entry: &RuntimeDiagnostic) -> &'static str {
    match entry.level {
        iwm_runtime_host::RuntimeDiagnosticLevel::Info => "info",
        iwm_runtime_host::RuntimeDiagnosticLevel::Warning => "warning",
        iwm_runtime_host::RuntimeDiagnosticLevel::Error => "error",
    }
}

fn status_label(status: RuntimeStatus) -> &'static str {
    match status {
        RuntimeStatus::Idle => "idle",
        RuntimeStatus::Ready => "ready",
        RuntimeStatus::Running => "running",
        RuntimeStatus::Error => "error",
    }
}

fn format_core_error(error: iwm_runtime_core::RuntimeCoreError) -> String {
    match error {
        iwm_runtime_core::RuntimeCoreError::NoRooms => "runtime package does not contain any rooms".into(),
        iwm_runtime_core::RuntimeCoreError::RoomMissing(room_id) => {
            format!("runtime package is missing room {}", room_id)
        }
        iwm_runtime_core::RuntimeCoreError::Host(host_error) => host_error.to_string(),
    }
}

fn runtime_host() -> &'static Mutex<WebRuntimeHost> {
    static RUNTIME: OnceLock<Mutex<WebRuntimeHost>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(WebRuntimeHost::new()))
}

fn last_result_bytes() -> &'static Mutex<Vec<u8>> {
    static LAST_RESULT: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
    LAST_RESULT.get_or_init(|| Mutex::new(Vec::new()))
}

fn store_result(result: String) -> usize {
    let mut bytes = last_result_bytes().lock().expect("last result mutex poisoned");
    *bytes = result.into_bytes();
    bytes.as_ptr() as usize
}

fn store_json_result<T: Serialize>(value: &T) -> usize {
    store_result(serde_json::to_string(value).unwrap_or_else(|error| {
        format!(r#"{{"error":"failed to encode bridge result: {}"}}"#, error)
    }))
}

fn store_error_result(message: String) -> usize {
    store_result(format!(
        r#"{{"error":"{}"}}"#,
        message.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

fn read_utf8_from_ptr(pointer: *const u8, len: usize) -> Result<String, String> {
    if pointer.is_null() {
        return Err("received null pointer for JSON payload".into());
    }

    let bytes = unsafe { std::slice::from_raw_parts(pointer, len) };
    std::str::from_utf8(bytes)
        .map(|text| text.to_owned())
        .map_err(|error| error.to_string())
}

#[no_mangle]
pub extern "C" fn iwm_alloc(len: usize) -> *mut u8 {
    let mut bytes = Vec::<u8>::with_capacity(len);
    let pointer = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    pointer
}

#[no_mangle]
pub extern "C" fn iwm_free(pointer: *mut u8, len: usize) {
    if pointer.is_null() {
        return;
    }

    unsafe {
        let _ = Vec::from_raw_parts(pointer, 0, len);
    }
}

#[no_mangle]
pub extern "C" fn iwm_last_result_len() -> usize {
    last_result_bytes()
        .lock()
        .expect("last result mutex poisoned")
        .len()
}

#[no_mangle]
pub extern "C" fn iwm_boot_json(pointer: *const u8, len: usize) -> usize {
    let package_json = match read_utf8_from_ptr(pointer, len) {
        Ok(value) => value,
        Err(error) => return store_error_result(error),
    };

    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.boot_from_json(&package_json) {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_tick(frames: u32) -> usize {
    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.tick(frames) {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_reset() -> usize {
    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.reset() {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_select_room(room_id: u32) -> usize {
    let mut host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.select_room(room_id as usize) {
        Ok(snapshot) => store_json_result(&snapshot),
        Err(error) => store_error_result(error),
    }
}

#[no_mangle]
pub extern "C" fn iwm_snapshot_json() -> usize {
    let host = runtime_host().lock().expect("runtime host mutex poisoned");
    match host.snapshot() {
        Some(snapshot) => store_json_result(&snapshot),
        None => store_error_result("runtime core is not booted".into()),
    }
}

#[no_mangle]
pub extern "C" fn iwm_diagnostics_json() -> usize {
    let host = runtime_host().lock().expect("runtime host mutex poisoned");
    store_json_result(&host.diagnostics())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use iwm_runtime_model::{
        AnalysisReport, BackgroundResource, CompatibilityLevel, LogicBlock, LogicOp, ObjectDefinition,
        ObjectEventEntry, ResourceIndex, RoomBackgroundLayer, RoomDefinition, RoomInstancePlacement, RoomView,
        RuntimeManifest, ScriptIrFile, SoundResource, SpriteResource,
    };

    fn sample_package() -> RuntimePackage {
        RuntimePackage {
            manifest: RuntimeManifest {
                format_version: 1,
                package_kind: "runtime-v1".into(),
                source_name: "sample.exe".into(),
                source_hash: "abc123".into(),
                engine_family: "gm8".into(),
                compatibility: CompatibilityLevel::Partial,
                default_room_id: Some(0),
                room_count: 2,
                object_count: 1,
                script_block_count: 1,
                sprite_count: 0,
                background_count: 0,
                sound_count: 0,
                resource_index_path: "resources/index.json".into(),
                warnings: vec![],
            },
            rooms: vec![RoomDefinition {
                id: 0,
                name: "room0".into(),
                width: 320,
                height: 240,
                speed: 60,
                persistent: false,
                backgrounds: vec![RoomBackgroundLayer {
                    visible_on_start: true,
                    is_foreground: false,
                    source_bg: 0,
                    xoffset: 0,
                    yoffset: 0,
                    tile_horz: false,
                    tile_vert: false,
                    hspeed: 0,
                    vspeed: 0,
                    stretch: false,
                }],
                views_enabled: false,
                views: vec![RoomView {
                    visible: true,
                    source_x: 0,
                    source_y: 0,
                    source_w: 320,
                    source_h: 240,
                    port_x: 0,
                    port_y: 0,
                    port_w: 320,
                    port_h: 240,
                    target: -1,
                }],
                instances: vec![RoomInstancePlacement {
                    instance_id: 1,
                    object_id: 0,
                    x: 32,
                    y: 64,
                    xscale: 1.0,
                    yscale: 1.0,
                    angle: 0.0,
                    blend: 0x00ff_ffff,
                    creation_block_id: None,
                    is_solid: false,
                    is_hazard: false,
                    is_checkpoint: false,
                }],
                creation_block_id: None,
                playable: true,
                transition_targets: vec![],
            }, RoomDefinition {
                id: 1,
                name: "room1".into(),
                width: 640,
                height: 480,
                speed: 30,
                persistent: false,
                backgrounds: vec![],
                views_enabled: false,
                views: vec![],
                instances: vec![],
                creation_block_id: None,
                playable: true,
                transition_targets: vec![],
            }],
            objects: vec![ObjectDefinition {
                id: 0,
                name: "obj_player".into(),
                sprite_index: -1,
                parent_index: -1,
                depth: 0,
                persistent: false,
                visible: true,
                solid: false,
                mask_index: -1,
                is_hazard: Some(false),
                is_checkpoint: Some(false),
                is_player: true,
                events: vec![ObjectEventEntry {
                    event_type: 0,
                    sub_event: 0,
                    event_tag: "create".into(),
                    block_id: "object:0:event:0:0".into(),
                    action_count: 0,
                }],
            }],
            scripts: ScriptIrFile {
                format: "iwm-script-ir-v1".into(),
                blocks: vec![LogicBlock {
                    id: "object:0:event:0:0".into(),
                    name: "object event".into(),
                    kind: "object-event".into(),
                    support: "source-only".into(),
                    executable_action_count: 0,
                    ops: vec![LogicOp::Unsupported {
                        reason: "placeholder".into(),
                    }],
                }],
            },
            resources: ResourceIndex {
                sprites: vec![SpriteResource {
                    id: 0,
                    name: "spr_player".into(),
                    origin_x: 0,
                    origin_y: 0,
                    frame_paths: vec![],
                    width: 0,
                    height: 0,
                }],
                backgrounds: vec![BackgroundResource {
                    id: 0,
                    name: "bg_room".into(),
                    width: 320,
                    height: 240,
                    image_path: "resources/backgrounds/0.png".into(),
                }],
                sounds: vec![SoundResource {
                    id: 0,
                    name: "snd_beep".into(),
                    file_path: "resources/audio/0.wav".into(),
                    extension: "wav".into(),
                    preload: false,
                }],
            },
            analysis: AnalysisReport {
                dlls: vec![],
                included_files: vec![],
                warnings: vec![],
                unsupported_features: vec![],
            },
        }
    }

    #[test]
    fn web_runtime_host_boots_and_ticks_headless_runtime() {
        let mut host = WebRuntimeHost::new();

        let boot = host.boot(sample_package()).unwrap();
        assert_eq!(boot.room_id, Some(0));
        assert_eq!(boot.status, "ready");

        let after_tick = host.tick(2).unwrap();
        assert_eq!(after_tick.tick, 2);
        assert_eq!(after_tick.room_id, Some(0));
        assert_eq!(host.host_frame_count(), 2);
    }

    #[test]
    fn web_runtime_host_requires_boot_before_tick() {
        let mut host = WebRuntimeHost::new();
        let error = host.tick(1).unwrap_err();

        assert!(error.contains("not booted"));
    }

    #[test]
    fn web_runtime_host_boots_from_json_payload() {
        let mut host = WebRuntimeHost::new();
        let package_json = serde_json::to_string(&sample_package()).unwrap();

        let boot = host.boot_from_json(&package_json).unwrap();

        assert_eq!(boot.tick, 0);
        assert_eq!(boot.room_id, Some(0));
        assert_eq!(boot.status, "ready");
    }

    #[test]
    fn web_runtime_host_can_select_room_and_reset() {
        let mut host = WebRuntimeHost::new();
        host.boot(sample_package()).unwrap();
        host.tick(2).unwrap();

        let selected = host.select_room(1).unwrap();
        assert_eq!(selected.room_id, Some(1));
        assert_eq!(selected.room_name.as_deref(), Some("room1"));

        let reset = host.reset().unwrap();
        assert_eq!(reset.tick, 0);
        assert_eq!(reset.room_id, Some(0));
        assert_eq!(reset.room_name.as_deref(), Some("room0"));
    }

    #[test]
    fn web_runtime_host_formats_diagnostics_for_bridge_consumers() {
        let mut host = WebRuntimeHost::new();
        host.boot(sample_package()).unwrap();
        host.tick(1).unwrap();

        let diagnostics = host.diagnostics();

        assert!(diagnostics.iter().any(|entry| entry.contains("runtime-idle")));
        assert!(json!(diagnostics).is_array());
    }
}
