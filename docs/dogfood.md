# Real-repo dogfood pass

The dogfood pass proves the P0 delegation loop end to end against a real
git repository: a user goal drives a plan → propose → gate → execute →
observe → verify loop, the work lands in a real clone of the repository,
and every claim is verified **independently of the loop's own report**.

The model path is exercised offline: planning scenarios are JSON fixture
files replayed through the same `Planner` contract a live provider
implements, and a separate provider-wire test drives the REAL
`ModelPlanner` through the OpenAI adapter with recorded HTTP responses
(`crates/lumi-agent/tests/provider_contract_loop.rs`). No live provider
is required until a credential-bearing deployment exists.

## What is proven

- A durable, project-bound task (desktop `JsonStateStore`) transitions
  CREATED → RUNNING → COMPLETED with a closed `Run` record.
- Every proposed action is normalized by the tool and passes the
  orchestrator gate (policy → journal → execute → verify → audit).
- The work is real: the expected file exists in the repository clone and
  `git status` shows it — checked by the harness, not the loop.
- Zero policy denials; the hash-chained audit ledger verifies intact.
- Failure behavior is honest: an exhausted fixture fails the run and
  persists FAILED; a scenario that claims files it did not produce fails
  the evaluation (`dogfood_pass_fails_when_the_claimed_work_is_absent`);
  an injected instruction in untrusted file content cannot expand
  authority (traversal writes are refused and fail the run).

## Run it

```sh
# Against this repository:
cargo run -p lumi-agent --bin lumi-dogfood -- \
    --repo . \
    --fixture crates/lumi-agent/tests/fixtures/planning/repo-note.json \
    --out docs/evals/dogfood-run-real-repo.json

# Against any real repository (read-only: work happens in a clone):
cargo run -p lumi-agent --bin lumi-dogfood -- \
    --repo /path/to/repo \
    --fixture <scenario.json> \
    --out /tmp/report.json --keep
```

`--keep` preserves the work directory (clone + durable state) for
inspection. Exit code 0 only when every acceptance check passed.

## Latest recorded run (2026-09-20)

- Scenario `repo-note` against `/Volumes/SSD/lumi-agents` — **verified**.
- 4 model turns, 3 gated actions executed, 0 policy denials.
- Durable task COMPLETED; run record closed; audit chain intact.
- Full evidence: [`docs/evals/dogfood-run-real-repo.json`](evals/dogfood-run-real-repo.json).

## Live-provider pass — measured results (2026-09-20)

Model: `qwen2.5:3b` on a local Ollama endpoint (OpenAI-compatible, data
never leaves the machine). Attempts 3–5 after clarifying the goal brief:

| Attempt | Verified | Turns | Gated actions | Notes |
|---|---|---|---|---|
| 1 | no | 1 | 0 | ambiguous brief: model guessed a wrong path; fail-closed |
| 2 | no | 3 | 3 | work landed, then the model attempted a redundant second write — **write guard refused it**, task FAILED honestly (unnecessary-action refusal recorded) |
| 3–5 | **yes ×3** | 3 each | 2 each | read → write → git status → answer; 7/7 independent checks each |

Live evidence files: `docs/evals/dogfood-run-live{,-write-guard,-attempt4,-attempt5}.json`.
The measured takeaway: the loop's guards turned two model mistakes into
honest failures instead of silent damage — exactly the containment the
product contract requires.

## Live-provider pass (model actually planning)

```sh
LUMI_DOGFOOD_API_KEY=sk-... \
LUMI_DOGFOOD_ENDPOINT=https://api.openai.com \   # or any OpenAI-compatible endpoint
LUMI_DOGFOOD_MODEL=gpt-4o-mini \
cargo run -p lumi-agent --bin lumi-dogfood -- \
    --repo . --fixture crates/lumi-agent/tests/fixtures/planning/live-ledger-note.json \
    --out docs/evals/dogfood-run-live.json
```

In `--live` mode the fixture supplies only the goal brief and expected files;
the MODEL plans the turns through the production `ModelPlanner` and every
action still passes the orchestrator gate. The harness's independent checks
are unchanged, so a model that claims work it did not do fails the evaluation.
An invalid key is exercised regularly: the provider rejects, the loop fails
honestly, nothing is written, and the report records `verified: false`.

## Scope and limits

- The fixture planner is deterministic; it demonstrates the loop, gate,
  verification, durability, and evidence — not model reasoning quality.
  Representative live-model evaluations remain the pilot runbook's
  responsibility ([`docs/live-pilot-runbook.md`](live-pilot-runbook.md)).
- The harness's independent checks are the trust anchor: a live-model
  deployment reuses the same postconditions, so "the model said done"
  never substitutes for file/git/audit evidence.
