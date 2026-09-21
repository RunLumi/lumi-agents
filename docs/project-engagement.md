# Project task engagement and local automations

This change adds a multiline composer, explicit tool selection, a bounded
managed-browser reader, durable project automations, and a bounded native
computer adapter for a verified local Cua installation. It is not a claim that
authenticated Chrome or live native-computer canaries are complete.

## User journey

Open a project and select **Tasks**. Enter keeps paragraphs; Cmd/Ctrl+Enter runs.
The Tools button and `@` picker share the runtime capability list. Drafts are
saved in local application storage separately for each project. A submitted
request keeps its identity so retrying a failed Run request does not create a
second task. Secrets should not be pasted into task drafts.

Files are enabled within the opened project. Shell is an explicit manual choice,
not an OS sandbox. The Browser tool needs exact approved public HTTPS origins,
the matching worker/Playwright installation, and a successful **Check browser**.
Changing website scope pauses affected browser automations for re-authorization.

The managed browser can read public pages and return bounded text and links to
the agent, which can create a new verified project file. JavaScript, sign-in,
forms, uploads, personal Chrome profiles and browser write effects are deliberately
unavailable on this path. `@chrome` requires an explicit Settings attach of one
selected Chrome window and approved origins; it never aliases the managed reader.
`@computer` requires the fixed-path, digest-verified Cua 0.28.2 installation and otherwise reports setup
required rather than silently executing through shell. Natural-language goals work with
the selected capabilities; unknown capabilities are never inferred as grants.

Development setup uses the existing pinned worker package under
`workers/playwright`. Install that package and its matching Chromium through the
reviewed developer setup. The host verifies the reader and network-helper bytes
against the compiled build before launching. Release packaging must provision
Node, the pinned Playwright dependency graph and browser binaries; this change
does not certify a bundled signed distribution.

## Automations

Each project has **Automations**, also reachable from the sidebar. Create a
one-time, interval, daily or weekly definition. Preview the next three instants
in an IANA timezone. A missing daylight-saving wall time is skipped; a repeated
wall time fires at the earlier instant once. The explicit catch-up policy either
skips a missed occurrence or runs once, never a burst. Interval schedules retain
their original anchor.

Enablement requires an explicit expiring authorization, at most 30 days. Each
run is bounded to 20 minutes, the normal task budget and the selected file/browser
capabilities. Unattended shell, Chrome and computer actions are refused. A live
pause/edit/delete revokes the current automation's remaining authority; already
completed effects are not undone. Run now creates a separately identifiable
manual occurrence. Successful tasks offer **Save as automation**.

The engagement store is the single Project Automation definition, occurrence,
and admission store for this path. The scheduler runs **only in the running
local application**. Closing or powering off the device does not move work to
the cloud. Provider persistence is opt-in and uses the existing OS-backed
credential broker on macOS and Windows; session-only configuration remains
available when the user does not opt in. Browser setup must still be checked
again after restart.

Definitions, request identities, admitted occurrences and task bindings are
atomically persisted outside project content. An OS lock prevents a second
local owner. Corrupt state, failed writes, expired authority and emergency stop
fail closed. A crash after an occurrence is claimed pauses that automation and
marks its interrupted run for review; it does not replay possible side effects.
Automation run history links back to ordinary Tasks and their evidence.

## Trust boundaries and limitations

All tools build normal ActionProposals and execute through the same orchestrator,
policy, persistence and verification path. Browser observations are untrusted.
The browser read adapter allows only exact approved HTTPS origins, pins a checked
public IPv4 at connection time, strips credentials/cookies, blocks non-GET traffic,
limits requests/bytes, and refuses new model-constructed URLs unless a URL came
from the user's goal or an observed link. These are adapter controls, not an OS
network sandbox or a proof against all browser vulnerabilities.

The durable scheduling host is the engagement adapter for desktop task admission.
There is no second desktop automation store or shell tick path. Event/webhook
triggers, remote scheduling, authenticated/browser-write flows, live native
canaries, per-task live takeover and full interaction UI coverage remain separate
work.

## Validation

Pure Node tests cover composer shortcuts, IME handling, mention boundaries,
project drafts and public-network admission. The controlled Chromium canary
checks actual DOM observations, disabled scripts, rejected interactive actions
and forbidden destinations. Rust tests cover schedule DST behavior, expiry,
optimistic edits, lock ownership, request identity and interrupted occurrences.

These tests are not authenticated customer workflows, live-provider reliability,
signed macOS/Windows release evidence or measured customer ROI. Do not change the
repository's NO-GO release status on the strength of this feature alone.
