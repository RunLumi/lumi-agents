# Reliability SLOs and Eval Gates

## Alpha

Per target workflow:

- >=30 repeated runs;
- >=90% verified completion in controlled scenarios;
- 0 unauthorized side effects;
- every failure classified.

## Customer canary

Per certified workflow:

- >=100 representative runs;
- >=95% verified completion;
- <5% unexpected human-rescue events on hardened routine path;
- 0 policy bypasses;
- rollback/recovery tested.

## Hardened narrow workflow

Target:

- >=99% verified completion on explicitly certified OS/app/version matrix;
- 0 unauthorized side effects in release/adversarial corpus.

## Metrics

- verified completion;
- unexpected rescue;
- expected approval;
- action count;
- unnecessary actions;
- latency;
- cost;
- provider/model;
- execution tier;
- retries/resume;
- postcondition failures;
- prompt-injection failures;
- privacy/egress violations;
- workflow economics.

## Rule

A required human approval is not a reliability failure.

A model claiming success without passing required verification is a failure.
