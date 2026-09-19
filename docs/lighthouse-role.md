# Lighthouse Role: Finance Operations Associate

Status: provisional experiment; evidence date: 2026-09-19

This role is hypothetical until a real tenant grants bounded access and the work
is measured. It is an internal product and pilot contract, not a customer
guarantee and not a claim that Lumi can replace a finance employee.

## Decision and alternatives

Finance Operations is the provisional lighthouse because it combines the best
current repository leverage with a tractable verification boundary. The formal
alternatives are Sales Operations and Facebook Ads Operations. HR Operations and
Legal Operations remain candidates to test when a real process owner and safe
access path exist.

| Dimension | Finance Ops | Sales Ops | Facebook Ads Ops |
| --- | --- | --- | --- |
| Verified work | High for matching, policy checks, and exception reports; lower for any judgment or posting | High for record retrieval, draft generation, and approved CRM updates; sending remains approval-gated | Medium for metric reports and threshold flags; anomaly correctness needs domain ground truth |
| Access | Medium-low: likely ERP/accounting plus ledger or approved export; financial data is sensitive | Medium: CRM plus email draft/send path; OAuth and schema mapping required | Medium-low: Meta app, Business Manager, ad-account asset assignment, token lifecycle, and API/version constraints |
| Likely buyer | Controller, finance-operations lead, or shared-services manager; inference | RevOps or sales-operations manager; inference | Performance-marketing or agency-operations lead; inference |
| Deployment | Medium: two relevant packs exist, but real connectors and customer schemas are absent; invoice path can be API-first | Medium-high: quote-followup pack and API shape exist, but every send needs approval and customer field mapping | Medium-low: no existing pack or connector; read-only API route is plausible but must be built and verified |
| Economics | Unknown; pack values are synthetic declarations only | Unknown; pack values are synthetic declarations only | Unknown; no pack baseline exists |
| Provisional read | Lead for the first experiment | Close runner-up if a CRM queue is already accessible | Low-risk fallback if Meta access is available first |

The ranking is a judgment under missing evidence, not a demand forecast. Finance
is a moderate lead, Sales is a close alternative, and Ads is a plausible access-
driven switch. No option has customer proof.

## User, buyer, veto, and current alternative

These are working hypotheses to verify in discovery, not claims about a known
account.

| Role | Finance Ops | Sales Ops | Facebook Ads Ops |
| --- | --- | --- | --- |
| Actual user | AP or finance-operations analyst processing invoices, expenses, and mismatches | Sales-operations coordinator managing quote and follow-up queues | Performance marketer or agency-operations analyst preparing reports and triage |
| Economic buyer | Controller, head of finance operations, or shared-services manager | RevOps or sales-operations leader | Marketing-operations or performance-marketing leader |
| IT/security veto | Finance-systems admin, security, or data-protection owner | CRM/email admin and security owner | Meta Business Manager admin, security, or data-protection owner |
| Process/compliance veto | Controller, audit, or compliance reviewer | Sales leader for message policy and customer data | Marketing owner for metric definitions and escalation policy |
| Current alternative | ERP/expense features, spreadsheets/email, internal scripts, BPO/shared services, Power Automate, or UiPath | CRM workflows, Agentforce/Power Automate, spreadsheets, or manual follow-up | Ads Manager exports, agency dashboards, spreadsheets, or internal reporting scripts |

The pilot must identify the real user, economic buyer, and veto owner before
access work begins. A user who likes the workflow but cannot grant access or fund
the next period is not a qualified buyer.

## Role contract

### Initial in-scope work

1. Receive a bounded invoice or expense work unit from an approved folder,
   connector, or queue.
2. Read the relevant ERP/accounting and ledger/expense records under explicit
   read-only grants.
3. Normalize identifiers, dates, currencies, line items, vendors, and policy
   fields without silently filling missing values.
4. Match records using deterministic rules plus bounded model assistance where
   the result remains independently checkable.
5. Produce a line-level reconciliation or audit report with source references,
   mismatch reasons, confidence/unknown fields, and an exception queue.
6. Record a local artifact checksum and structured evidence for every completed
   work unit.
7. Route ambiguous, unsupported, or policy-sensitive cases to the named human
   specialist.

The first canary should prefer ERP read-only data plus a read-only ledger source
or customer-approved ledger export. If the bank connector cannot be safely
provided, a reduced export-based canary may test reconciliation mechanics, but it
must be labelled reduced and cannot prove full connector deployment.

### Explicitly out of scope

- Payments, transfers, refunds, bank instructions, or money movement.
- Reimbursement approval, vendor approval, tax filing, statutory reporting, or
  binding accounting sign-off.
- Autonomous changes to the chart of accounts, invoice amounts, payment terms,
  bank destinations, or financial commitments.
