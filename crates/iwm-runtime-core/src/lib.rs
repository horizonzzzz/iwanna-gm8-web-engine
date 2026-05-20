use std::collections::HashMap;

use iwm_parser::models::{
    AnalysisReport, ObjectDefinition, ResourceIndex, RoomDefinition, RuntimeManifest, ScriptIrFile,
};
use iwm_runtime_host::{
    Rgba8, RuntimeButton, RuntimeDrawCommand, RuntimeHost, RuntimeHostError, RuntimeRenderFrame,
};

#[derive(Debug, Clone)]
pub struct RuntimePackage {
    pub manifest: RuntimeManifest,
    pub rooms: Vec<RoomDefinition>,
    pub objects: Vec<ObjectDefinition>,
    pub scripts: ScriptIrFile,
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
    pub x: i32,
    pub y: i32,
    pub alive: bool,
    pub solid: bool,
    pub hazard: bool,
    pub checkpoint: bool,
    pub player_candidate: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeRoomState {
    pub room_id: usize,
    pub room_name: String,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
    pub playable: bool,
    pub instances: Vec<RuntimeInstance>,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub tick: u64,
    pub room_id: Option<usize>,
    pub room_name: Option<String>,
    pub instance_count: usize,
    pub diagnostics: Vec<iwm_runtime_host::RuntimeDiagnostic>,
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

#[derive(Debug)]
pub struct RuntimeCore {
    package: RuntimePackage,
    room_index: HashMap<usize, usize>,
    current_room: Option<RuntimeRoomState>,
    status: RuntimeStatus,
    tick: u64,
    diagnostics: Vec<iwm_runtime_host::RuntimeDiagnostic>,
}

impl RuntimeCore {
    pub fn load(package: RuntimePackage) -> Result<Self, RuntimeCoreError> {
        if package.rooms.is_empty() {
            return Err(RuntimeCoreError::NoRooms);
        }

        let room_index = package
            .rooms
            .iter()
            .enumerate()
            .map(|(index, room)| (room.id, index))
            .collect::<HashMap<_, _>>();

        let mut core = Self {
            package,
            room_index,
            current_room: None,
            status: RuntimeStatus::Ready,
            tick: 0,
            diagnostics: Vec::new(),
        };

        core.boot_default_room()?;
        Ok(core)
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status
    }

    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    pub fn current_room(&self) -> Option<&RuntimeRoomState> {
        self.current_room.as_ref()
    }

    pub fn diagnostics(&self) -> &[iwm_runtime_host::RuntimeDiagnostic] {
        &self.diagnostics
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            status: self.status,
            tick: self.tick,
            room_id: self.current_room.as_ref().map(|room| room.room_id),
            room_name: self
                .current_room
                .as_ref()
                .map(|room| room.room_name.clone()),
            instance_count: self
                .current_room
                .as_ref()
                .map(|room| room.instances.len())
                .unwrap_or(0),
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn boot_default_room(&mut self) -> Result<(), RuntimeCoreError> {
        let room_id = self
            .package
            .manifest
            .default_room_id
            .or_else(|| self.package.rooms.first().map(|room| room.id))
            .ok_or(RuntimeCoreError::NoRooms)?;

        self.current_room = Some(self.build_room(room_id)?);
        self.status = RuntimeStatus::Ready;
        Ok(())
    }

    pub fn tick<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
        let Some(room) = self.current_room.as_ref() else {
            self.status = RuntimeStatus::Error;
            return Err(RuntimeCoreError::NoRooms);
        };

        let left = host.button_state(RuntimeButton::Keyboard(0x25)).pressed;
        let right = host.button_state(RuntimeButton::Keyboard(0x27)).pressed;
        let jump = host.button_state(RuntimeButton::Keyboard(0x20)).pressed;

        self.tick += 1;
        self.status = RuntimeStatus::Running;

        if !left && !right && !jump {
            self.diagnostics.push(iwm_runtime_host::RuntimeDiagnostic {
                level: iwm_runtime_host::RuntimeDiagnosticLevel::Info,
                code: "runtime-idle".into(),
                message: format!("tick {} advanced without player input", self.tick),
            });
        }

        if room.instances.is_empty() {
            self.diagnostics.push(iwm_runtime_host::RuntimeDiagnostic {
                level: iwm_runtime_host::RuntimeDiagnosticLevel::Warning,
                code: "runtime-empty-room".into(),
                message: format!("room {} has no live instances", room.room_name),
            });
        }

        host.submit_frame(RuntimeRenderFrame {
            width: room.width,
            height: room.height,
            commands: vec![
                RuntimeDrawCommand::Clear {
                    colour: Rgba8 {
                        r: 12,
                        g: 16,
                        b: 22,
                        a: 255,
                    },
                },
                RuntimeDrawCommand::Present,
            ],
        })?;

        Ok(())
    }

    pub fn reload_room(&mut self, room_id: usize) -> Result<(), RuntimeCoreError> {
        self.current_room = Some(self.build_room(room_id)?);
        self.status = RuntimeStatus::Ready;
        Ok(())
    }

    fn build_room(&self, room_id: usize) -> Result<RuntimeRoomState, RuntimeCoreError> {
        let room = self
            .room_index
            .get(&room_id)
            .and_then(|index| self.package.rooms.get(*index))
            .ok_or(RuntimeCoreError::RoomMissing(room_id))?;

        let instances = room
            .instances
            .iter()
            .enumerate()
            .filter_map(|(runtime_id, instance)| {
                let object = self.package.objects.get(instance.object_id as usize)?;
                Some(RuntimeInstance {
                    runtime_id,
                    instance_id: instance.instance_id,
                    object_id: instance.object_id as usize,
                    object_name: object.name.clone(),
                    x: instance.x,
                    y: instance.y,
                    alive: true,
                    solid: instance.is_solid || object.solid,
                    hazard: instance.is_hazard || object.is_hazard.unwrap_or(false),
                    checkpoint: instance.is_checkpoint || object.is_checkpoint.unwrap_or(false),
                    player_candidate: object.is_player,
                })
            })
            .collect::<Vec<_>>();

        Ok(RuntimeRoomState {
            room_id: room.id,
            room_name: room.name.clone(),
            width: room.width,
            height: room.height,
            speed: room.speed,
            playable: room.playable,
            instances,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iwm_parser::models::{
        AnalysisReport, BackgroundResource, CompatibilityLevel, LogicBlock, LogicOp,
        ObjectEventEntry, ResourceIndex, RoomBackgroundLayer, RoomInstancePlacement, RoomView,
        RuntimeManifest, ScriptIrFile, SoundResource, SpriteResource,
    };
    use iwm_runtime_host::{HeadlessHost, RuntimeDiagnosticLevel};

    fn sample_package() -> RuntimePackage {
        RuntimePackage {
            manifest: RuntimeManifest {
                format_version: 1,
                package_kind: "runtime-v1".into(),
                source_name: "sample.exe".into(),
                source_hash: "abc123".into(),
                engine_family: "gm8".into(),
                compatibility: CompatibilityLevel::Partial,
                default_room_id: Some(7),
                room_count: 1,
                object_count: 1,
                script_block_count: 1,
                sprite_count: 0,
                background_count: 0,
                sound_count: 0,
                resource_index_path: "resources/index.json".into(),
                warnings: vec![],
            },
            rooms: vec![RoomDefinition {
                id: 7,
                name: "room7".into(),
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
                    instance_id: 11,
                    object_id: 0,
                    x: 12,
                    y: 24,
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
    fn core_loads_default_room_and_instances() {
        let core = RuntimeCore::load(sample_package()).unwrap();

        assert_eq!(core.status(), RuntimeStatus::Ready);
        assert_eq!(core.current_room().map(|room| room.room_id), Some(7));
        assert_eq!(
            core.current_room().map(|room| room.instances.len()),
            Some(1)
        );
        assert!(core.current_room().unwrap().instances[0].player_candidate);
    }

    #[test]
    fn core_ticks_and_submits_a_frame() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        core.tick(&mut host).unwrap();

        assert_eq!(core.status(), RuntimeStatus::Running);
        assert_eq!(core.tick_count(), 1);
        assert_eq!(host.renderer.submitted_frames.len(), 1);
        assert!(host.renderer.submitted_frames[0]
            .commands
            .iter()
            .any(|command| matches!(command, RuntimeDrawCommand::Present)));
    }

    #[test]
    fn core_reports_missing_room() {
        let mut package = sample_package();
        package.manifest.default_room_id = Some(99);

        let error = RuntimeCore::load(package).unwrap_err();
        assert!(matches!(error, RuntimeCoreError::RoomMissing(99)));
    }

    #[test]
    fn core_emits_idle_diagnostic_when_no_input_is_active() {
        let mut core = RuntimeCore::load(sample_package()).unwrap();
        let mut host = HeadlessHost::new("sandbox");

        core.tick(&mut host).unwrap();

        assert!(core
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "runtime-idle"
                && matches!(diagnostic.level, RuntimeDiagnosticLevel::Info)));
    }
}
