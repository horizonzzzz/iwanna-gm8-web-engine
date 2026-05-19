use anyhow::{Context, Result};
use gm8exe::{reader, GameAssets};
use std::fs;
use std::path::Path;

pub fn read_gm8_assets(exe_path: &Path) -> Result<GameAssets> {
    let mut bytes = fs::read(exe_path)
        .with_context(|| format!("failed to read GM8 executable: {}", exe_path.display()))?;

    reader::from_exe(&mut bytes, None::<fn(&str)>, false, false)
        .with_context(|| format!("failed to parse GM8 data from: {}", exe_path.display()))
}
