//! Real-repo dogfood CLI: runs the delegation loop against a real git
//! repository and writes an independently verified report.
//!
//! ```text
//! cargo run -p lumi-agent --bin lumi-dogfood -- \
//!     --repo /path/to/real/repo \
//!     --fixture crates/lumi-agent/tests/fixtures/planning/repo-note.json \
//!     --out docs/evals/dogfood-report.json [--keep] [--live]
//! ```
//!
//! Exit code 0 only when every acceptance check passed. `--keep` leaves
//! the work directory (workspace clone + durable state) for inspection.
//!
//! `--live` swaps the deterministic fixture planner for a REAL model:
//! credentials come from the environment and are consumed per call —
//! never written to disk, never logged.
//!
//! ```text
//! LUMI_DOGFOOD_API_KEY=sk-...            # required for --live
//! LUMI_DOGFOOD_ENDPOINT=https://api.openai.com
//! LUMI_DOGFOOD_MODEL=gpt-4o-mini
//! LUMI_DOGFOOD_FAMILY=openai             # or openai-compatible
//! ```

use lumi_agent::dogfood::{run_dogfood, run_dogfood_live, DogfoodConfig};
use lumi_agent::tools::default_tool_specs;
use lumi_agent::{ModelPlanner, Planner};
use lumi_models::adapters::{OpenAICompatibleDriver, OpenAIDriver};
use lumi_models::instance::{AuthHeaders, ProviderInstance};
use lumi_models::ureq_transport::UreqTransport;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

struct LiveSession {
    family: String,
    endpoint: String,
    model: String,
    api_key: String,
}

fn live_session_from_env() -> Result<LiveSession, String> {
    let api_key = std::env::var("LUMI_DOGFOOD_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "--live requires LUMI_DOGFOOD_API_KEY in the environment".to_owned())?;
    Ok(LiveSession {
        family: std::env::var("LUMI_DOGFOOD_FAMILY").unwrap_or_else(|_| "openai".to_owned()),
        endpoint: std::env::var("LUMI_DOGFOOD_ENDPOINT")
            .unwrap_or_else(|_| "https://api.openai.com".to_owned()),
        model: std::env::var("LUMI_DOGFOOD_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_owned()),
        api_key: api_key.trim().to_owned(),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(repo) = arg_value(&args, "--repo") else {
        eprintln!(
            "usage: lumi-dogfood --repo <path> --fixture <path> [--out <path>] [--keep] [--live]"
        );
        std::process::exit(2);
    };
    let Some(fixture) = arg_value(&args, "--fixture") else {
        eprintln!(
            "usage: lumi-dogfood --repo <path> --fixture <path> [--out <path>] [--keep] [--live]"
        );
        std::process::exit(2);
    };
    let out = arg_value(&args, "--out");
    let keep = args.iter().any(|a| a == "--keep");
    let live = args.iter().any(|a| a == "--live");

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
        provider: live.then(|| {
            let session = match live_session_from_env() {
                Ok(session) => session,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(2);
                }
            };
            eprintln!(
                "LIVE pass: family={} model={} endpoint={}",
                session.family, session.model, session.endpoint
            );
            (session.family.clone(), session.model.clone())
        }),
    };

    let report = std::thread::scope(
        |_scope| -> Result<lumi_agent::dogfood::DogfoodReport, String> {
            if !live {
                return run_dogfood(&config);
            }
            // Live pass: the production planner over the session's provider.
            // Session pieces are owned here; the planner borrows them for
            // exactly one run. The key exists only inside this scope.
            let session = live_session_from_env()?;
            eprintln!(
                "LIVE pass: family={} model={} endpoint={}",
                session.family, session.model, session.endpoint
            );
            let driver: Box<dyn lumi_models::ProviderDriver> = match session.family.as_str() {
                "openai" => Box::new(OpenAIDriver::new()),
                "openai-compatible" => Box::new(OpenAICompatibleDriver::new()),
                other => return Err(format!("unknown LUMI_DOGFOOD_FAMILY {other:?}")),
            };
            let instance = ProviderInstance::new(
                "dogfood-live",
                "openai-compatible",
                "dogfood live session",
                session.endpoint.clone(),
                lumi_protocol::SecretRef(session.api_key.clone()),
                session.family.clone(),
            );
            let auth: AuthHeaders = vec![(
                "Authorization".to_owned(),
                format!("Bearer {}", session.api_key),
            )];
            let transport = UreqTransport;
            let planner = ModelPlanner {
                driver: driver.as_ref(),
                instance: &instance,
                auth: &auth,
                transport: &transport,
                model: session.model.clone(),
                tools: default_tool_specs(),
                system_prompt: String::new(),
                timeout_ms: 120_000,
            };
            let planner_ref: &dyn Planner = &planner;
            run_dogfood_live(&config, planner_ref)
        },
    );

    let report = match report {
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
