use crate::models::{
    BackgroundResource, FontGlyphResource, FontResource, ResourceIndex, SoundResource,
    SpriteCollisionMask, SpriteResource,
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

    Ok(ResourceIndex {
        sprites,
        backgrounds,
        sounds,
        fonts,
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
