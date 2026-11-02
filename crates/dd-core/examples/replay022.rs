//! Replay v2 command; checkpoints include the delta baseline and committed log offset.
use dd_core::campaign018::runner::{
    Config, Status,
    arena021::{Runtime021, Strategy},
    replay022::{Replay, SCHEMA, apply},
};
use dd_data::combat::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Seek, Write},
    path::Path,
    time::Instant,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    schema_version: String,
    journal_bytes: u64,
    replay: String,
}

fn checkpoint(out: &Path, r: &Replay, journal: &mut BufWriter<fs::File>) -> Result<()> {
    journal.flush().map_err(|e| e.to_string())?;
    let save = Commit {
        schema_version: SCHEMA.into(),
        journal_bytes: journal.stream_position().map_err(|e| e.to_string())?,
        replay: r.checkpoint()?,
    };
    fs::write(
        out.join("checkpoint.tmp"),
        serde_json::to_vec(&save).map_err(|e| e.to_string())?,
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
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let load = Instant::now();
    let (out, cap, mut r, mut journal, mut decoded) = if args.first().map(String::as_str)
        == Some("--resume")
        && args.len() == 3
    {
        let out = std::path::PathBuf::from(&args[1]);
        let cap = args[2].parse::<u64>().map_err(|e| e.to_string())?;
        let save: Commit = serde_json::from_slice(
            &fs::read(out.join("checkpoint.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if save.schema_version != SCHEMA {
            return Err("022 checkpoint schema mismatch".into());
        }
        let mut r = Replay::restore_config(&save.replay)?;
        let mut f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(out.join("steps.jsonl"))
            .map_err(|e| e.to_string())?;
        if f.metadata().map_err(|e| e.to_string())?.len() < save.journal_bytes {
            return Err("log shorter than committed checkpoint".into());
        }
        // Verify the committed prefix before trimming an interrupted write.
        let mut reader = BufReader::new(&f);
        let mut bytes = 0u64;
        let mut last_step = 0;
        let mut state = Value::Null;
        while bytes < save.journal_bytes {
            let mut line = String::new();
            let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
            if n == 0 || !line.ends_with('\n') {
                return Err("incomplete committed log".into());
            }
            bytes += n as u64;
            let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            if v["schema_version"] != SCHEMA || v["step"].as_u64() != Some(last_step + 1) {
                return Err("log schema/step mismatch".into());
            }
            apply(&mut state, &v["before"]["replay"])?;
            apply(&mut state, &v["after"]["replay"])?;
            last_step += 1;
        }
        if bytes != save.journal_bytes
            || last_step != r.inner.inner.inner.decisions
            || (last_step > 0 && state != r.frame()?)
        {
            return Err("log prefix differs from checkpoint".into());
        }
        drop(reader);
        f.set_len(save.journal_bytes).map_err(|e| e.to_string())?;
        f.seek(std::io::SeekFrom::End(0))
            .map_err(|e| e.to_string())?;
        (out, cap, r, BufWriter::new(f), state)
    } else if args.len() == 5 {
        let strategy = match args[0].as_str() {
            "baseline" => Strategy::Baseline,
            "rules021" => Strategy::Rules021,
            _ => return Err("unknown replay strategy".into()),
        };
        let config = Config {
            seed: args[1].parse::<u64>().map_err(|e| e.to_string())?,
            week_limit: args[2].parse::<u32>().map_err(|e| e.to_string())?,
            ..Default::default()
        };
        let cap = args[3].parse::<u64>().map_err(|e| e.to_string())?;
        let out = std::path::PathBuf::from(&args[4]);
        fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        if out.join("checkpoint.json").exists() || out.join("result.json").exists() {
            return Err("replay artifacts exist; use --resume or a new directory".into());
        }
        let f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out.join("steps.jsonl"))
            .map_err(|e| e.to_string())?;
        let definition = config.definition()?.with_content019()?.with_steward020()?;
        let rules = if strategy == Strategy::Rules021 {
            Some(Runtime021::rules_for(&definition)?)
        } else {
            None
        };
        let r = Replay::new(Runtime021::with_definition(
            config, definition, strategy, rules,
        )?);
        (out, cap, r, BufWriter::new(f), Value::Null)
    } else {
        return Err("usage: replay022 baseline|rules021 SEED WEEKS STEP_CAP OUT_DIRECTORY\n       replay022 --resume OUT_DIRECTORY ADDITIONAL_STEPS".into());
    };
    let load_seconds = load.elapsed().as_secs_f64();
    checkpoint(&out, &r, &mut journal)?;
    let verify = std::env::var("DD_REPLAY_VERIFY").ok().as_deref() == Some("1");
    let start_step = r.inner.inner.inner.decisions;
    let start = Instant::now();
    let mut verified = 0;
    let mut failure = None;
    for _ in 0..cap {
        if r.inner.inner.inner.status != Status::Running {
            break;
        }
        let before = if verify { Some(r.frame()?) } else { None };
        let log = match r.step() {
            Ok(Some(v)) => v,
            Ok(None) => break,
            Err(e) => {
                failure = Some(e);
                break;
            }
        };
        if let Some(before) = before {
            apply(&mut decoded, &log["before"]["replay"])?;
            if decoded != before {
                return Err("before observation roundtrip mismatch".into());
            }
            apply(&mut decoded, &log["after"]["replay"])?;
            if decoded != r.frame()? {
                return Err("after observation roundtrip mismatch".into());
            }
            verified += 1;
        }
        serde_json::to_writer(&mut journal, &log).map_err(|e| e.to_string())?;
        writeln!(journal).map_err(|e| e.to_string())?;
        if r.inner.inner.inner.decisions.is_multiple_of(257) {
            checkpoint(&out, &r, &mut journal)?;
        }
        if r.inner.inner.inner.decisions.is_multiple_of(500) {
            eprintln!("replay step={} week={}", log["step"], log["after"]["week"]);
        }
    }
    checkpoint(&out, &r, &mut journal)?;
    let seconds = start.elapsed().as_secs_f64();
    let visible = r.inner.refresh()?;
    let status = failure
        .as_ref()
        .map(|e| Status::Error(e.clone()))
        .unwrap_or_else(|| r.inner.inner.inner.status.clone());
    let result = json!({"schema_version":SCHEMA,"strategy":r.inner.strategy,"seed":r.inner.inner.inner.config.seed,"week_budget":r.inner.inner.inner.config.week_limit,
        "start_step":start_step,"decisions":r.inner.inner.inner.decisions,"status":status,"seconds":seconds,"load_seconds":load_seconds,"roundtrip_verified_steps":verified,
        "journal_bytes":journal.stream_position().map_err(|e|e.to_string())?,"rules_hash":r.inner.rules_hash(),"visible":visible,
        "boundary":"original values; Running and Timeout are not victory; checkpoint is private host state, never distribute as replay"});
    fs::write(
        out.join("result.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"result":out.join("result.json"),"status":status,"seconds":seconds,"verified":verified})
    );
    if let Some(e) = failure {
        Err(e)
    } else {
        Ok(())
    }
}
