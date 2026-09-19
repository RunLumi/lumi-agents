use std::process::Command;

const EXAMPLE: &str = include_str!("../../../docs/evals/fixtures/role-scorecard.synthetic.json");

fn input_file(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lumi-scorecard-cli-{}-{name}.json",
        std::process::id()
    ));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn imported_example_produces_metrics_without_live_certification() {
    let path = input_file("example", EXAMPLE);
    let output = Command::new(env!("CARGO_BIN_EXE_lumi-role-scorecard"))
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], true);
    assert!(matches!(
        report["canary"]["status"].as_str(),
        Some("fail" | "unknown")
    ));
    assert!(matches!(
        report["mature"]["status"].as_str(),
        Some("fail" | "unknown")
    ));
    assert_eq!(report["metrics"]["received_work_units"], 1);
}

#[test]
fn input_cannot_mint_runtime_verified_provenance() {
    let path = input_file(
        "forgery",
        &EXAMPLE.replace("imported_assertion", "runtime_verified"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_lumi-role-scorecard"))
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn duplicate_work_units_fail_instead_of_inflating_completion() {
    let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
    let duplicate = value["work_units"][0].clone();
    value["work_units"].as_array_mut().unwrap().push(duplicate);
    let path = input_file("duplicate", &value.to_string());
    let output = Command::new(env!("CARGO_BIN_EXE_lumi-role-scorecard"))
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], false);
    assert!(report["metrics"].is_null());
}

#[test]
fn production_shaped_import_is_still_only_an_assertion() {
    let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
    value["provenance"] = "production".into();
    value["coverage"]["representative"] = true.into();
    let mut units = Vec::new();
    let mut windows = Vec::new();
    let mut costs = Vec::new();
    let mut evidence = Vec::new();
    for week in 0..4 {
        let id = format!("week-{week}");
        windows.push(serde_json::json!({"window_id":id,"start_unix_seconds":week*604800,"end_unix_seconds":(week+1)*604800}));
        costs.push(serde_json::json!({"window_id":id,"support_cost_micro_usd":0,"deployment_cost_micro_usd":0}));
        let mut source = value["evidence"][0].clone();
        source["reference"] = format!("fixture://claimed-production-{week}").into();
        source["provenance"] = "production".into();
        source["window_id"] = id.clone().into();
        evidence.push(source);
        for item in 0..25 {
            let mut unit = value["work_units"][0].clone();
            unit["work_unit_id"] = format!("unit-{week}-{item}").into();
            unit["window_id"] = id.clone().into();
            unit["observed_at_unix_seconds"] = (week * 604800 + item + 1).into();
            units.push(unit);
        }
    }
    value["coverage"]["evidence_reference"] = evidence[0]["reference"].clone();
    value["work_units"] = units.into();
    value["windows"] = windows.into();
    value["window_costs"] = costs.into();
    value["evidence"] = evidence.into();
    let path = input_file("claimed-production", &value.to_string());
    let output = Command::new(env!("CARGO_BIN_EXE_lumi-role-scorecard"))
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["metrics"]["received_work_units"], 100);
    assert_eq!(report["recommended_maturity"], "ASSISTED");
    assert!(matches!(
        report["canary"]["status"].as_str(),
        Some("fail" | "unknown")
    ));
    assert!(matches!(
        report["mature"]["status"].as_str(),
        Some("fail" | "unknown")
    ));
}
