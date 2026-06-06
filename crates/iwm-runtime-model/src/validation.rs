use std::{
    collections::HashSet,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    AnalysisReport, LoweredLogicFile, ObjectDefinition, RawLogicFile, ResourceIndex,
    RoomDefinition, RuntimeManifest, ScriptIrFile,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePackageContract {
    pub manifest: RuntimeManifest,
    pub rooms: Vec<RoomDefinition>,
    pub objects: Vec<ObjectDefinition>,
    pub scripts: ScriptIrFile,
    pub raw_logic: RawLogicFile,
    pub lowered_logic: LoweredLogicFile,
    pub analysis: AnalysisReport,
    pub resources: ResourceIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePackageValidationReport {
    pub valid: bool,
    pub errors: Vec<RuntimePackageValidationError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RuntimePackageValidationError {
    ManifestCountMismatch {
        field: String,
        expected: usize,
        actual: usize,
    },
    MissingDefaultRoom {
        room_id: usize,
    },
    MissingRoomOrderRoom {
        room_id: usize,
    },
    MissingRoomInstanceObject {
        room_id: usize,
        instance_id: i32,
        object_id: i32,
    },
    MissingRoomBackground {
        room_id: usize,
        source_bg: i32,
    },
    MissingRoomTileBackground {
        room_id: usize,
        tile_id: i32,
        source_bg: i32,
    },
    MissingTransitionRoom {
        room_id: usize,
        target_room_id: usize,
    },
    MissingObjectSprite {
        object_id: usize,
        sprite_index: i32,
    },
    MissingObjectMask {
        object_id: usize,
        mask_index: i32,
    },
    MissingObjectParent {
        object_id: usize,
        parent_index: i32,
    },
    MissingRawLogicObject {
        block_id: String,
        object_id: usize,
    },
    MissingLogicBlock {
        owner: String,
        block_id: String,
        missing_from: String,
    },
}

pub fn validate_runtime_package(
    package: &RuntimePackageContract,
) -> RuntimePackageValidationReport {
    let mut errors = Vec::new();

    validate_manifest_counts(package, &mut errors);

    let room_ids = package
        .rooms
        .iter()
        .map(|room| room.id)
        .collect::<HashSet<_>>();
    let object_ids = package
        .objects
        .iter()
        .map(|object| object.id)
        .collect::<HashSet<_>>();
    let sprite_ids = package
        .resources
        .sprites
        .iter()
        .map(|sprite| sprite.id)
        .collect::<HashSet<_>>();
    let background_ids = package
        .resources
        .backgrounds
        .iter()
        .map(|background| background.id)
        .collect::<HashSet<_>>();
    let script_block_ids = package
        .scripts
        .blocks
        .iter()
        .map(|block| block.id.as_str())
        .collect::<HashSet<_>>();
    let lowered_block_ids = package
        .lowered_logic
        .entries
        .iter()
        .map(|entry| entry.block_id.as_str())
        .collect::<HashSet<_>>();
    let raw_block_ids = raw_logic_block_ids(&package.raw_logic);

    if let Some(room_id) = package.manifest.default_room_id {
        if !room_ids.contains(&room_id) {
            errors.push(RuntimePackageValidationError::MissingDefaultRoom { room_id });
        }
    }

    for room_id in &package.manifest.room_order {
        if !room_ids.contains(room_id) {
            errors.push(RuntimePackageValidationError::MissingRoomOrderRoom { room_id: *room_id });
        }
    }

    for room in &package.rooms {
        if let Some(block_id) = &room.creation_block_id {
            require_logic_block(
                &mut errors,
                &script_block_ids,
                &raw_block_ids,
                &lowered_block_ids,
                format!("room:{}", room.id),
                block_id,
            );
        }

        for background in &room.backgrounds {
            if background.visible_on_start
                && background.source_bg >= 0
                && !background_ids.contains(&(background.source_bg as usize))
            {
                errors.push(RuntimePackageValidationError::MissingRoomBackground {
                    room_id: room.id,
                    source_bg: background.source_bg,
                });
            }
        }

        for tile in &room.tiles {
            if tile.source_bg >= 0 && !background_ids.contains(&(tile.source_bg as usize)) {
                errors.push(RuntimePackageValidationError::MissingRoomTileBackground {
                    room_id: room.id,
                    tile_id: tile.tile_id,
                    source_bg: tile.source_bg,
                });
            }
        }

        for target_room_id in &room.transition_targets {
            if !room_ids.contains(target_room_id) {
                errors.push(RuntimePackageValidationError::MissingTransitionRoom {
                    room_id: room.id,
                    target_room_id: *target_room_id,
                });
            }
        }

        for instance in &room.instances {
            if instance.object_id < 0 || !object_ids.contains(&(instance.object_id as usize)) {
                errors.push(RuntimePackageValidationError::MissingRoomInstanceObject {
                    room_id: room.id,
                    instance_id: instance.instance_id,
                    object_id: instance.object_id,
                });
            }

            if let Some(block_id) = &instance.creation_block_id {
                require_logic_block(
                    &mut errors,
                    &script_block_ids,
                    &raw_block_ids,
                    &lowered_block_ids,
                    format!("room:{}:instance:{}", room.id, instance.instance_id),
                    block_id,
                );
            }
        }
    }

    for object in &package.objects {
        if object.sprite_index >= 0 && !sprite_ids.contains(&(object.sprite_index as usize)) {
            errors.push(RuntimePackageValidationError::MissingObjectSprite {
                object_id: object.id,
                sprite_index: object.sprite_index,
            });
        }

        if object.mask_index >= 0 && !sprite_ids.contains(&(object.mask_index as usize)) {
            errors.push(RuntimePackageValidationError::MissingObjectMask {
                object_id: object.id,
                mask_index: object.mask_index,
            });
        }

        if object.parent_index >= 0 && !object_ids.contains(&(object.parent_index as usize)) {
            errors.push(RuntimePackageValidationError::MissingObjectParent {
                object_id: object.id,
                parent_index: object.parent_index,
            });
        }

        for event in &object.events {
            require_logic_block(
                &mut errors,
                &script_block_ids,
                &raw_block_ids,
                &lowered_block_ids,
                event.block_id.clone(),
                &event.block_id,
            );
        }
    }

    for raw_event in &package.raw_logic.object_events {
        if !object_ids.contains(&raw_event.object_id) {
            errors.push(RuntimePackageValidationError::MissingRawLogicObject {
                block_id: raw_event.block_id.clone(),
                object_id: raw_event.object_id,
            });
        }
    }

    RuntimePackageValidationReport {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn read_runtime_package_dir(
    root: impl AsRef<Path>,
) -> Result<RuntimePackageContract, RuntimePackageReadError> {
    let root = root.as_ref();
    let manifest: RuntimeManifest = read_json(root.join("manifest.json"))?;

    Ok(RuntimePackageContract {
        rooms: read_json(root.join("rooms.json"))?,
        objects: read_json(root.join("objects.json"))?,
        scripts: read_json(root.join("scripts.ir.json"))?,
        raw_logic: read_json(root.join("logic.raw.json"))?,
        lowered_logic: read_json(root.join("logic.lowered.json"))?,
        analysis: read_json(root.join("analysis.json"))?,
        resources: read_json(root.join(&manifest.resource_index_path))?,
        manifest,
    })
}

#[derive(Debug)]
pub enum RuntimePackageReadError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for RuntimePackageReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimePackageReadError::Io { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            RuntimePackageReadError::Json { path, source } => {
                write!(f, "failed to parse {}: {source}", path.display())
            }
        }
    }
}

impl Error for RuntimePackageReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RuntimePackageReadError::Io { source, .. } => Some(source),
            RuntimePackageReadError::Json { source, .. } => Some(source),
        }
    }
}

