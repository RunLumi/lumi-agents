# 23 — User Experience, Handoff & Control v1

Status: Normative product contract

## 23.1 Goal

Make delegation feel like working with a strong colleague, not babysitting a cursor.

## 23.2 Task creation

User MUST be able to express:

- goal;
- desired output;
- constraints;
- deadline optional;
- allowed systems/resources;
- approval preferences where permitted.

System SHOULD show inferred plan at appropriate granularity.

## 23.3 Progress

Task view SHOULD show:

- current phase;
- completed milestones;
- active system/app;
- pending approvals;
- exceptions;
- artifacts;
- time;
- cost/budget;
- evidence summary.

Do not stream noisy internal reasoning.

## 23.4 Approval UX

Approval MUST show business effect, not raw technical gesture.

Good:
- Send quote to customer@example.com for 12,500,000 VND.

Bad:
- Click button at (812, 477).

## 23.5 Exception UX

Exception card SHOULD show:

- what blocked;
- why;
- safe choices;
- consequences;
- evidence;
- suggested next step.

## 23.6 Handoff

User MUST be able to:

- pause;
- take over;
- edit goal/constraints;
- approve/reject;
- supply missing information;
- resume;
- cancel.

Material goal change MAY require replanning/policy.

## 23.7 Takeover

When user takes over native/browser session:

- automation MUST pause conflicting input;
- state SHOULD remain observable;
- resume MUST re-observe state before continuing.

## 23.8 Completion

Final result SHOULD include:

- concise outcome;
- artifact links;
- exceptions/unresolved items;
- evidence/provenance;
- actions taken;
- approvals used;
- relevant cost/time.

## 23.9 Trust language

UI MUST distinguish:

- planned;
- attempted;
- executed;
- verified;
- ambiguous.

Never collapse "attempted" into "done".

## 23.10 Background visibility

Background/unattended local execution SHOULD be visible through tray/status.

User MUST have local stop.

## 23.11 Privacy

Capture indicators SHOULD appear when screen/microphone capture active beyond ordinary selective evidence.

## 23.12 Provider choice

Advanced user/admin MAY select allowed provider preference.

UI MUST communicate when policy forces local-only or specific provider restrictions.

## 23.13 Artifacts

Draft artifact and published/sent artifact MUST be visually distinct.

## 23.14 Notifications

Notifications SHOULD be reserved for:

- approval;
- exception;
- completion;
- failure;
- material budget issue.

Avoid notification spam for routine steps.

## 23.15 Accessibility

Desktop app SHOULD meet accessible keyboard/navigation/contrast expectations appropriate to platform.

## 23.16 Tests

V1 UX tests SHOULD cover:

- approval clarity;
- pause/takeover/resume;
- ambiguous state;
- local stop;
- permission onboarding;
- background task visibility;
- artifact draft vs publish;
- failed verification shown honestly.
