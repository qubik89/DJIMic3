//! `cargo run -p recordings --example list [-- --import <dir>]`
//! Print the transmitters found; with `--import`, copy everything into `<dir>`.
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dest = args.iter().position(|a| a == "--import").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let scanner = recordings::Scanner::new();
    let mut txs = scanner.scan();
    if let Some(dest) = &dest {
        recordings::mark_imported(&mut txs, dest);
    }
    for tx in &txs {
        println!("{} at {} · TX{:?} · {:?} {:?}", tx.id, tx.mount.display(), tx.slot, tx.model, tx.firmware);
        for t in &tx.takes {
            let i = t.info.as_ref();
            println!(
                "  {:<28} {:>7.2}s {}-bit{} {} Hz · {}",
                t.key,
                i.map(|i| i.duration_secs).unwrap_or(0.0),
                i.map(|i| i.bits_per_sample).unwrap_or(0),
                if i.map(|i| i.float).unwrap_or(false) { " float" } else { "" },
                i.map(|i| i.sample_rate).unwrap_or(0),
                t.files.iter().map(|f| format!("{:?} {}B{}", f.kind, f.size, if f.imported { " ✓" } else { "" })).collect::<Vec<_>>().join(", ")
            );
        }
    }
    if let Some(dest) = dest {
        let all: Vec<PathBuf> = txs.iter().flat_map(|tx| &tx.takes).flat_map(|t| t.files.iter().map(|f| f.path.clone())).collect();
        let jobs = recordings::plan(&txs, &all, &dest).expect("plan");
        let started = std::time::Instant::now();
        let out = recordings::run(&jobs, &AtomicBool::new(false), |_| {});
        println!("{out:?} in {:.1}s", started.elapsed().as_secs_f64());
    }
}
