use crate::models::{
    BackgroundResource, ResourceIndex, SoundResource, SpriteCollisionMask, SpriteResource,
};
use anyhow::{Context, Result};
use gm8exe::GameAssets;
use png::{BitDepth, ColorType, Encoder};
use std::fs;
use std::io::BufWriter;
use std::path::Path;

pub fn export_resources(assets: &GameAssets, output_dir: &Path) -> Result<ResourceIndex> {
    let resources_dir = output_dir.join("resources");
    let sprite_dir = resources_dir.join("sprites");
    let background_dir = resources_dir.join("backgrounds");
    let audio_dir = resources_dir.join("audio");

    fs::create_dir_all(&sprite_dir)?;
    fs::create_dir_all(&background_dir)?;
    fs::create_dir_all(&audio_dir)?;

    let sprites = assets
        .sprites
        .iter()
        .enumerate()
        .filter_map(|(id, sprite)| sprite.as_ref().map(|sprite| (id, sprite)))
        .map(|(id, sprite)| {
            let mut frame_paths = Vec::new();
            for (frame_index, frame) in sprite.frames.iter().enumerate() {
                let path = sprite_dir.join(format!("{id}-{frame_index}.png"));
                write_rgba_png(&path, frame.width, frame.height, &frame.data)?;
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
                        data: collider.data.iter().copied().collect(),
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
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ResourceIndex {
        sprites,
        backgrounds,
        sounds,
    })
}

pub fn bgra_to_rgba(input: Vec<u8>) -> Vec<u8> {
    input
        .chunks_exact(4)
        .flat_map(|chunk| [chunk[2], chunk[1], chunk[0], chunk[3]])
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
