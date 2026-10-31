//! Unattended host adapter. Only filtered records enter the public log and monitor.
use dd_core::campaign018::runner::{Config, Runtime, Status, render};
use dd_data::combat::Result;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
#[cfg(windows)]
fn replace_existing(path: &Path, temporary: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut core::ffi::c_void,
            reserved: *mut core::ffi::c_void,
        ) -> i32;
    }
    let old = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let new = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // Both files are in the same directory; ReplaceFileW performs the replacement
    // without a period in which the committed checkpoint is absent.
    if unsafe {
        ReplaceFileW(
            old.as_ptr(),
            new.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
pub(super) fn atomic(path: &Path, text: &str) -> Result<()> {
    let temporary = path.with_extension("pending");
    let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    #[cfg(windows)]
    if path.exists() {
        return replace_existing(path, &temporary);
    }
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
pub fn run(args: &[String]) -> Result<()> {
    let mut config = Config::default();
    let mut directory = PathBuf::from("runs/018-seed-1");
    let mut resume = None;
    let mut plain = !std::io::stdout().is_terminal();
    let mut delay = 0u64;
    let mut every = if plain { 100u64 } else { 1u64 };
    let mut config_override = false;
    let mut candidate = false;
    let mut i = 0;
    while i < args.len() {
        let key = &args[i];
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
        if saved["run_id"].as_str() != Some(runtime.identity()) {
            return Err("output directory belongs to another run".into());
        }
    }
    recover_log(&directory.join("steps.jsonl"), runtime.decisions)?;
    let starting_decisions = runtime.decisions;
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
        runtime.decisions,
        &runtime.status,
        "初始化或恢复",
        "读取存档中的合法状态",
    );
    println!("{frame}");
    atomic(&directory.join("monitor.txt"), &frame)?;
    let mut last_week = initial["week"].clone();
    loop {
        let record = match runtime.step() {
            Ok(Some(r)) => r,
            Ok(None) => break,
            Err(error) => {
                runtime.mark_error(error.clone());
                atomic(&checkpoint, &runtime.checkpoint()?)?;
                let failed = serde_json::json!({"status":runtime.status,"decisions":runtime.decisions,"error":error,"last_visible":initial});
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
            || runtime.status != Status::Running;
        if show {
            let action = record.action.to_string();
            let frame = render(
                &record.after,
                record.step,
                &runtime.status,
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
            atomic(&directory.join("status.json"),&serde_json::to_string_pretty(&serde_json::json!({"status":runtime.status,"decisions":record.step,"visible":record.after})).map_err(|e|e.to_string())?)?;
            last_week = record.after["week"].clone();
        }
        initial = record.after;
        if delay > 0 {
            std::thread::sleep(Duration::from_millis(delay));
        }
    }
    let visible = runtime.refresh()?;
    let final_report = serde_json::json!({"profile":"candidate-auto-018","status":runtime.status,"config":runtime.config,"decisions":runtime.decisions,"elapsed_seconds":start.elapsed().as_secs_f64(),"decisions_per_second":(runtime.decisions-starting_decisions) as f64/start.elapsed().as_secs_f64(),"run_id":runtime.identity(),"source_root":runtime.source_root(),"statistics":"input_values_unmodified","visible":visible});
    let report = serde_json::to_string_pretty(&final_report).map_err(|e| e.to_string())?;
    atomic(&directory.join("result.json"), &report)?;
    println!("{report}");
    Ok(())
}

pub(super) fn recover_log(path: &Path, step: u64) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut reader = std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
    let mut line = Vec::new();
    let mut keep = 0u64;
    let mut previous = None;
    loop {
        line.clear();
        let n = reader
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if !line.ends_with(b"\n") {
            break;
        }
        let v: serde_json::Value = serde_json::from_slice(&line)
            .map_err(|e| format!("invalid committed action log: {e}"))?;
        let current = v["step"].as_u64().ok_or("action log missing step")?;
        if current > step {
            break;
        }
        if previous.is_some_and(|p| current != p + 1) {
            return Err("non-contiguous action log".into());
        }
        previous = Some(current);
        keep += n as u64;
    }
    drop(reader);
    OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|f| f.set_len(keep))
        .map_err(|e| e.to_string())
}
