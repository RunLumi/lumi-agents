//! Assess imported role measurements without granting certification or authority.

use lumi_evals::RoleScorecard;
use std::io::{Read, Write};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && (arguments[0] == "--help" || arguments[0] == "-h") {
        println!("Usage: lumi-role-scorecard INPUT.json\n\nReads local imported measurements and writes a JSON assessment to stdout.\nThis command cannot certify a live role or authorize execution.\nExit 0: valid assessment produced; exit 2: invalid input or output failure.");
        return Ok(());
    }
    if arguments.len() != 1 {
        return Err("expected one input JSON path; use --help".to_owned());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&arguments[0])
        .map_err(|error| format!("open input: {error}"))?
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read input: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("input exceeds the 16 MiB assessment limit".to_owned());
    }
    let scorecard: RoleScorecard = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid scorecard JSON: {error}"))?;
    // No trusted evidence resolver is available at this import boundary.
    // Never turn an input's provenance or reviewer labels into certification.
    let assessment = scorecard.assess();
    let output = serde_json::to_vec_pretty(&assessment)
        .map_err(|error| format!("serialize assessment: {error}"))?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&output)
        .and_then(|()| stdout.write_all(b"\n"))
        .map_err(|error| format!("write assessment: {error}"))?;
    if !assessment.valid {
        return Err(
            "scorecard validation failed; inspect the assessment's validation_errors".to_owned(),
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
