# Lumi Agents Product Thesis

Status: provisional market thesis; evidence date: 2026-09-19

This document is the source of truth for the market proof question. It does not
claim product-market fit, customer demand, pricing, or production readiness.

## Decision

Lumi should pursue verified digital labor for one bounded operational role before
expanding its role catalog. The provisional lighthouse is a **Finance Operations
Associate**, initially limited to read-only invoice and expense reconciliation,
policy checks, and exception reporting. It must not move money, approve
reimbursements, file taxes, make accounting judgments that require a licensed
professional, send external communications, or make legal or personnel
decisions.

This is a product-readiness choice, not a market conclusion. Finance leads the
initial test because the repository already contains two relevant workflow packs
and because the proposed first work units have structured inputs and observable
postconditions. A real tenant, access approval, measured human baseline, and
paid or otherwise consequential pilot evidence could overturn the choice.

## What Lumi is trying to replace

The economic unit is verified useful work, not tokens, seats, prompts, clicks, or
tasks started:

```text
primary output: verified work units completed
operating inputs: baseline minutes, residual minutes, rescue/support minutes
financial inputs: loaded labor value, runtime cost, support cost, deployment cost
```

The product earns the right to claim role capacity only when a manager can
delegate a recurring queue, inspect what was verified, see what was escalated,
and measure the human minutes that disappeared. The decision metric uses
consistent units:

```text
gross_labor_value_released
  = (baseline_minutes - residual_minutes - rescue_support_minutes) / 60
    * loaded_labor_cost_per_hour

net_value_created
  = gross_labor_value_released
    - runtime_cost - support_cost - allocated_deployment_cost
```

A fixture that passes is engineering evidence. It is not market evidence.

## Current truth boundary

### Confirmed in the repository

- The core execution invariant is models propose, policy authorizes, executors
  act, verifiers determine the actual outcome, and audit records evidence.
- The runtime has three fixture workflow packs: invoice reconciliation, expense
  report audit, and quote follow-up.
- The readiness report records fixture certification, but marks real-system
  canaries and measured human baselines as missing. It currently says V1 is
  NO-GO.
- The invoice reconciliation pack declares ERP and bank connector inputs,
  connector-only execution, a reconciliation artifact, and mismatch reporting.
- The expense audit pack declares an expense connector, a native receipt-reading
  step, and an external write that records an audit verdict. That write path is
  not part of the initial read/reporting canary.
- The quote follow-up pack declares CRM and email connectors and requires human
  approval before sending an external message.

### Not established

- No customer tenant, customer process owner, or customer system access is
  identified in this decision.
- No real connector adapter, authenticated session, or production-like run has
  established live reliability.
- The pack economics are declared hypotheses using `acme.test` fixtures. They do
  not establish demand, market size, customer labor rates, willingness to pay,
  or a price.
- No claim is made about how often Finance, Sales, Ads, HR, or Legal teams buy
  automation.

## Why this wedge and not generic agent features

Generic reasoning, browser automation, connectors, approvals, schedules, and
desktop control are becoming table stakes. Current official documentation shows
that Microsoft Power Automate can extract invoice fields through a prebuilt model
and feed them into a cloud flow, while UiPath Document Understanding supports
invoice extraction and schema customization, with human validation documented in
its broader framework. Finance cannot be won by adding another invoice OCR demo.

The testable Lumi wedge is narrower: own a repeatable reconciliation or audit
queue across the customer's existing systems, prove each match or exception,
refuse unverifiable actions, preserve least privilege, and expose the residual
human work. The value hypothesis is verified exception ownership, not document
extraction.

Sales Ops is also exposed to commodity workflow automation. Salesforce documents
REST query and record update primitives, and Agentforce Operations documents
reusable workflows that can start from integrations, email, or CSV and pause on
problems. Lumi therefore must not sell a generic follow-up bot; it would need to
prove verified queue ownership and approval-safe communication.

Facebook Ads Ops is a credible low-risk fallback because read-only reporting and
anomaly triage can exclude spend or campaign changes. It has less repository
leverage and a harder proof problem: an API response proves that data was read,
not that an anomaly diagnosis was correct. Meta's current Marketing API support
notice also shows that Business Manager context and ad-account identity matter
for support and integration operations.

## North-star proof loop

```text
real operator performs queue
  -> Lumi observes approved inputs and human baseline
  -> Lumi proposes a bounded routine
  -> policy and approval rules define the authority ceiling
  -> connector/browser/native executor performs the allowed steps
  -> postconditions prove the result or return refusal/ambiguity
  -> evidence records the result without continuous employee surveillance
  -> human handles exceptions
  -> measured minutes, cost, and failures update the Role Pack
```

