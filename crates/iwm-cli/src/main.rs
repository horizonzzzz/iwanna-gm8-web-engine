use clap::{Parser, Subcommand};
use iwm_detector::{detect_input, load_package, selected_executable};
use iwm_parser::build_package;
use iwm_runtime_core::{RuntimeCore, RuntimePackage};
use iwm_runtime_host::{ButtonState, HeadlessHost, RuntimeButton, RuntimeDiagnostic};
use iwm_runtime_model::{read_runtime_package_dir, validate_runtime_package};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "iwm-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Detect {
        #[arg(long)]
        input: PathBuf,
    },
    BuildPackage {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ValidatePackage {
        #[arg(long)]
        input: PathBuf,
    },
    RuntimeDiagnostics {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value_t = 600)]
        ticks: u32,
        #[arg(long, value_delimiter = ',')]
        press_keys: Vec<u16>,
        #[arg(long, value_delimiter = ',')]
        hold_keys: Vec<u16>,
        #[arg(long)]
        select_room: Option<usize>,
        #[arg(long, default_value_t = 0)]
        preselect_ticks: u32,
        #[arg(long, default_value_t = false)]
        trace_player: bool,
        #[arg(long, default_value_t = 1)]
        trace_every: u32,
        #[arg(long)]
        trace_output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect { input } => match detect_input(&input) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        },
        Commands::BuildPackage { input, output } => {
            let package = match load_package(&input) {
                Ok(package) => package,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };

            let report = match detect_input(&input) {
                Ok(report) => report,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };

            if report.verdict != iwm_detector::DetectionVerdict::Gm8Likely {
                eprintln!("input is not classified as gm8-likely");
                std::process::exit(2);
            }

            let exe = match selected_executable(&package) {
                Ok(exe) => exe,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(2);
                }
            };

            if let Err(err) = build_package(exe, &output, &report.dlls) {
                eprintln!("{err:#}");
                std::process::exit(1);
            }
        }
        Commands::ValidatePackage { input } => {
            let package = match read_runtime_package_dir(&input) {
                Ok(package) => package,
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };
            let report = validate_runtime_package(&package);
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            if !report.valid {
                std::process::exit(2);
            }
        }
        Commands::RuntimeDiagnostics {
            input,
            ticks,
            press_keys,
            hold_keys,
            select_room,
            preselect_ticks,
            trace_player,
            trace_every,
            trace_output,
        } => {
            if trace_player && trace_every == 0 {
                eprintln!("--trace-every must be greater than 0");
                std::process::exit(2);
            }
            let package = match read_runtime_package_dir(&input) {
                Ok(package) => {
                    let lowered_logic = match serde_json::to_value(package.lowered_logic)
                        .and_then(serde_json::from_value)
                    {
                        Ok(lowered_logic) => lowered_logic,
                        Err(err) => {
                            eprintln!("failed to convert lowered runtime logic: {err}");
                            std::process::exit(1);
                        }
                    };
                    RuntimePackage {
                        manifest: package.manifest,
                        rooms: package.rooms,
                        objects: package.objects,
                        scripts: package.scripts,
                        lowered_logic: Some(lowered_logic),
                        resources: package.resources,
                        analysis: package.analysis,
                    }
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            };
            let mut core = match RuntimeCore::load(package) {
                Ok(core) => core,
                Err(err) => {
                    eprintln!("failed to boot runtime core: {err:?}");
                    std::process::exit(1);
                }
            };
            let mut host = HeadlessHost::new("runtime-diagnostics");
            for _ in 0..preselect_ticks {
                apply_cli_input(&mut host, core.tick_count(), &[], &[]);
                if let Err(err) = core.tick(&mut host) {
                    eprintln!(
                        "runtime preselect tick failed at tick {}: {err:?}",
                        core.tick_count()
                    );
                    std::process::exit(1);
                }
                host.input.clear_transitions();
            }
            if let Some(room_id) = select_room {
                if let Err(err) = core.reload_room(room_id) {
                    eprintln!("failed to select room {room_id}: {err:?}");
                    std::process::exit(1);
                }
                if let Err(err) = core.render(&mut host) {
                    eprintln!("failed to settle selected room {room_id}: {err:?}");
                    std::process::exit(1);
                }
            }
            let mut seen_messages = HashSet::new();
            let mut blockers: HashMap<String, RuntimeBlockerSummary> = HashMap::new();
            let mut player_trace = Vec::new();

            if trace_player {
                maybe_collect_player_trace(&core, trace_every, &mut player_trace);
            }

            for run_tick in 0..ticks {
                apply_cli_input(&mut host, u64::from(run_tick), &press_keys, &hold_keys);
                if let Err(err) = core.tick(&mut host) {
                    eprintln!("runtime tick failed at tick {}: {err:?}", core.tick_count());
                    std::process::exit(1);
                }
                collect_runtime_blockers(core.diagnostics(), &mut seen_messages, &mut blockers);
                if trace_player {
                    maybe_collect_player_trace(&core, trace_every, &mut player_trace);
                }
                host.input.clear_transitions();
            }

            let mut ranked = blockers.into_values().collect::<Vec<_>>();
            ranked.sort_by(|left, right| {
                right
                    .count
                    .cmp(&left.count)
                    .then_with(|| left.key.cmp(&right.key))
            });

            let mut output = json!({
                "ticks": ticks,
                "current_room": core.snapshot().room_name,
                "current_room_id": core.snapshot().room_id,
                "current_tick": core.tick_count(),
                "runtime_blockers": ranked,
            });
            if trace_player {
                output["trace_every"] = json!(trace_every);
                output["player_trace"] = json!(player_trace);
            }

            let output = serde_json::to_string_pretty(&output).unwrap();
            if let Some(path) = trace_output {
                if let Err(err) = fs::write(&path, output) {
                    eprintln!("failed to write trace output {}: {err}", path.display());
                    std::process::exit(1);
                }
            } else {
                println!("{output}");
            }
        }
    }
}

