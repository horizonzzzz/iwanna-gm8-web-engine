use crate::models::EngineFamily;

pub fn known_signatures() -> &'static [(EngineFamily, &'static [&'static [u8]])] {
    &[
        (
            EngineFamily::Gm8,
            &[
                b"Game Maker",
                b"Version 8",
                b"D3DX8.dll",
                b"room_goto",
                b"keyboard_check",
            ],
        ),
        (
            EngineFamily::Gms,
            &[b"data.win", b"YoYo Games", b"audiogroup"],
        ),
        (EngineFamily::Unity, &[b"UnityPlayer.dll", b"UnityEngine"]),
        (
            EngineFamily::RpgMaker,
            &[b"RPG_RT.exe", b"Game.rgss", b"www/js/plugins"],
        ),
        (EngineFamily::Clickteam, &[b"Clickteam", b"Fusion"]),
        (EngineFamily::Godot, &[b"Godot Engine", b"godot_windows"]),
        (EngineFamily::Nwjs, &[b"nw.exe", b"nw_elf.dll"]),
    ]
}

pub fn match_signals(bytes: &[u8]) -> Vec<EngineFamily> {
    let mut matched = Vec::new();
    for (family, needles) in known_signatures() {
        if needles
            .iter()
            .any(|needle| bytes.windows(needle.len()).any(|window| window == *needle))
        {
            matched.push(*family);
        }
    }
    matched.sort_by_key(|family| *family as u8);
    matched.dedup();
    matched
}

pub fn match_inventory_signals(paths: &[String]) -> Vec<EngineFamily> {
    let haystack = paths
        .iter()
        .map(|path| path.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("\n");

    let mut matched = Vec::new();

    if haystack.contains("data.win") {
        matched.push(EngineFamily::Gms);
    }
    if haystack.contains("unityplayer.dll") {
        matched.push(EngineFamily::Unity);
    }
    if haystack.contains("rpg_rt.exe")
        || haystack.contains("game.rgss")
        || haystack.contains("www/js/plugins")
    {
        matched.push(EngineFamily::RpgMaker);
    }
    if haystack.contains("nw.exe") {
        matched.push(EngineFamily::Nwjs);
    }

    matched.sort_by_key(|family| *family as u8);
    matched.dedup();
    matched
}