The first deployment must leave behind reusable workflow configuration,
postconditions, an exception taxonomy, an eval corpus, and a measured economic
baseline. Customer-specific work should become configuration, adapter code,
rules, a workflow pack, or a regression case. If it remains bespoke service
work, the wedge is not productizing.

## Candidate pool

The candidate pool is intentionally wider than the formal comparison. It does
not mean these are active commitments.

| Candidate | Safe initial boundary | Main proof risk | Current status |
| --- | --- | --- | --- |
| Finance Ops | Reconciliation, expense checks, exception reports; no money movement | Sensitive access, cross-system mapping, incumbent automation | Provisional lead |
| Sales Ops | Quote/order intake, CRM hygiene, approved follow-up drafts | External communication approval and CRM schema variation | Formal alternative |
| Facebook Ads Ops | Read-only reporting, anomaly triage, recommendation drafts; no spend changes | Metric freshness, anomaly ground truth, Meta asset/token access | Formal alternative |
| HR Ops | Document and onboarding coordination; no candidate or employee judgments | Personal data, field-level permissions, employment sensitivity | Candidate, access unknown |
| Legal Ops | Intake completeness, matter checklists, draft routing; no legal advice or commitments | Privilege, confidentiality, legal review, commitment risk | Candidate, access unknown |
| Order-entry / procurement / support / reporting | Bounded administrative queues | No tenant, baseline, or current process evidence | Candidate pool only |

## Commercial thesis

The first commercial test must name four roles and one incumbent alternative:

| Role | Finance Ops hypothesis | Evidence required |
| --- | --- | --- |
| Actual user | AP or finance-operations analyst who processes invoices, expenses, and mismatches | Observed queue, SOP, 5–10 timed manual runs, and exception examples |
| Economic buyer | Controller, head of finance operations, or shared-services manager accountable for the queue | Confirmed labor baseline, budget owner, and willingness to fund a bounded pilot |
| IT/security veto | Finance-systems administrator, security, or data-protection owner | Tenant identity, least-privilege scopes, data-egress decision, deployment and retention approval |
| Process/compliance veto | Controller, audit, or compliance reviewer where policy interpretation is material | Escalation boundary and written confirmation that Lumi cannot approve or post financial commitments |
| Current alternative | ERP/expense-system features, spreadsheets and email, internal scripts, BPO/shared services, or Power Automate/UiPath | Current steps, human minutes, rework, switching cost, and why the queue remains manual |

These buyer and veto assignments are hypotheses, not customer evidence. The first
commercial test should sell a narrow measurable responsibility, such as “reconcile
this queue and produce an inspectable exception report,” rather than broad
employee replacement.

### Founder-led test

Build a list of **10–30 relevant prospects**, selected for a visible recurring
finance queue and a reachable process owner. Use existing relationships, warm
introductions, or a tightly researched list; do not buy broad demand data and do
not treat a reply as a pilot. The founder may send the message below later; this
agent has not sent it.

```text
Subject: Read-only reconciliation pilot for [process]

Hi [name] — I’m testing a narrow workflow that reconciles [invoice/expense queue]
inside the systems you already use and returns line-level matches, exceptions,
and evidence. It does not move money, approve reimbursements, or send vendors
messages. If this queue is relevant, could we review the current steps, volume,
and 5–10 timed examples? With an approved export or read-only staging access, I
can tell you within seven days whether a 30-unit canary is safe and worth running.
If it is not relevant, no action needed.
```

Working commercial thresholds for this test, not market benchmarks: from the
10–30 prospects, obtain at least three substantive workflow reviews, two approved
baseline/sample packages, and one process owner willing to evaluate a bounded
pilot. If the list cannot produce those signals, change the segment or message
before adding product scope.

### Pricing and renewal hypothesis

Do not invent a market rate. Test a paid pilot only after the buyer approves the
baseline and scope. Use the measured quantities above and record the hypothesis
explicitly:

```text
pilot_fee = max(measured_pilot_delivery_and_support_cost,
                alpha * gross_labor_value_released_during_pilot)
```

`alpha` is an experiment parameter, not an industry claim; use a pre-declared
10% or 20% test cell across separate pilots and record objections, conversion,
and measured value. Quote an actual amount only after the 5–10-run baseline and
pilot-cost estimate exist.

The ongoing model hypothesis is a monthly role-capacity or verified-volume fee,
renewed only when four consecutive weeks show positive `net_value_created`, at
least 95% verified completion, fewer than 5% unexpected rescue, correct
escalation, and a named buyer willing to fund the next period. A completed task
without measurable net value or renewal intent does not count as commercial
proof.

