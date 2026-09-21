# 34. Computer use

Status: **Native contracts and bounded project engagement adapter implemented; live canary pending**
Date: 2026-09-21

## Implemented

`lumi-native` defines Lumi-owned semantic target, permission, cancellation,
revocation, effect-oracle and outcome contracts with fixture certification for
macOS/Windows expectations.

## Project engagement integration

`@computer` is admitted only for interactive tasks when the host finds the
reviewed Cua Driver 0.28.2 at a fixed installation location and its executable
matches the anchored release digest. The model receives one Lumi-owned
`computer_use` tool with semantic application/window/control targets. The tool
does not expose coordinates, arbitrary Cua calls, shell fallback or hidden
foreground escalation. Scheduled automation remains refused because native
computer control is not unattended-safe.

When the driver is absent or the digest does not match, the capability is
`Needs setup` and the task is refused before execution.

Each action binds one exact application/window, requires current OS permissions,
uses a fresh semantic snapshot, revalidates before mutation, checks revocation
and expiry, records Lumi action outcomes, and reads back post-state. Stop ends
remaining authority; it does not undo delivered effects.

## Acceptance

The native contract and adapter tests cover denied permissions, wrong window,
stale snapshots/generations, ambiguous resolution, and effect readback. A live
macOS canary with a real installed Cua Driver and fixture app remains required;
Windows remains blocked until its exact executable pin and adapter canary are
reviewed. Fixture-driver tests alone are not native proof.
