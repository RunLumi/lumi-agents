//! Real-repo dogfood CLI: runs the fixture-driven delegation loop against
//! a real git repository and writes an independently verified report.
//!
//! ```text
//! cargo run -p lumi-evals --bin lumi-dogfood -- \
//!     --repo /path/to/real/repo \
//!     --fixture crates/lumi-agent/tests/fixtures/planning/repo-note.json \
//!     --out docs/evals/dogfood-report.json [--keep]
//! ```
//!
//! Exit code 0 only when every acceptance check passed. `--keep` leaves
//! the work directory (workspace clone + durable state) for inspection.

use lumi_agent::dogfood::{run_dogfood, DogfoodConfig};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(repo) = arg_value(&args, "--repo") else {
        eprintln!("usage: lumi-dogfood --repo <path> --fixture <path> [--out <path>] [--keep]");
        std::process::exit(2);
    };
    let Some(fixture) = arg_value(&args, "--fixture") else {
        eprintln!("usage: lumi-dogfood --repo <path> --fixture <path> [--out <path>] [--keep]");
        std::process::exit(2);
    };
    let out = arg_value(&args, "--out");
    let keep = args.iter().any(|a| a == "--keep");

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let work_dir = std::env::temp_dir().join(format!(
        "lumi-dogfood-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&work_dir).expect("create work dir");

    let config = DogfoodConfig {
        source_repo: PathBuf::from(&repo),
        fixture_path: PathBuf::from(&fixture),
        work_dir: work_dir.clone(),
    };
    let report = match run_dogfood(&config) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("dogfood pass failed: {e}");
            if !keep {
                let _ = std::fs::remove_dir_all(&work_dir);
            }
            std::process::exit(1);
        }
    };

    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Some(out_path) = out {
        if let Some(parent) = std::path::Path::new(&out_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&out_path, format!("{json}\n")).expect("write report");
        eprintln!("report written to {out_path}");
    }
    println!("{json}");

    if keep {
        eprintln!("work directory kept for inspection: {}", work_dir.display());
    } else {
        let _ = std::fs::remove_dir_all(&work_dir);
    }
    if !report.verified {
        eprintln!("dogfood pass DID NOT verify — see checks in the report");
        std::process::exit(1);
    }
}
