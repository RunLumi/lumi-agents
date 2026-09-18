# 21 — Release & Workflow Certification v1

Status: Normative

## 21.1 Goal

Make release readiness and workflow certification evidence-based.

## 21.2 Release rings

Canonical rings:

- DEV
- INTERNAL_ALPHA
- DOGFOOD
- CUSTOMER_CANARY
- STABLE

## 21.3 Runtime release requirements

External release MUST include:

- signed artifacts;
- checksums;
- SBOM;
- dependency/license/advisory scan;
- release notes;
- rollback target;
- compatibility metadata.

## 21.4 macOS

External macOS stable/canary MUST be signed and notarized.

## 21.5 Windows

External Windows stable/canary MUST be code-signed.

## 21.6 Workflow certification states

- EXPERIMENTAL
- ALPHA
- CERTIFIED
- DEPRECATED

## 21.7 Alpha certification

At least:

- 30 repeated controlled runs;
- >=90% verified completion;
- 0 unauthorized side effects;
- known failure taxonomy.

## 21.8 Customer canary certification

At least:

- 100 representative runs;
- >=95% verified completion;
- <5% unexpected human rescue on routine hardened path;
- 0 policy bypass;
- tested restart/recovery;
- compatibility matrix documented.

## 21.9 Certified workflow

For narrow hardened workflow target:

- >=99% verified completion on explicitly supported matrix;
- no unresolved high-severity security issues;
- postconditions defined;
- exception path defined;
- production monitoring in place.

## 21.10 Security gate

Release blocked on:

- known policy bypass;
- cross-tenant leak;
- unsigned/corrupt updater;
- secret exposure;
- unauthorized side effect;
- failed required adversarial suite.

## 21.11 Regression gate

Reproducible production failures SHOULD enter regression corpus before next stable release.

## 21.12 Provider certification

Provider/model combination MAY be certified per workflow.

A new model version SHOULD re-run representative evals before becoming default for hardened workflow.

## 21.13 App/OS certification

Certification is matrix-specific.

"Certified on Windows" is insufficient without supported version/session/app context where material.

## 21.14 Release evidence

Release record MUST answer:

- what changed;
- affected contracts;
- affected workflows/providers/platforms;
- security boundary change;
- eval evidence;
- known gaps;
- rollback;
- license/dependency change.

## 21.15 Emergency hotfix

Hotfix MAY bypass normal cadence but MUST NOT bypass:

- signing;
- critical tests;
- security review;
- rollback planning;
- audit trail.