fn apply_cli_input(host: &mut HeadlessHost, tick: u64, press_keys: &[u16], hold_keys: &[u16]) {
    let states = press_keys
        .iter()
        .map(|key| {
            (
                RuntimeButton::Keyboard(*key),
                ButtonState {
                    pressed: tick == 0,
                    just_pressed: tick == 0,
                    just_released: false,
                },
            )
        })
        .chain(hold_keys.iter().map(|key| {
            (
                RuntimeButton::Keyboard(*key),
                ButtonState {
                    pressed: true,
                    just_pressed: tick == 0,
                    just_released: false,
                },
            )
        }))
        .collect::<Vec<_>>();
    host.input.replace_button_states(states);
}

#[derive(Debug, serde::Serialize)]
struct RuntimeBlockerSummary {
    key: String,
    code: String,
    count: usize,
    first: String,
}

#[derive(Debug, serde::Serialize)]
struct PlayerTraceEntry {
    tick: u64,
    room_id: Option<usize>,
    room: Option<String>,
    runtime_id: usize,
    instance_id: i32,
    object_id: usize,
    object: String,
    x: f64,
    y: f64,
    hspeed: f64,
    vspeed: f64,
    facing_left: bool,
    alive: bool,
    grounded: bool,
    jump_active: bool,
    jump_hold_frames: u32,
    jump_cut_applied: bool,
    jump_button_key: u16,
    jump_pressed: bool,
    jump_just_pressed: bool,
    jump_just_released: bool,
    active_keys: Vec<String>,
    diagnostic_count: usize,
}

fn maybe_collect_player_trace(
    core: &RuntimeCore,
    trace_every: u32,
    player_trace: &mut Vec<PlayerTraceEntry>,
) {
    let tick = core.tick_count();
    if tick % u64::from(trace_every) != 0 {
        return;
    }

    let snapshot = core.snapshot();
    let Some(player) = snapshot.player else {
        return;
    };
    player_trace.push(PlayerTraceEntry {
        tick,
        room_id: snapshot.room_id,
        room: snapshot.room_name,
        runtime_id: player.runtime_id,
        instance_id: player.instance_id,
        object_id: player.object_id,
        object: player.object_name,
        x: player.x,
        y: player.y,
        hspeed: player.hspeed,
        vspeed: player.vspeed,
        facing_left: player.facing_left,
        alive: player.alive,
        grounded: player.jump.grounded,
        jump_active: player.jump.active,
        jump_hold_frames: player.jump.hold_frames,
        jump_cut_applied: player.jump.cut_applied,
        jump_button_key: snapshot.input_trace.jump_button_key,
        jump_pressed: snapshot.input_trace.jump_pressed,
        jump_just_pressed: snapshot.input_trace.jump_just_pressed,
        jump_just_released: snapshot.input_trace.jump_just_released,
        active_keys: snapshot.input_trace.active_keys,
        diagnostic_count: snapshot.diagnostics.len(),
    });
}

fn collect_runtime_blockers(
    diagnostics: &[RuntimeDiagnostic],
    seen_messages: &mut HashSet<String>,
    blockers: &mut HashMap<String, RuntimeBlockerSummary>,
) {
    for diagnostic in diagnostics {
        if !diagnostic.code.starts_with("runtime-unsupported-") {
            continue;
        }
        let message_key = format!("{}:{}", diagnostic.code, diagnostic.message);
        if !seen_messages.insert(message_key) {
            continue;
        }
        let key = runtime_blocker_key(diagnostic);
        let entry = blockers
            .entry(key.clone())
            .or_insert_with(|| RuntimeBlockerSummary {
                key,
                code: diagnostic.code.clone(),
                count: 0,
                first: diagnostic.message.clone(),
            });
        entry.count += 1;
    }
}

fn runtime_blocker_key(diagnostic: &RuntimeDiagnostic) -> String {
    if let Some(function) = message_field(&diagnostic.message, "function") {
        return format!("{}:{}", diagnostic.code, function);
    }
    if let Some(statement_kind) = message_field(&diagnostic.message, "statement_kind") {
        return format!("{}:{}", diagnostic.code, statement_kind);
    }
    diagnostic.code.clone()
}

fn message_field<'a>(message: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("{field}=");
    message
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&prefix))
}
