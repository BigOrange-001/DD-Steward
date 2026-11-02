//! Independent025 entry. Only this wrapper adds strategy025 metadata.
use dd_core::campaign018::runner::{
    Config, Status,
    replay022::{SCHEMA, apply},
    rollout027::Runtime027,
};
use dd_data::combat::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
    time::Instant,
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    schema_version: String,
    log_mode: String,
    journal_bytes: u64,
    runtime: String,
}
fn save(
    out: &Path,
    r: &Runtime027,
    journal: &mut Option<BufWriter<fs::File>>,
    mode: &str,
) -> Result<()> {
    let bytes = if let Some(f) = journal {
        f.flush().map_err(|e| e.to_string())?;
        f.stream_position().map_err(|e| e.to_string())?
    } else {
        0
    };
    let v = Commit {
        schema_version: SCHEMA.into(),
        log_mode: mode.into(),
        journal_bytes: bytes,
        runtime: r.checkpoint()?,
    };
    fs::write(
        out.join("checkpoint.tmp"),
        serde_json::to_vec(&v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(out.join("checkpoint.tmp"), out.join("checkpoint.json")).map_err(|e| e.to_string())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let funnel = args
        .iter()
        .position(|a| a == "--funnel")
        .map(|i| args.remove(i))
        .is_some();
    let start_kind = if let Some(i) = args.iter().position(|a| a == "--start") {
        if i + 1 >= args.len() {
            return Err("--start needs old-road|veteran|champion".into());
        }
        let v = args.remove(i + 1);
        args.remove(i);
        v
    } else {
        "old-road".into()
    };
    let guard_on = args
        .iter()
        .position(|a| a == "--zero-progress-fallback-on")
        .map(|i| args.remove(i))
        .is_some();
    let guard_off = args
        .iter()
        .position(|a| a == "--zero-progress-fallback-off")
        .map(|i| args.remove(i))
        .is_some();
    if guard_on && guard_off {
        return Err("026 conflicting cycle guard switches".into());
    }
    let rooms_legacy = args
        .iter()
        .position(|a| a == "--legacy-rooms")
        .map(|i| args.remove(i))
        .is_some();
    let ai_off = args
        .iter()
        .position(|a| a == "--legacy-ai-weights")
        .map(|i| args.remove(i))
        .is_some();
    let retreat_off = args
        .iter()
        .position(|a| a == "--legacy-csv-retreat")
        .map(|i| args.remove(i))
        .is_some();
    let disease_off = args
        .iter()
        .position(|a| a == "--legacy-disease-conflicts")
        .map(|i| args.remove(i))
        .is_some();
    let contract_off = args
        .iter()
        .position(|a| a == "--legacy-shrieker-contract")
        .map(|i| args.remove(i))
        .is_some();
    let mode = if let Some(i) = args.iter().position(|a| a == "--log") {
        if i + 1 >= args.len() {
            return Err("--log requires full or none".into());
        }
        let m = args.remove(i + 1);
        args.remove(i);
        Some(m)
    } else {
        None
    };
    if mode
        .as_deref()
        .is_some_and(|m| !["full", "none"].contains(&m))
    {
        return Err("--log requires full or none".into());
    }
    let load = Instant::now();
    let (out, cap, mut r, mut journal, mode) = if args.first().map(String::as_str)
        == Some("--resume")
        && args.len() == 3
    {
        let out = PathBuf::from(&args[1]);
        let saved: Commit = serde_json::from_slice(
            &fs::read(out.join("checkpoint.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if saved.schema_version != SCHEMA
            || !["full", "none"].contains(&saved.log_mode.as_str())
            || mode.as_ref().is_some_and(|m| m != &saved.log_mode)
        {
            return Err("025 checkpoint log mode/schema mismatch".into());
        }
        let mut r = Runtime027::restore(&saved.runtime)?;
        let journal = if saved.log_mode == "full" {
            let mut f = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(out.join("steps.jsonl"))
                .map_err(|e| e.to_string())?;
            if f.metadata().map_err(|e| e.to_string())?.len() < saved.journal_bytes {
                return Err("025 log shorter than checkpoint".into());
            }
            let mut reader = BufReader::new(Read::by_ref(&mut f).take(saved.journal_bytes));
            let mut line = String::new();
            let mut state = Value::Null;
            let mut last = 0;
            let mut bytes = 0;
            while reader.read_line(&mut line).map_err(|e| e.to_string())? > 0 {
                bytes += line.len() as u64;
                if !line.ends_with('\n') {
                    return Err("025 incomplete committed log".into());
                }
                let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
                last += 1;
                if v["schema_version"] != SCHEMA || v["step"] != last {
                    return Err("025 log step/schema mismatch".into());
                }
                apply(&mut state, &v["before"]["replay"])?;
                apply(&mut state, &v["after"]["replay"])?;
                line.clear();
            }
            if bytes != saved.journal_bytes
                || last != r.inner.inner.replay.inner.inner.inner.decisions
                || last > 0 && state != r.frame()?
            {
                return Err("025 log/checkpoint mismatch".into());
            }
            drop(reader);
            f.set_len(saved.journal_bytes).map_err(|e| e.to_string())?;
            f.seek(std::io::SeekFrom::End(0))
                .map_err(|e| e.to_string())?;
            Some(BufWriter::new(f))
        } else {
            if saved.journal_bytes != 0 || out.join("steps.jsonl").exists() {
                return Err("025 none checkpoint has a replay journal".into());
            }
            None
        };
        (
            out,
            args[2].parse::<u64>().map_err(|e| e.to_string())?,
            r,
            journal,
            saved.log_mode,
        )
    } else if args.len() == 5 {
        let mode = mode.unwrap_or("full".into());
        let config = Config {
            seed: args[1].parse::<u64>().map_err(|e| e.to_string())?,
            week_limit: args[2].parse::<u32>().map_err(|e| e.to_string())?,
            ..Default::default()
        };
        let mut r = Runtime027::new(config, &args[0])?;
        r.test_start(&start_kind)?;
        let out = PathBuf::from(&args[4]);
        fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        if out.join("checkpoint.json").exists() || out.join("result.json").exists() {
            return Err("025 artifacts exist; use --resume or new directory".into());
        }
        let journal = if mode == "full" {
            Some(BufWriter::new(
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(out.join("steps.jsonl"))
                    .map_err(|e| e.to_string())?,
            ))
        } else {
            None
        };
        (
            out,
            args[3].parse::<u64>().map_err(|e| e.to_string())?,
            r,
            journal,
            mode,
        )
    } else {
        return Err("campaign027 [--start old-road|veteran|champion] [--log full|none] all|none|GROUPS SEED WEEKS STEPS OUT; or --resume OUT ADDITIONAL_STEPS".into());
    };
    if args.first().is_some_and(|s| s == "--resume") {
        if guard_off && r.options.zero_progress_guard {
            return Err("026 cycle guard differs from checkpoint".into());
        }
        if ai_off && r.inner.mechanisms.zero_final_weights
            || retreat_off && r.inner.mechanisms.csv_corridor_retreat
            || disease_off && r.inner.mechanisms.disease_quirk_conflicts
            || contract_off && r.inner.mechanisms.shrieker_contract
        {
            return Err("025 mechanism switches differ from saved checkpoint".into());
        }
    } else {
        if guard_off {
            r.zero_progress_guard(false);
        }
        r.inner.mechanisms.zero_final_weights &= !ai_off;
        r.inner.mechanisms.csv_corridor_retreat &= !retreat_off;
        r.inner.mechanisms.disease_quirk_conflicts &= !disease_off;
        r.inner.mechanisms.shrieker_contract &= !contract_off;
    }
    if guard_on {
        r.zero_progress_guard(true);
    }
    if rooms_legacy {
        if args.first().is_some_and(|s| s == "--resume") {
            if !r.options.legacy_rooms {
                return Err("026 room variant differs from checkpoint".into());
            }
        } else {
            r.legacy_rooms()?;
        }
    }
    save(&out, &r, &mut journal, &mode)?;
    let load_seconds = load.elapsed().as_secs_f64();
    let start_step = r.inner.inner.replay.inner.inner.inner.decisions;
    if funnel && mode != "full" {
        return Err("028 funnel requires the existing full public replay".into());
    }
    let mut funnel_file = if funnel {
        let path = out.join("funnel-events.jsonl");
        let mut kept = String::new();
        if path.exists() {
            for line in fs::read_to_string(&path)
                .map_err(|e| e.to_string())?
                .lines()
            {
                if let Ok(v) = serde_json::from_str::<Value>(line)
                    && v["step"].as_u64().is_some_and(|n| n <= start_step)
                {
                    kept.push_str(line);
                    kept.push('\n');
                }
            }
        }
        fs::write(&path, kept).map_err(|e| e.to_string())?;
        Some(BufWriter::new(
            fs::OpenOptions::new()
                .append(true)
                .open(path)
                .map_err(|e| e.to_string())?,
        ))
    } else {
        None
    };
    let start = Instant::now();
    let mut failure = None;
    for _ in 0..cap {
        if r.inner.inner.replay.inner.inner.inner.status != Status::Running {
            break;
        }
        let before_view = if funnel { Some(r.observation()?) } else { None };
        let log = match r.step(mode == "full") {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(e) => {
                failure = Some(e);
                break;
            }
        };
        if let Some(f) = &mut funnel_file {
            let first = log["step"] == 1;
            let stage = log["before"]["opening_completed"] != log["after"]["opening_completed"];
            let depart = log["action"]["Provision"] == "Embark"
                || log["action"]["Opening"] == "Embark"
                || stage;
            let exit =
                !log["before"]["expedition"].is_null() && log["after"]["expedition"].is_null();
            if first || depart || exit {
                let after_view = r.observation()?;
                let event = json!({"step":log["step"],"before":dd_core::campaign018::runner::rollout027::funnel028::snapshot(&r,before_view.as_ref().ok_or("028 missing public view")?,first)?,"after":dd_core::campaign018::runner::rollout027::funnel028::snapshot(&r,&after_view,depart)?});
                serde_json::to_writer(&mut *f, &event).map_err(|e| e.to_string())?;
                writeln!(f).map_err(|e| e.to_string())?;
                f.flush().map_err(|e| e.to_string())?;
            }
        }
        if let Some(f) = &mut journal {
            serde_json::to_writer(&mut *f, &log).map_err(|e| e.to_string())?;
            writeln!(f).map_err(|e| e.to_string())?;
        }
        if r.inner
            .inner
            .replay
            .inner
            .inner
            .inner
            .decisions
            .is_multiple_of(257)
        {
            save(&out, &r, &mut journal, &mode)?;
            eprintln!(
                "027 step={} week={}",
                r.inner.inner.replay.inner.inner.inner.decisions, log["after"]["week"]
            );
        }
    }
    save(&out, &r, &mut journal, &mode)?;
    let visible = r.visible()?;
    let mut caretaker56 = match r.observation()? {
        dd_core::campaign018::runner::View::Campaign(o) => {
            dd_core::strategy025::caretaker::progress(&o)?
        }
        _ => serde_json::Value::Null,
    };
    if r.options.start != "old-road" {
        if let Some(goals) = caretaker56["goals"].as_array_mut() {
            for goal in goals {
                if goal["id"]
                    .as_str()
                    .is_some_and(|id| r.options.baseline_goals.contains(id))
                {
                    goal["done"] = json!(false);
                }
            }
        }
        caretaker56["achieved"] = json!(
            caretaker56["goals"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|g| g["done"] == true)
                .count()
        );
        caretaker56["main_achieved"] = json!(
            caretaker56["goals"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|g| g["done"] == true
                    && g["id"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("plot_") && id != "plot_tutorial_crypts"))
                .count()
        );
    }
    let status = failure
        .as_ref()
        .map(|s| Status::Error(s.clone()))
        .unwrap_or_else(|| r.inner.inner.replay.inner.inner.inner.status.clone());
    let mut result = json!({"profile":"candidate-rollout-027-v1","schema_version":SCHEMA,"log_mode":mode,"seed":r.inner.inner.replay.inner.inner.inner.config.seed,"week_budget":r.inner.inner.replay.inner.inner.inner.config.week_limit,"start_step":start_step,"decisions":r.inner.inner.replay.inner.inner.inner.decisions,"status":status,"seconds":start.elapsed().as_secs_f64(),"load_seconds":load_seconds,"visible":visible,"guard":r.inner.inner.guard,"candidates":r.inner.inner.candidates,
  "caretaker56":caretaker56,"strategy025":{"enabled":r.inner.enabled,"ordered":r.inner.ordered,"hits":r.inner.memory.hits,"ledger":r.inner.memory.ledger,"metrics":r.inner.memory.metrics,"future_plan":r.inner.memory.future_plan,"diagnostics":r.inner.memory.state,"boss_checks":r.inner.sources.boss_checks,"boss_attempts":r.inner.memory.boss_attempts,"boss_last":r.inner.memory.boss_last,"binding":r.inner.profile.binding(&r.inner.enabled,r.inner.ordered)?},"metrics025":{"counts":r.inner.memory.metrics.counts,"weeks":r.inner.memory.metrics.weeks,"trips":r.inner.memory.metrics.trips},"mechanism025":{"zero_target_fallback":!r.inner.enabled.is_empty(),"events":r.inner.zero_target_events,"flags":r.inner.mechanisms,"candidate_events":r.inner.mechanism_events},"boundary":"Unverified experience candidate; no formal training, no hidden policy inputs. Timeout is not victory."});
    result["diagnostics026"] = json!({"audit":r.audit.diagnostics,"medium":dd_core::strategy025::sync026::diagnostics::Audit::medium_summary(&r.inner.memory)});
    if r.options.zero_progress_guard {
        result["zero_progress_guard026"] =
            serde_json::from_str(r#"{
  "version": "candidate-zero-progress-026-v1",
  "enabled": true,
  "fallback": "existing_legacy_visited_rooms_gate_for_this_sortie_only",
  "source": "Unattended section3 strategy-deadpoint recovery; identical visible field exit recurs on the next return without a week advancing. No engine week rule or normal retreat threshold changes."
}
"#)
                .map_err(|e| e.to_string())?;
    }
    result["test_start027"] = json!({"kind":r.options.start,"formal_progress":r.options.start=="old-road","excluded_initial_goals":r.options.baseline_goals});
    result["sync026"] = r.inner.profile.parameters["sync026"].clone();
    result["rollout026"] = json!({"options":r.options,"settings":r.settings,"audit":r.audit});
    result["strategy025"]["usage"] = json!(r.inner.memory.usage);
    let warnings = if status == Status::Running {
        vec![]
    } else {
        r.inner.memory.usage.warnings()
    };
    for warning in &warnings {
        eprintln!("025 audit warning (endpoint may be truncated): {warning}");
    }
    result["strategy025"]["warnings"] = json!(warnings);
    fs::write(
        out.join("result.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"result":out.join("result.json"),"status":status})
    );
    if let Some(e) = failure {
        Err(e)
    } else {
        Ok(())
    }
}