fn read_json<T: DeserializeOwned>(path: PathBuf) -> Result<T, RuntimePackageReadError> {
    let bytes = fs::read(&path).map_err(|source| RuntimePackageReadError::Io {
        path: path.clone(),
        source,
    })?;

    serde_json::from_slice(&bytes).map_err(|source| RuntimePackageReadError::Json { path, source })
}

fn validate_manifest_counts(
    package: &RuntimePackageContract,
    errors: &mut Vec<RuntimePackageValidationError>,
) {
    push_count_error(
        errors,
        "room_count",
        package.manifest.room_count,
        package.rooms.len(),
    );
    push_count_error(
        errors,
        "object_count",
        package.manifest.object_count,
        package.objects.len(),
    );
    push_count_error(
        errors,
        "script_block_count",
        package.manifest.script_block_count,
        package.scripts.blocks.len(),
    );
    push_count_error(
        errors,
        "sprite_count",
        package.manifest.sprite_count,
        package.resources.sprites.len(),
    );
    push_count_error(
        errors,
        "background_count",
        package.manifest.background_count,
        package.resources.backgrounds.len(),
    );
    push_count_error(
        errors,
        "sound_count",
        package.manifest.sound_count,
        package.resources.sounds.len(),
    );
}

fn push_count_error(
    errors: &mut Vec<RuntimePackageValidationError>,
    field: &str,
    expected: usize,
    actual: usize,
) {
    if expected != actual {
        errors.push(RuntimePackageValidationError::ManifestCountMismatch {
            field: field.into(),
            expected,
            actual,
        });
    }
}

fn raw_logic_block_ids(raw_logic: &RawLogicFile) -> HashSet<&str> {
    let mut ids = HashSet::new();

    for owner in &raw_logic.room_creation_codes {
        ids.insert(owner.block_id.as_str());
    }
    for owner in &raw_logic.instance_creation_codes {
        ids.insert(owner.block_id.as_str());
    }
    for event in &raw_logic.object_events {
        ids.insert(event.block_id.as_str());
    }

    ids
}

fn require_logic_block(
    errors: &mut Vec<RuntimePackageValidationError>,
    script_block_ids: &HashSet<&str>,
    raw_block_ids: &HashSet<&str>,
    lowered_block_ids: &HashSet<&str>,
    owner: String,
    block_id: &str,
) {
    for (file_name, ids) in [
        ("scripts.ir.json", script_block_ids),
        ("logic.raw.json", raw_block_ids),
        ("logic.lowered.json", lowered_block_ids),
    ] {
        if !ids.contains(block_id) {
            errors.push(RuntimePackageValidationError::MissingLogicBlock {
                owner: owner.clone(),
                block_id: block_id.into(),
                missing_from: file_name.into(),
            });
        }
    }
}
