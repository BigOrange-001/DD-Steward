use dd_data::combat::Result;
fn run() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 5 {
        return Err("arena026 MANIFEST OUT TRIALS quick|full JOBS".into());
    }
    let trials = a[2].parse().map_err(|_| "invalid trials")?;
    let jobs = a[4].parse().map_err(|_| "invalid jobs")?;
    if !["quick", "full"].contains(&a[3].as_str()) {
        return Err("mode must be quick or full".into());
    }
    let result = dd_core::rollout027::arena::run(
        std::path::Path::new(&a[0]),
        std::path::Path::new(&a[1]),
        trials,
        a[3] == "quick",
        jobs,
    )?;
    println!("seconds={}", result["seconds"]);
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