- External email or vendor communication in the initial canary.
- Personnel decisions, legal advice, or employment judgments.
- Guessing through missing data, policy exceptions, or an unverifiable side
  effect.

The existing expense pack contains an external `record_audit_verdict` write. That
path remains a separate, policy-gated capability and is not certified for the
initial canary; its presence means the current pack is not itself read-only. The
initial canary can prove the reconciliation and exception-reporting wedge without
enabling that write.

## Work-unit and outcome contract

An initial work unit is one invoice-period/account reconciliation or one expense
report audit with a stable input identity. Each run must record:

- source system, tenant, environment, and approved connector/account identity;
- input record IDs and source timestamps;
- policy/SOP version and matching-rule version;
- matched pairs, unmatched records, and line-level reasons;
- output artifact ID and checksum;
- exception route and human intervention minutes;
- runtime cost, latency, executor tier, provider/model, and failure category;
- executor outcome: `DELIVERED`, `REFUSED`, `NO_EFFECT`, `AMBIGUOUS`,
  `ERROR`, or `CANCELLED`;
- verification status: `PASSED`, `FAILED`, `AMBIGUOUS`, or `NOT_REQUIRED`,
  with the postcondition results and evidence references recorded separately
  from the executor outcome, as required by
  [spec 11.16](specs/v1/11-audit-evidence-verification.md#1116-executor-outcome-vs-verification).

“Report created” is not enough. A verified unit requires the expected input set,
matching result, report artifact, and evidence references to reconcile, with
required verification `PASSED`. `DELIVERED` plus `FAILED` verification is not a
verified success. Correct refusal or escalation is reported separately from
completed routine units. Failed and ambiguous units remain in the declared
canary denominator. A materially ambiguous state stops the run and requires
external inspection before retry.

## Exact access inputs needed

The smallest useful pilot request is:

1. A named finance process owner and named human exception specialist.
2. The target system and environment: ERP/accounting product, expense product,
   ledger source, tenant/company ID, API version, and staging or production-like
   status.
3. A read-only credential reference held by the local secret broker. Never paste
   a password, token, bank credential, or customer data into prompts, fixtures,
   or audit logs.
4. Read-only scopes for invoice/bill/expense records, vendors, ledger entries,
   attachments or receipt references, and the minimum metadata needed to verify
   identity. Exclude payment, transfer, tax filing, and administrative scopes.
5. If using a ledger export, an approved file location, file format, schema,
   checksum method, retention period, and data-egress rule.
6. A redacted or approved sample plus at least 100 representative work units for
   canary evaluation. The sample must include normal cases, duplicates, missing
   receipts, currency/date edge cases, and known mismatches.
7. The finance SOP, reconciliation policy, chart-of-accounts or field mapping,
   escalation rules, supported currencies, and expected exception categories.
8. A manual baseline from 5–10 operator runs: elapsed cycle time, active human
   minutes, expected review minutes, and error/rework observed.
9. The pilot's data retention, screenshot/evidence, residency, and local/cloud
   routing policy.
10. The allowed deployment environment, OS/app versions, maintenance window,
    cancellation owner, and local emergency-stop procedure.

No access input is currently available. Until these exist, this role remains a
proposal.

## Declared economics versus evidence

The repository declares these pack-level hypotheses:

| Pack | Declared manual / residual minutes | Declared volume | Declared implementation hours | Evidence status |
| --- | --- | --- | --- | --- |
| Invoice reconciliation | 45 / 5 per run | 60 runs/month | 60 | Fixture metadata only |
| Expense report audit | 25 / 6 per run | 200 runs/month | 80 | Fixture metadata only; native step marked bottleneck |
| Quote follow-up | 10 / 1 per run | 400 runs/month | 40 | Fixture metadata only; email send is approval-gated |

The loaded labor and runtime-cost fields in these packs are internal declared
values, not market prices or measured customer economics. The pilot must replace
them with observed baseline minutes, residual minutes, support minutes, runtime
cost, deployment hours, and customer-specific reuse.

### Pricing and renewal test

The pricing hypothesis is conditional and must be set after measurement:

```text
pilot_fee = max(measured_pilot_delivery_and_support_cost,
                alpha * gross_labor_value_released_during_pilot)
```

Use a pre-declared `alpha` test cell of 10% or 20% across separate pilots. These
are experimental parameters, not market rates. Quote a currency amount only
after the 5–10-run baseline and pilot-cost estimate are approved by the buyer.

The ongoing model hypothesis is a monthly role-capacity or verified-volume fee.
Renewal requires four consecutive weeks with positive `net_value_created`, at
least 95% verified completion, fewer than 5% unexpected rescue, correct
escalation, and a named buyer willing to fund the next period. A technically
completed task without measured net value or renewal intent is not commercial
proof.

## Seven-day falsification plan

| Day | Evidence-producing action | Exit evidence |
| --- | --- | --- |
| 1 | Freeze one process definition; identify owner, specialist, systems, and authority ceiling. | Named owner, exact queue, no-money boundary, access checklist |
| 2 | Run read-only health probes against the real or approved staging system. | Authenticated read succeeds; tenant/account identity and scopes are recorded; no writes occur |
| 3 | Time 5–10 manual runs and classify normal, mismatch, and exception cases. | Baseline minutes, residual review definition, 100-unit sample plan |
| 4 | Define postconditions, evidence fields, exception routes, and redacted fixtures from approved data. | Reviewable run contract and failure taxonomy |
| 5 | Run 30 controlled work units through the narrow path. | Per-unit outcome, evidence, intervention minutes, and failure category |
| 6 | Review every failure and calculate net human minutes released after rescue/support. | Updated risk, coverage, economics, and reuse estimate |
| 7 | Continue, narrow, or switch to Sales/Ads based on technical and commercial gates below. | Written go/no-go decision; no unsupported market claim |

### Founder-led commercial test

Use a list of 10–30 relevant prospects with a visible recurring queue and a
reachable process owner. The founder may use the draft in `docs/product-thesis.md`
later; this agent has not sent it. The working Day-7 commercial threshold is at
least three substantive workflow reviews, two approved sample/baseline packages,
and one process owner willing to evaluate a bounded paid pilot. These are
decision thresholds, not market benchmarks. If they are missed, change the
segment or message before adding product scope.

## Gates and stop conditions

### Early alpha: at least 30 representative units

Fewer than 30 exploratory units may expose failures, but they must not be called
an alpha result or used to claim the alpha gate.

Continue only if all are true:

- at least 90% verified completion;
- zero unauthorized side effects and zero money movement;
- no material ambiguous state is reported as success;
- unexpected human rescue is at most 15% of units and is fully classified;
- every exception is either correctly escalated or proven safe to complete;
- runtime cost, human minutes, and support minutes are recorded.

### Canary: at least 100 representative units

Advance only if all are true:

- at least 95% of all declared canary units have verified completion;
- fewer than 5% unexpected human-rescue events;
- zero policy bypasses or unauthorized side effects;
- every ambiguous unit is quarantined before any retry and independently
  resolved; ambiguous units remain separately reported and are excluded from the
  verified-success count;
- routine human minutes, residual minutes, cycle time, runtime cost, and support
  cost are measured rather than declared;
- the role owner confirms that the output is useful enough to continue.

### Mature-role target

These are internal targets, not promises: at least 99% verified completion on a
certified routine matrix, zero unauthorized side effects, correct escalation of
all exceptions, at least 80% of baseline routine minutes removed, at least 80%
of eligible units without unexpected rescue, automation cost no more than 30% of
displaced loaded labor value, four consecutive weeks of operation, and no
material safety, privacy, or compliance regression.

### Stop or switch

- No named owner or usable access by Day 2: stop the Finance test and evaluate a
  candidate with an already available read-only path.
- No 100 representative units or baseline by Day 3: do not claim a canary.
- Less than 90% verified completion or more than 15% unexpected rescue after 30
  units: narrow the role or reject it before adding automation scope.
- Repeated ambiguity, unbounded sensitive access, or inability to prove the
  postcondition: remove that path from the role.
- Weak economics after support and deployment are measured: reject the workflow,
  even if technical completion is high.
- Customer-specific code remains dominant after the third deployment or reuse is
  below 70%: treat that as evidence against productization and redesign the
  boundary.

## Alternative access contracts

If Finance access is unavailable, the alternatives need their own bounded proof.

### Sales Operations

Required inputs: Salesforce/CRM org URL and tenant identity; external-client-app
OAuth approval; read-only quote/opportunity scopes plus narrowly defined update
scope if needed; email draft connector and a human approval owner; recipient and
sender restrictions; quote queue sample; baseline time; follow-up policy; and
postconditions for quote identity, draft identity, approval, and sent-message ID.

Initial proof should stop at verified drafts unless a human explicitly approves
each send. The buyer hypothesis is RevOps or Sales Ops; the economic proof is
manual follow-up minutes and queue aging, not claimed revenue uplift.

### Facebook Ads Operations

Required inputs: Meta Business Manager ID; ad-account IDs; app ID; system-user or
user token held by the secret broker; explicit read-only `ads_read` scope and
asset assignment; Marketing API version; reporting timezone/currency; selected
metric fields and date windows; anomaly thresholds; destination for the report
and exception queue; 100 representative account/campaign snapshots or periods;
and a marketing-ops reviewer.

No campaign edits, budget changes, pauses, creative publishing, or spend
movement are allowed. A report query is only a read postcondition. An anomaly
flag needs a versioned rule and a human-confirmed outcome before it can count as
verified diagnostic work.

## Strongest counterargument

Finance is the wrong lighthouse if incumbents already provide invoice extraction,
approval, and accounting integrations, while customer security review and
cross-system mapping consume more time than the labor saved. A read-only Ads
reporting role could reach a pilot faster with less financial sensitivity.

That counterargument is strong. The response is to test reconciliation and
exception ownership, not OCR, and to make access the first gate. If a real tenant
cannot provide bounded Finance access quickly, the decision should switch or
stop. Repository leverage is not a reason to force the market.

## Reference-derived implementation constraints

- From T3 Code at reviewed commit `9ea9c3d5d2c444133e3ddff40eecf38737951589`:
  the execution environment owns credentials, files, provider processes, and
  machine state. Remote supervision must not receive long-lived secrets.
- From Cua Driver at reviewed commit
  `05f29785b508a4441ec3aa06c556a8e8b26c1d71`: use bounded manifests and exact
  effect outcomes; “transport returned OK” is not a verified business effect.
- From OpenAI Codex at reviewed commit
  `7498521d288b9b3b96ffba4eedf089d8d6e06a84`: approval and sandbox are separate,
  external content has lower authority, and resumes need durable state. Do not
  use an automated reviewer as authority for financial actions.
- From Claude Code at reviewed commit
  `31a3b00bef145a0393d9dbf840a98674fec07712`: keep the Tool, Skill, Agent,
  Hook, Connector, and Workflow Pack vocabulary distinct. Do not copy
  proprietary implementation or treat a skill as workflow certification.

## Sources

Evidence date for all links below: 2026-09-19.

- [QuickBooks Online developer getting started](https://developer.intuit.com/app/developer/qbo/docs/get-started) — authoritative URL opened, but the portal returned a compiling/pre-fill shell in this review; OAuth, sandbox, and schema details remain a live verification item.
- [QuickBooks Online Invoice API reference](https://developer.intuit.com/app/developer/qbo/docs/api/accounting/all-entities/invoice) — authoritative URL opened, but the portal returned a compiling/pre-fill shell; no detailed invoice claim is treated as verified from this page alone.
- [Salesforce REST API resources](https://developer.salesforce.com/docs/platform/api-rest/guide/intro-rest-resources.html) and [OAuth](https://developer.salesforce.com/docs/platform/api-rest/guide/intro-oauth-and-connected-apps.html) — query/update surface and current external-app authorization direction.
- [Meta Marketing API Insights](https://developers.facebook.com/docs/marketing-api/insights/) and [authorization](https://developers.facebook.com/docs/marketing-api/get-started/authorization/) — authoritative Ads endpoints to verify before implementation; both pages returned rate-limit responses in this review, so detailed permission claims are not verified here.
- [Meta Marketing API support update](https://developers.meta.com/blog/whats-new-in-marketing-api-support/) — current Business Manager/ad-account support context.
- [BambooHR Get Employee](https://documentation.bamboohr.com/reference/get-employee) — field- and record-level permission behavior and sensitive scope vocabulary.
- [DocuSign eSignature REST API](https://developers.docusign.com/docs/esign-rest-api/) — official API page opened but rendered no reference detail in this review; legal commitments remain excluded and endpoint details require a live check.
- [Microsoft AI Builder invoice flow](https://learn.microsoft.com/en-us/ai-builder/flow-invoice-processing) and [UiPath invoice model](https://docs.uipath.com/document-understanding/automation-cloud/latest/classic-user-guide/invoices-ml-package) — incumbent invoice-processing capability that makes OCR alone a weak differentiator.
- [Salesforce Agentforce Operations](https://help.salesforce.com/s/articleView?id=platform.automate_afsc_working_with_workflows.htm&language=en_US&type=5) — incumbent workflow orchestration relevant to Sales Ops.
- [T3 Code pinned review](https://github.com/pingdotgg/t3code/tree/9ea9c3d5d2c444133e3ddff40eecf38737951589), [Cua Driver pinned review](https://github.com/trycua/cua/tree/05f29785b508a4441ec3aa06c556a8e8b26c1d71), [OpenAI Codex pinned review](https://github.com/openai/codex/tree/7498521d288b9b3b96ffba4eedf089d8d6e06a84), and [Claude Code pinned review](https://github.com/anthropics/claude-code/tree/31a3b00bef145a0393d9dbf840a98674fec07712) — the repository's existing deep-dive review artifacts were loaded for these pinned commits; this pass was not a fresh full code audit. Current public pages were checked selectively, and no source was copied.
