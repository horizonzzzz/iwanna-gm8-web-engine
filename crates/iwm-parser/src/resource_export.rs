use crate::models::{
    BackgroundResource, FontGlyphResource, FontResource, LoweredLogicExpr, LoweredLogicFile,
    LoweredLogicStatement, PathPointResource, PathResource, ResourceIndex, SoundResource,
    SpriteCollisionMask, SpriteResource,
};
use anyhow::{Context, Result};
use gm8exe::GameAssets;
use png::{BitDepth, ColorType, Encoder};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufWriter, Read};
use std::path::{Path, PathBuf};

pub fn export_resources(assets: &GameAssets, output_dir: &Path) -> Result<ResourceIndex> {
    let resources_dir = output_dir.join("resources");
    let sprite_dir = resources_dir.join("sprites");
    let background_dir = resources_dir.join("backgrounds");
    let audio_dir = resources_dir.join("audio");
    let font_dir = resources_dir.join("fonts");

    fs::create_dir_all(&sprite_dir)?;
    fs::create_dir_all(&background_dir)?;
    fs::create_dir_all(&audio_dir)?;
    fs::create_dir_all(&font_dir)?;

    let sprites = assets
        .sprites
        .iter()
        .enumerate()
        .filter_map(|(id, sprite)| sprite.as_ref().map(|sprite| (id, sprite)))
        .map(|(id, sprite)| {
            let mut frame_paths = Vec::new();
            for (frame_index, frame) in sprite.frames.iter().enumerate() {
                let path = sprite_dir.join(format!("{id}-{frame_index}.png"));
                let rgba = bgra_to_rgba(frame.data.to_vec());
                write_rgba_png(&path, frame.width, frame.height, &rgba)?;
                frame_paths.push(relative_path(output_dir, &path)?);
            }

            let (width, height) = sprite
                .frames
                .first()
                .map(|frame| (frame.width, frame.height))
                .unwrap_or((0, 0));

            Ok(SpriteResource {
                id,
                name: sprite.name.to_string(),
                origin_x: sprite.origin_x,
                origin_y: sprite.origin_y,
                frame_paths,
                width,
                height,
                bbox_left: sprite
                    .colliders
                    .iter()
                    .map(|collider| collider.bbox_left)
                    .min()
                    .unwrap_or(0),
                bbox_right: sprite
                    .colliders
                    .iter()
                    .map(|collider| collider.bbox_right)
                    .max()
                    .unwrap_or(width.saturating_sub(1)),
                bbox_top: sprite
                    .colliders
                    .iter()
                    .map(|collider| collider.bbox_top)
                    .min()
                    .unwrap_or(0),
                bbox_bottom: sprite
                    .colliders
                    .iter()
                    .map(|collider| collider.bbox_bottom)
                    .max()
                    .unwrap_or(height.saturating_sub(1)),
                collision_masks: sprite
                    .colliders
                    .iter()
                    .map(|collider| SpriteCollisionMask {
                        width: collider.width,
                        height: collider.height,
                        bbox_left: collider.bbox_left,
                        bbox_right: collider.bbox_right,
                        bbox_top: collider.bbox_top,
                        bbox_bottom: collider.bbox_bottom,
                        data: collider.data.to_vec(),
                    })
                    .collect(),
                per_frame_collision_masks: sprite.per_frame_colliders,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let backgrounds = assets
        .backgrounds
        .iter()
        .enumerate()
        .filter_map(|(id, background)| background.as_ref().map(|background| (id, background)))
        .map(|(id, background)| {
            let path = background_dir.join(format!("{id}.png"));
            if let Some(data) = &background.data {
                let rgba = bgra_to_rgba(data.to_vec());
                write_rgba_png(&path, background.width, background.height, &rgba)?;
            }

            Ok(BackgroundResource {
                id,
                name: background.name.to_string(),
                width: background.width,
                height: background.height,
                image_path: relative_path(output_dir, &path)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let sounds = assets
        .sounds
        .iter()
        .enumerate()
        .filter_map(|(id, sound)| sound.as_ref().map(|sound| (id, sound)))
        .filter_map(|(id, sound)| sound.data.as_ref().map(|data| (id, sound, data)))
        .map(|(id, sound, data)| {
            let extension = sound.extension.to_string();
            let path = audio_dir.join(format!("{id}.{}", extension.trim_start_matches('.')));
            fs::write(&path, data)
                .with_context(|| format!("failed to write {}", path.display()))?;

            Ok(SoundResource {
                id,
                name: sound.name.to_string(),
                file_path: relative_path(output_dir, &path)?,
                extension,
                preload: sound.preload,
                kind: match sound.kind {
                    gm8exe::asset::sound::SoundKind::BackgroundMusic => "background-music",
                    gm8exe::asset::sound::SoundKind::ThreeDimensional => "three-dimensional",
                    gm8exe::asset::sound::SoundKind::Multimedia => "multimedia",
                    gm8exe::asset::sound::SoundKind::Normal => "normal",
                }
                .into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let fonts = assets
        .fonts
        .iter()
        .enumerate()
        .filter_map(|(id, font)| font.as_ref().map(|font| (id, font)))
        .map(|(id, font)| {
            let path = font_dir.join(format!("{id}.png"));
            let rgba = font_alpha_to_rgba(&font.pixel_map, font.map_width, font.map_height);
            write_rgba_png(&path, font.map_width, font.map_height, &rgba)?;

            Ok(FontResource {
                id,
                name: font.name.to_string(),
                system_name: font.sys_name.to_string(),
                size: font.size,
                bold: font.bold,
                italic: font.italic,
                range_start: font.range_start,
                range_end: font.range_end,
                map_width: font.map_width,
                map_height: font.map_height,
                image_path: relative_path(output_dir, &path)?,
                glyphs: font_glyphs(&font.dmap),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let paths = assets
        .paths
        .iter()
        .enumerate()
        .filter_map(|(id, path)| path.as_ref().map(|path| (id, path)))
        .map(|(id, path)| PathResource {
            id,
            name: path.name.to_string(),
            smooth: path.connection == gm8exe::asset::path::ConnectionKind::SmoothCurve,
            precision: path.precision,
            closed: path.closed,
            points: path
                .points
                .iter()
                .map(|point| PathPointResource {
                    x: point.x,
                    y: point.y,
                    speed: point.speed,
                })
                .collect(),
        })
        .collect();

    Ok(ResourceIndex {
        sprites,
        backgrounds,
        sounds,
        fonts,
        paths,
    })
}

pub fn export_external_audio_sidecars(
    input_root: &Path,
    output_dir: &Path,
    logic: &LoweredLogicFile,
    resources: &mut ResourceIndex,
) -> Result<Vec<String>> {
    let references = referenced_audio_paths(logic);
    if references.is_empty() {
        return Ok(Vec::new());
    }

    let sidecars = inventory_audio_sidecars(input_root)?;
    let mut exact = HashMap::new();
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, sidecar) in sidecars.iter().enumerate() {
        exact.insert(sidecar.key.as_str(), index);
        if let Some(name) = sidecar.key.rsplit('/').next() {
            by_name.entry(name).or_default().push(index);
        }
    }

    let mut references = references.into_iter().collect::<Vec<_>>();
    references.sort();
    let mut exported = HashSet::new();
    let mut warnings = Vec::new();
    let mut next_id = resources
        .sounds
        .iter()
        .map(|sound| sound.id)
        .max()
        .map_or(0, |id| id + 1);
    let audio_dir = output_dir.join("resources").join("audio");
    fs::create_dir_all(&audio_dir)?;

    for reference in references {
        let candidates = exact
            .get(reference.as_str())
            .map(std::slice::from_ref)
            .or_else(|| {
                reference
                    .rsplit('/')
                    .next()
                    .and_then(|name| by_name.get(name).map(Vec::as_slice))
            });
        let Some([index]) = candidates else {
            warnings.push(format!("external-audio-unresolved:{reference}"));
            continue;
        };
        let sidecar = &sidecars[*index];
        if !exported.insert(sidecar.path.clone()) {
            continue;
        }
        if !has_supported_audio_header(&sidecar.path, &sidecar.extension)? {
            warnings.push(format!("external-audio-invalid:{}", sidecar.key));
            continue;
        }

        let destination = audio_dir.join(format!("external-{next_id}.{}", sidecar.extension));
        fs::copy(&sidecar.path, &destination).with_context(|| {
            format!(
                "failed to copy external audio {} to {}",
                sidecar.path.display(),
                destination.display()
            )
        })?;
        resources.sounds.push(SoundResource {
            id: next_id,
            name: sidecar.key.clone(),
            file_path: relative_path(output_dir, &destination)?,
            extension: sidecar.extension.clone(),
            preload: false,
            kind: "normal".into(),
        });
        next_id += 1;
    }

    Ok(warnings)
}

struct AudioSidecar {
    path: PathBuf,
    key: String,
    extension: String,
}

fn inventory_audio_sidecars(root: &Path) -> Result<Vec<AudioSidecar>> {
    let mut directories = vec![root.to_path_buf()];
    let mut sidecars = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory)
            .with_context(|| format!("failed to inspect {}", directory.display()))?
        {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(entry.path());
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(extension) = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .filter(|extension| matches!(extension.as_str(), "ogg" | "wav" | "mp3"))
            else {
                continue;
            };
            let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
            let Some(key) = normalize_audio_path(&relative) else {
                continue;
            };
            sidecars.push(AudioSidecar {
                path,
                key,
                extension,
            });
        }
    }
    Ok(sidecars)
}

fn referenced_audio_paths(logic: &LoweredLogicFile) -> HashSet<String> {
    let mut references = HashSet::new();
    let mut statements = logic
        .entries
        .iter()
        .flat_map(|entry| entry.statements.iter())
        .collect::<Vec<_>>();
    let mut expressions = Vec::new();

    while let Some(statement) = statements.pop() {
        match statement {
            LoweredLogicStatement::Assignment { target, value } => {
                expressions.extend([target, value]);
            }
            LoweredLogicStatement::Return { value } => expressions.extend(value),
            LoweredLogicStatement::FunctionCall { args, .. } => expressions.extend(args),
            LoweredLogicStatement::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                expressions.push(condition);
                statements.extend(then_branch);
                statements.extend(else_branch);
            }
            LoweredLogicStatement::ConditionalChain {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    expressions.push(&branch.condition);
                    statements.extend(&branch.body);
                }
                statements.extend(else_branch);
            }
            LoweredLogicStatement::Switch { expression, cases } => {
                expressions.push(expression);
                for switch_case in cases {
                    expressions.extend(switch_case.value.as_ref());
                    statements.extend(&switch_case.body);
                }
            }
            LoweredLogicStatement::With { target, body } => {
                expressions.push(target);
                statements.extend(body);
            }
            LoweredLogicStatement::Repeat { count, body } => {
                expressions.push(count);
                statements.extend(body);
            }
            LoweredLogicStatement::While { condition, body } => {
                expressions.push(condition);
                statements.extend(body);
            }
            LoweredLogicStatement::For {
                init,
                condition,
                step,
                body,
            } => {
                expressions.extend([init, condition, step]);
                statements.extend(body);
            }
            LoweredLogicStatement::VariableDeclaration { .. }
            | LoweredLogicStatement::Raw { .. } => {}
        }
    }

    while let Some(expr) = expressions.pop() {
        match expr {
            LoweredLogicExpr::LiteralText(value) => {
                if let Some(path) = normalize_audio_path(value) {
                    references.insert(path);
                }
            }
            LoweredLogicExpr::UnaryExpr { child, .. } => expressions.push(child),
            LoweredLogicExpr::Call { args, .. } => expressions.extend(args),
            LoweredLogicExpr::MemberAccess { target, .. } => expressions.push(target),
            LoweredLogicExpr::IndexAccess { target, index } => {
                expressions.extend([target.as_ref(), index.as_ref()]);
            }
            LoweredLogicExpr::BinaryExpr { left, right, .. } => {
                expressions.extend([left.as_ref(), right.as_ref()]);
            }
            LoweredLogicExpr::Identifier(_)
            | LoweredLogicExpr::LiteralNumber(_)
            | LoweredLogicExpr::LiteralBool(_)
            | LoweredLogicExpr::Raw { .. } => {}
        }
    }

    references
}

fn normalize_audio_path(value: &str) -> Option<String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.contains(':') {
        return None;
    }
    let mut parts = Vec::new();
    for part in normalized.trim_start_matches('/').split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            _ => parts.push(part),
        }
    }
    let path = parts.join("/").to_ascii_lowercase();
    let extension = path.rsplit_once('.')?.1;
    let stem = path.rsplit('/').next()?.rsplit_once('.')?.0;
    (!stem.is_empty() && matches!(extension, "ogg" | "wav" | "mp3")).then_some(path)
}

fn has_supported_audio_header(path: &Path, extension: &str) -> Result<bool> {
    let mut header = [0u8; 12];
    let read = fs::File::open(path)?.read(&mut header)?;
    Ok(match extension {
        "ogg" => read >= 4 && &header[..4] == b"OggS",
        "wav" => read >= 12 && &header[..4] == b"RIFF" && &header[8..12] == b"WAVE",
        "mp3" => {
            (read >= 3 && &header[..3] == b"ID3")
                || (read >= 2 && header[0] == 0xff && header[1] & 0xe0 == 0xe0)
        }
        _ => false,
    })
}

pub fn bgra_to_rgba(input: Vec<u8>) -> Vec<u8> {
    input
        .chunks_exact(4)
        .flat_map(|chunk| [chunk[2], chunk[1], chunk[0], chunk[3]])
        .collect()
}

fn font_alpha_to_rgba(input: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width as usize).saturating_mul(height as usize);
    (0..pixel_count)
        .flat_map(|index| [255, 255, 255, input.get(index).copied().unwrap_or(0)])
        .collect()
}

fn font_glyphs(dmap: &[u32; 0x600]) -> Vec<FontGlyphResource> {
    (0..256)
        .map(|code| {
            let index = code * 6;
            // GM8/OpenGMK draws at dmap[5] + cursor and advances cursor by dmap[4].
            FontGlyphResource {
                code: code as u32,
                x: dmap[index],
                y: dmap[index + 1],
                width: dmap[index + 2],
                height: dmap[index + 3],
                offset: dmap[index + 5] as i32,
                advance: dmap[index + 4] as i32,
            }
        })
        .collect()
}

fn relative_path(output_dir: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(output_dir)
        .with_context(|| format!("{} is not under {}", path.display(), output_dir.display()))?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn write_rgba_png(path: &Path, width: u32, height: u32, bytes: &[u8]) -> Result<()> {
    let file =
        fs::File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(bytes)?;
    Ok(())
}
