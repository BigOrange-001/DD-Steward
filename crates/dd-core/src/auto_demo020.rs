//! Steward CLI; shared durable file and committed-log protocol with legacy auto.
use super::auto_demo018::{atomic, recover_log};
use dd_core::campaign018::runner::{Config, Status, render, steward020::Runtime020 as Runtime};
use dd_data::combat::Result;
use std::{
    fs::{self, OpenOptions},
    io::{IsTerminal, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
pub fn run(args: &[String]) -> Result<()> {
    let mut config = Config::default();
    let mut directory = PathBuf::from("runs/020-seed-1");
    let mut resume = None;
    let mut plain = !std::io::stdout().is_terminal();
    let mut delay = 0u64;
    let mut every = if plain { 100u64 } else { 1u64 };
    let mut config_override = false;
    let mut candidate = false;
    let mut out_override = false;
    let mut budget: Option<u64> = None;
    let mut i = 0;
    while i < args.len() {
        let key = &args[i];
        if key == "--out" {
            out_override = true;
        }
        if key == "--candidate" {
            candidate = true;
            i += 1;
            continue;
        }
        if key == "--plain" {
            plain = true;
            i += 1;
            continue;
        }
        if ["--seed", "--weeks", "--mode", "--never-again"].contains(&key.as_str()) {
            config_override = true;
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {key}"))?;
        match key.as_str() {
            "--steps" => budget = Some(value.parse::<u64>().map_err(|e| e.to_string())?),
            "--seed" => config.seed = value.parse::<u64>().map_err(|e| e.to_string())?,
            "--weeks" => config.week_limit = value.parse::<u32>().map_err(|e| e.to_string())?,
            "--mode" => config.mode = value.clone(),
            "--never-again" => config.never_again = value.clone(),
            "--out" => directory = PathBuf::from(value),
            "--resume" => resume = Some(PathBuf::from(value)),
            "--delay-ms" => delay = value.parse::<u64>().map_err(|e| e.to_string())?,
            "--every" => every = value.parse::<u64>().map_err(|e| e.to_string())?.max(1),
            _ => return Err(format!("unknown auto option {key}")),
        }
        i += 2;
    }
    if !candidate {
        return Err("auto requires --candidate; uncalibrated prototype".into());
    }
    if resume.is_some() && config_override {
        return Err(
            "resume uses stored configuration; seed/mode/week overrides are rejected".into(),
        );
    }
    if !out_override {
        directory = match &resume {
            Some(path) => path
                .parent()
                .ok_or("resume checkpoint has no parent directory")?
                .to_path_buf(),
            None => PathBuf::from(format!("runs/020-seed-{}", config.seed)),
        };
    }
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("run.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("another process owns this output directory: {e}"))?;
    let checkpoint = directory.join("checkpoint.json");
    if resume.is_none() && checkpoint.exists() {
        return Err("output already contains a run; use --resume or a new directory".into());
    }
    let mut runtime = match resume {
        Some(path) => {
            Runtime::restore_config(&fs::read_to_string(path).map_err(|e| e.to_string())?)?
        }
        None => Runtime::new(config)?,
    };
    if checkpoint.exists() {
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&checkpoint).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let bound: serde_json::Value =
            serde_json::from_str(saved["inner"].as_str().ok_or("missing 020 inner save")?)
                .map_err(|e| e.to_string())?;
        if bound["run_id"].as_str() != Some(runtime.inner.identity()) {
            return Err("output directory belongs to another run".into());
        }
    }
    recover_log(&directory.join("steps.jsonl"), runtime.inner.decisions)?;
    let starting_decisions = runtime.inner.decisions;
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("steps.jsonl"))
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut initial = runtime.refresh()?;
    atomic(&checkpoint, &runtime.checkpoint()?)?;
    atomic(
        &directory.join("status.json"),
        &serde_json::to_string_pretty(&initial).map_err(|e| e.to_string())?,
    )?;
    let frame = render(
        &initial,
        runtime.inner.decisions,
        &runtime.inner.status,
        "初始化或恢复",
        "读取存档中的合法状态",
    );
    println!("{frame}");
    atomic(&directory.join("monitor.txt"), &frame)?;
    let mut last_week = initial["week"].clone();
    loop {
        if budget.is_some_and(|limit| runtime.inner.decisions - starting_decisions >= limit) {
            break;
        }
        let record = match runtime.step() {
            Ok(Some(r)) => r,
            Ok(None) => break,
            Err(error) => {
                runtime.inner.mark_error(error.clone());
                atomic(&checkpoint, &runtime.checkpoint()?)?;
                let failed = serde_json::json!({"status":runtime.inner.status,"decisions":runtime.inner.decisions,"error":error,"last_visible":initial});
                atomic(
                    &directory.join("status.json"),
                    &serde_json::to_string_pretty(&failed).map_err(|e| e.to_string())?,
                )?;
                return Err(error);
            }
        };
        writeln!(
            log,
            "{}",
            serde_json::to_string(&record).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
        log.flush().map_err(|e| e.to_string())?;
        log.sync_data().map_err(|e| e.to_string())?;
        atomic(&checkpoint, &runtime.checkpoint()?)?;
        let show = record.step.is_multiple_of(every)
            || record.after["week"] != last_week
            || runtime.inner.status != Status::Running;
        if show {
            let action = record.action.to_string();
            let frame = render(
                &record.after,
                record.step,
                &runtime.inner.status,
                &action,
                &record.reason,
            );
            if plain {
                println!("{frame}");
            } else {
                print!("\x1b[2J\x1b[H{frame}");
                std::io::stdout().flush().map_err(|e| e.to_string())?;
            }
            atomic(&directory.join("monitor.txt"), &frame)?;
            atomic(&directory.join("status.json"),&serde_json::to_string_pretty(&serde_json::json!({"status":runtime.inner.status,"decisions":record.step,"visible":record.after})).map_err(|e|e.to_string())?)?;
            last_week = record.after["week"].clone();
        }
        initial = record.after;
        if delay > 0 {
            std::thread::sleep(Duration::from_millis(delay));
        }
    }
    let visible = runtime.refresh()?;
    let final_report = serde_json::json!({"profile":"candidate-auto-020-v1","stop_reason":if runtime.inner.status==Status::Running{"prefix_budget"}else{"terminal"},"status":runtime.inner.status,"config":runtime.inner.config,"decisions":runtime.inner.decisions,"elapsed_seconds":start.elapsed().as_secs_f64(),"decisions_per_second":(runtime.inner.decisions-starting_decisions) as f64/start.elapsed().as_secs_f64(),"run_id":runtime.inner.identity(),"source_root":runtime.inner.source_root(),"statistics":"input_values_unmodified","visible":visible});
    let report = serde_json::to_string_pretty(&final_report).map_err(|e| e.to_string())?;
    atomic(&directory.join("result.json"), &report)?;
    println!("{report}");
    Ok(())
}
