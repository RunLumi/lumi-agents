# 34. Computer use

Status: **Native contracts implemented; Project engagement adapter planned**
Date: 2026-09-21

## Implemented

`lumi-native` defines Lumi-owned semantic target, permission, cancellation,
revocation, effect-oracle and outcome contracts with fixture certification for
macOS/Windows expectations.

## Not yet wired into project engagement

`@computer` is surfaced as unavailable. The desktop engagement runner must not
fall back to shell, browser reading, global pointer input or foreground activation.

When wired, a run must bind one exact application/window, require current OS
permissions, use a fresh semantic snapshot, revalidate before mutation, propagate
deadline/cancel/revocation, record actions, and read back post-state. Stop must
end remaining authority; it does not undo delivered effects.

## Acceptance

Named macOS and Windows fixture applications need real adapter canaries for denied
permissions, wrong window, lock/disconnect, cancellation, deadline, effect
readback and emergency stop. Fixture-driver tests alone are not native proof.