## Durable artifacts and boundaries

This document should remain the concise thesis and decision record. The companion
`docs/lighthouse-role.md` should hold the concrete role contract, access inputs,
work-unit schema, proof scorecard, thresholds, pilot sequence, and stop rules.
Do not create a larger market framework until one role produces live evidence.

## What would change the decision

Switch from Finance if another candidate can produce a real, read-only or
approval-safe tenant within the same seven-day test window and shows materially
better verified completion, lower supervision, or faster reuse. Switch to Ads Ops
if Meta read-only access is available but Finance access is not, provided the
team can define and verify anomaly outcomes instead of merely producing reports.
Switch to Sales Ops if an existing CRM queue and approval-controlled email path
are available and quote aging or follow-up work is measured.

Reject all candidates if access, baseline, or independent postconditions cannot
be obtained. Technical possibility is not a market proof.

## Evidence and references

Repository evidence inspected on 2026-09-19:

- [README](../README.md)
- [Roadmap](roadmap.md)
- [V1 readiness](v1-readiness.md)
- [Architecture](architecture.md)
- [Workflow pack contract](workflow-packs.md)
- [Invoice reconciliation pack](../packs/invoice-reconciliation/pack.json)
- [Expense report audit pack](../packs/expense-report-audit/pack.json)
- [Quote follow-up pack](../packs/quote-followup/pack.json)

Current official product documentation inspected on 2026-09-19:

- [Microsoft Power Automate invoice processing](https://learn.microsoft.com/en-us/ai-builder/flow-invoice-processing) — prebuilt invoice extraction and cloud-flow output fields.
- [UiPath Invoices ML package](https://docs.uipath.com/document-understanding/automation-cloud/latest/classic-user-guide/invoices-ml-package) — invoice fields, line items, and schema customization.
- [UiPath Document Understanding overview](https://docs.uipath.com/document-understanding/automation-cloud/latest/user-guide/about-document-understanding) — broader document-processing and human-validation framework context.
- [Salesforce REST API requests](https://developer.salesforce.com/docs/platform/api-rest/guide/intro-rest-resources.html) — query, read, update, and API authorization primitives.
- [Salesforce OAuth for external apps](https://developer.salesforce.com/docs/platform/api-rest/guide/intro-oauth-and-connected-apps.html) — current OAuth and external-client-app setup direction.
- [Salesforce Agentforce Operations workflows](https://help.salesforce.com/s/articleView?id=platform.automate_afsc_working_with_workflows.htm&language=en_US&type=5) — reusable workflows, integrations, pauses, and bulk starts.
- [Meta Marketing API support update](https://developers.meta.com/blog/whats-new-in-marketing-api-support/) — current support and Business Manager/ad-account context.
- [Meta Marketing API Insights](https://developers.facebook.com/docs/marketing-api/insights/) and [authorization](https://developers.facebook.com/docs/marketing-api/get-started/authorization/) — authoritative endpoints to re-check before an Ads pilot; both pages returned rate-limit responses during this review, so no detailed permission claim is treated as verified here.
- [BambooHR Get Employee](https://documentation.bamboohr.com/reference/get-employee) — current field-level and record-level permissions relevant to HR Ops.
- [DocuSign eSignature REST API](https://developers.docusign.com/docs/esign-rest-api/) — current envelope/API surface relevant to Legal Ops; signing and commitments remain out of scope.

Pinned reference reviews loaded from the repository's existing deep-dive artifacts
on 2026-09-19. This pass did not perform a fresh full clone/code audit at each
commit; current public pages were checked selectively where accessible, and no
source was copied:

- `pingdotgg/t3code` at `9ea9c3d5d2c444133e3ddff40eecf38737951589`: retain environment ownership of files, credentials, provider processes, and machine state; use capability negotiation and durable intent before side effects. Do not copy full event sourcing or Electron assumptions.
- `trycua/cua` at `05f29785b508a4441ec3aa06c556a8e8b26c1d71`: retain bounded manifests, exact `DELIVERED`/`REFUSED`/unproven outcomes, and empirical effect oracles. Do not treat generic desktop risk classes as business policy.
- `openai/codex` at `7498521d288b9b3b96ffba4eedf089d8d6e06a84`: retain explicit task lifecycle, scoped approval/sandbox separation, and lower authority for external content. Do not copy provider-specific types or use automated review as authority for financial or legal effects.
- `anthropics/claude-code` at `31a3b00bef145a0393d9dbf840a98674fec07712`: retain the distinction between tools, skills, agents, hooks, connectors, and workflow packs. Do not copy proprietary implementation or treat model-selected skills as workflow proof.
