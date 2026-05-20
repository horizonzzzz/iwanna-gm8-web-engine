use iwm_runtime_core::{RuntimeCore, RuntimePackage, RuntimeSnapshot};
use iwm_runtime_host::HeadlessHost;

#[derive(Debug)]
pub struct WebRuntimeHost {
    host: HeadlessHost,
    core: Option<RuntimeCore>,
}

impl WebRuntimeHost {
    pub fn new() -> Self {
        Self {
            host: HeadlessHost::new("runtime-web"),
            core: None,
        }
    }

    pub fn boot(&mut self, package: RuntimePackage) -> Result<RuntimeSnapshot, String> {
        let core = RuntimeCore::load(package).map_err(format_core_error)?;
        let snapshot = core.snapshot();
        self.core = Some(core);
        Ok(snapshot)
    }

    pub fn tick(&mut self, frames: u32) -> Result<RuntimeSnapshot, String> {
        let Some(core) = self.core.as_mut() else {
            return Err("runtime core is not booted".into());
        };

        let frame_count = frames.max(1);
        for _ in 0..frame_count {
            self.host.clock.advance_frames(1);
            core.tick(&mut self.host).map_err(format_core_error)?;
        }

        Ok(core.snapshot())
    }

    pub fn snapshot(&self) -> Option<RuntimeSnapshot> {
        self.core.as_ref().map(RuntimeCore::snapshot)
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

fn format_core_error(error: iwm_runtime_core::RuntimeCoreError) -> String {
    match error {
        iwm_runtime_core::RuntimeCoreError::NoRooms => "runtime package does not contain any rooms".into(),
        iwm_runtime_core::RuntimeCoreError::RoomMissing(room_id) => {
            format!("runtime package is missing room {}", room_id)
        }
        iwm_runtime_core::RuntimeCoreError::Host(host_error) => host_error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(boot.status, iwm_runtime_core::RuntimeStatus::Ready);

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
}
