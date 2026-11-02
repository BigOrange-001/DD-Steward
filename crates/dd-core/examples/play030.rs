//! JSONL transport and private commit protocol. Policies never receive this file's state.
use dd_core::campaign018::runner::{
    Config,
    human030::Human,
    replay022::{self, SCHEMA},
};
use dd_data::combat::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
fn read(p: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn scene(id: &str) -> Result<dd_core::rollout027::PublicBattle> {
    let catalog = read(Path::new("data/human-scenes-030.json"))?;
    let row = catalog["rows"]
        .as_array()
        .ok_or("030 catalog")?
        .iter()
        .find(|r| r["id"] == id)
        .ok_or("unknown frozen scene id; use --list-scenes")?;
    serde_json::from_value(read(Path::new(
        row["file"].as_str().ok_or("030 scene path")?,
    ))?)
    .map_err(|e| e.to_string())
}
struct Writer {
    dir: PathBuf,
    journal: File,
    digest: Sha256,
    _lock: File,
}
impl Writer {
    fn open(dir: &Path, resume: bool) -> Result<(Self, Option<Human>)> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("writer.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("030 directory already has a writer: {e}"))?;
        let mut digest = Sha256::new();
        let restored = if resume {
            let cp = read(&dir.join("checkpoint.json"))?;
            if cp["version"] != "human030-commit-v1" {
                return Err("030 commit version".into());
            }
            let bytes = cp["committed_bytes"].as_u64().ok_or("030 byte boundary")?;
            let mut file = File::open(dir.join("steps.jsonl")).map_err(|e| e.to_string())?;
            let mut prefix = vec![];
            Read::by_ref(&mut file)
                .take(bytes)
                .read_to_end(&mut prefix)
                .map_err(|e| e.to_string())?;
            if prefix.len() as u64 != bytes
                || format!("{:x}", Sha256::digest(&prefix))
                    != cp["prefix_sha256"].as_str().ok_or("030 prefix digest")?
            {
                return Err("030 committed prefix damaged".into());
            }
            let mut frame = Value::Null;
            let mut step = 0;
            for line in prefix.split(|b| *b == b'\n').filter(|b| !b.is_empty()) {
                let log: Value = serde_json::from_slice(line).map_err(|e| e.to_string())?;
                step += 1;
                if log["schema_version"] != SCHEMA
                    || log["step"] != step
                    || log["decision_maker"] != "human"
                {
                    return Err("030 committed schema/step boundary".into());
                }
                replay022::apply(&mut frame, &log["before"]["replay"])?;
                replay022::apply(&mut frame, &log["after"]["replay"])?;
            }
            let mut human = Human::restore(&cp["state"])?;
            if human.steps != step
                || (step > 0 && frame != cp["last_frame"])
                || human.frame()? != cp["last_frame"]
            {
                return Err("030 checkpoint/replay public frame mismatch".into());
            }
            if file.metadata().map_err(|e| e.to_string())?.len() > bytes {
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_millis();
                let mut tail = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(dir.join(format!("uncommitted-tail-{stamp}.jsonl")))
                    .map_err(|e| e.to_string())?;
                file.seek(SeekFrom::Start(bytes))
                    .map_err(|e| e.to_string())?;
                io::copy(&mut file, &mut tail).map_err(|e| e.to_string())?;
                tail.sync_all().map_err(|e| e.to_string())?;
                OpenOptions::new()
                    .write(true)
                    .open(dir.join("steps.jsonl"))
                    .map_err(|e| e.to_string())?
                    .set_len(bytes)
                    .map_err(|e| e.to_string())?;
            }
            digest.update(&prefix);
            Some(human)
        } else {
            if ["steps.jsonl", "checkpoint.json", "result.json"]
                .iter()
                .any(|p| dir.join(p).exists())
            {
                return Err(
                    "030 refuses to overwrite an existing run; use --resume or a new directory"
                        .into(),
                );
            }
            None
        };
        let journal = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("steps.jsonl"))
            .map_err(|e| e.to_string())?;
        Ok((
            Self {
                dir: dir.into(),
                journal,
                digest,
                _lock: lock,
            },
            restored,
        ))
    }
    fn append(&mut self, log: &Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(log).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        self.journal.write_all(&bytes).map_err(|e| e.to_string())?;
        self.journal.flush().map_err(|e| e.to_string())?;
        self.digest.update(bytes);
        Ok(())
    }
    fn save(&mut self, h: &mut Human) -> Result<()> {
        self.journal.flush().map_err(|e| e.to_string())?;
        self.journal.sync_all().map_err(|e| e.to_string())?;
        let frame = h.frame()?;
        let status = h.status()?;
        let arena_outcome = h.observation()?["Arena"]["outcome"].clone();
        let cp = json!({"version":"human030-commit-v1","committed_bytes":self.journal.metadata().map_err(|e|e.to_string())?.len(),"prefix_sha256":format!("{:x}",self.digest.clone().finalize()),"last_frame":frame,"state":h.save()?});
        let temp = self.dir.join("checkpoint.tmp");
        fs::write(&temp, serde_json::to_vec(&cp).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temp)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        fs::rename(temp, self.dir.join("checkpoint.json")).map_err(|e| e.to_string())?;
        fs::write(self.dir.join("result.json"),serde_json::to_vec_pretty(&json!({"schema_version":SCHEMA,"profile":"human030","decision_maker":"human","decisions":h.steps,"status":status,"arena_outcome":arena_outcome,"frame":frame})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
    }
}
fn answer(value: Value) -> Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, &value).map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}
fn rpc(h: &mut Human, w: &mut Writer, request: &Value) -> Result<Value> {
    if let Some(step) = request["step"].as_u64()
        && step != h.steps + 1
    {
        return Err("stale_step".into());
    }
    match request["cmd"].as_str().unwrap_or("view") {
        "info" => Ok(
            json!({"protocol":"dd-human-030-v1","localization_files":h.localization_files(),"private_checkpoint":"checkpoint.json","commands":["view","poll","note","hints","seen","act","save","q"]}),
        ),
        "view" | "poll" => h.public(),
        "wait_hints" => h.wait_advice(),
        "note" => {
            let text = request["text"].as_str().ok_or("note text required")?;
            if text.len() > 65536 {
                return Err("note exceeds 64 KiB".into());
            }
            h.notes.push(text.into());
            Ok(json!({"notes":h.notes}))
        }
        "hints" => {
            h.hints = request["enabled"].as_bool().unwrap_or(!h.hints);
            h.public()
        }
        "seen" => {
            h.seen(
                request["rules"].as_bool().unwrap_or(false),
                request["rollout"].as_bool().unwrap_or(false),
            );
            Ok(json!({"acknowledged":true}))
        }
        "act" => {
            let public = h.public()?;
            let action = if let Some(a) = request.get("action") {
                a.clone()
            } else {
                let id = request["index"].as_u64().ok_or("action index required")?;
                let row = public["actions"]
                    .as_array()
                    .ok_or("030 menu")?
                    .iter()
                    .find(|r| r["id"] == id)
                    .ok_or("action_not_in_public_menu")?;
                if let Some(a) = row.get("action") {
                    a.clone()
                } else {
                    let family = row["template"].as_str().ok_or("030 action family")?;
                    let params = request.get("params").ok_or("parameters_required")?;
                    json!({"Town":{family:params}})
                }
            };
            let mut log = h.act(
                action,
                request["confirm"].as_bool().unwrap_or(false),
                request["thinking_seconds"].as_f64().unwrap_or(0.),
            )?;
            log["human"]["scripted"] = json!(request["scripted"].as_bool().unwrap_or(false));
            w.append(&log)
                .map_err(|e| format!("030 persistence failure: {e}"))?;
            if h.steps.is_multiple_of(257) || h.status()? != "Running" {
                w.save(h)
                    .map_err(|e| format!("030 persistence failure: {e}"))?;
            }
            Ok(json!({"committed_action":log,"next":h.public()?}))
        }
        "q" | "save" => {
            let t = request["thinking_seconds"].as_f64().unwrap_or(0.);
            if !t.is_finite() || t < 0. {
                return Err("invalid thinking time".into());
            }
            h.thinking += t;
            w.save(h)?;
            Ok(json!({"saved":true,"decisions":h.steps}))
        }
        _ => Err("unknown_command".into()),
    }
}
fn main_inner() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|a| a == "--shadow-worker") {
        let mut games = std::collections::BTreeMap::new();
        for line in io::stdin().lock().lines() {
            let line = line.map_err(|e| e.to_string())?;
            let result = (|| -> Result<Value> {
                let req: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
                let job: dd_core::campaign018::runner::human030::advice::Job =
                    serde_json::from_value(req["job"].clone()).map_err(|e| e.to_string())?;
                let id = job.input.content.id()?;
                if !games.contains_key(&id) {
                    games.insert(
                        id.clone(),
                        dd_data::combat::GameData::load_with_content(
                            &dd_data::game_dir(),
                            job.input.content.clone(),
                        )?,
                    );
                }
                dd_core::campaign018::runner::human030::advice::compare(
                    &games[&id],
                    &job,
                    &req["human"],
                    req.get("cached").filter(|v| !v.is_null()),
                )
            })();
            answer(match result {
                Ok(data) => json!({"ok":true,"data":data}),
                Err(e) => json!({"ok":false,"error":e}),
            })?;
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--list-scenes") {
        return answer(json!({"ok":true,"data":read(Path::new("data/human-scenes-030.json"))?}));
    }
    let mut arena = None;
    let mut resume = None;
    let mut start = "old-road".to_string();
    let mut seed = 1u64;
    let mut weeks = 208;
    let mut out = None;
    let mut hints = true;
    let mut hints_override = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str(){"--json"|"--campaign"=>{},"--no-hints"=>{hints=false;hints_override=Some(false);},"--hints"=>{hints=true;hints_override=Some(true);},"--arena"|"--resume"|"--start"|"--seed"|"--weeks"|"--out"=>{let key=&args[i];i+=1;let v=args.get(i).ok_or("missing argument")?;match key.as_str(){"--arena"=>arena=Some(v.clone()),"--resume"=>resume=Some(v.clone()),"--start"=>start=v.clone(),"--seed"=>seed=v.parse().map_err(|_|"invalid seed")?,"--weeks"=>weeks=v.parse().map_err(|_|"invalid weeks")?,"--out"=>out=Some(v.clone()),_=>unreachable!()}},_=>return Err("usage: play030 --json [--arena ID | --start old-road|veteran|champion] --seed N --out DIR [--no-hints] | --resume DIR".into())}
        i += 1;
    }
    let dir = PathBuf::from(
        resume
            .as_ref()
            .or(out.as_ref())
            .ok_or("--out or --resume required")?,
    );
    let (mut writer, restored) = Writer::open(&dir, resume.is_some())?;
    let mut h = if let Some(h) = restored {
        h
    } else if let Some(id) = arena {
        Human::arena(&id, scene(&id)?, seed, hints)?
    } else {
        Human::campaign(
            Config {
                seed,
                week_limit: weeks,
                ..Default::default()
            },
            &start,
            hints,
        )?
    };
    if let Some(enabled) = hints_override {
        h.hints = enabled;
    }
    writer.save(&mut h)?;
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let request = serde_json::from_str::<Value>(&line);
        let result = request
            .as_ref()
            .map_err(|e| e.to_string())
            .and_then(|r| rpc(&mut h, &mut writer, r));
        let quit = result.is_ok() && request.as_ref().is_ok_and(|r| r["cmd"] == "q");
        match result {
            Ok(data) => answer(json!({"ok":true,"data":data}))?,
            Err(e) => {
                if e.starts_with("030 persistence failure:") {
                    return Err(e);
                }
                let mut f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("diagnostics.txt"))
                    .map_err(|e| e.to_string())?;
                writeln!(f, "step={} error={e}", h.steps + 1).map_err(|e| e.to_string())?;
                let public = if [
                    "confirmation_required",
                    "action_not_in_public_menu",
                    "parameters_required",
                    "stale_step",
                    "unknown_command",
                    "invalid thinking time",
                ]
                .contains(&e.as_str())
                {
                    e
                } else {
                    "operation_failed; see private diagnostics.txt".into()
                };
                answer(json!({"ok":false,"error":public}))?;
            }
        }
        if quit {
            return Ok(());
        }
    }
    writer.save(&mut h)
}
fn main() {
    if let Err(e) = main_inner() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
