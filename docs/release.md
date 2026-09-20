# Release process — Lumi Agents desktop (macOS & Windows)

The `.github/workflows/release.yml` pipeline builds the desktop app for
macOS (universal: Apple silicon + Intel) and Windows (x64), and on a
version tag publishes everything to a GitHub Release with SHA-256
checksums.

```text
tag vX.Y.Z pushed ──► release.yml (matrix build)
                      ├─ macos-universal : npm build → tauri build (universal) → .dmg
                      ├─ windows-x64     : npm build → tauri build (x64)       → .exe + .msi
                      └─ github-release  : SHA256SUMS.txt + bundles → GitHub Release
```

## 1. Cut a release

1. **Make sure `main` is green** (all CI checks passing) and contains
   what you intend to ship.
2. **Bump the version** in *both* files, keeping them equal:
   - `apps/desktop/src-tauri/tauri.conf.json` → `version`
   - `apps/desktop/ui/package.json` → `version`
3. Commit on `main`, e.g. `chore(release): v0.2.0`.
4. **Tag and push:**

   ```sh
   ver=0.2.0   # must equal the files above, without the "v"
   git tag "v$ver" && git push origin "v$ver"
   ```

   The workflow fails fast with a clear error if the tag and
   `tauri.conf.json` version disagree.

## 2. What the pipeline does

Per platform (matrix, `fail-fast: false` so one platform failing never
hides the other's results):

1. Pinned Rust 1.98.1 + pinned Node 22 (the same toolchains CI enforces).
2. `npm ci` + `npm run build` for the React frontend into
   `apps/desktop/ui/dist` (lockfile-enforced; no version drift).
3. `cargo fetch --locked` — desktop dependency graph must match
   `Cargo.lock` exactly.
4. `tauri build` → unsigned release bundles.

Artifacts produced:

| Job | Bundle | Path pattern |
|---|---|---|
| macos-universal | DMG (arm64 + Intel) | `Lumi Agents_X.Y.Z_universal.dmg` |
| windows-x64 | NSIS installer | `Lumi Agents_X.Y.Z_x64-setup.exe` |
| windows-x64 | MSI installer | `Lumi Agents_X.Y.Z_x64_en-US.msi` |

On a **tag push** a final job downloads all bundles, writes
`SHA256SUMS.txt`, and publishes a GitHub Release with auto-generated
notes. On a manual **workflow_dispatch** run the bundles are uploaded
as workflow artifacts only — nothing is published. Use a manual run to
validate the pipeline before cutting a real tag:

> *Actions* → *release* → *Run workflow* → pick `main`.

## 3. Verify before sharing a release

1. All matrix jobs green; `SHA256SUMS.txt` attached.
2. Checksums match locally: `shasum -a 256 -c SHA256SUMS.txt`.
3. **macOS:** open the DMG, launch the app, open a real folder as a
   project, delegate a task, check the Files tab and ⌘K palette.
4. **Windows:** run the setup exe, repeat the same smoke pass.
5. Confirm the release notes list the merged PRs you expect.

## 4. Signing status (important, currently unsigned)

Bundles are **not code-signed** — no Apple Developer ID or Windows
 Authenticode certificate is configured. That means:

- **macOS:** first launch requires a right-click → *Open*, or
  `xattr -cr "/Applications/Lumi Agents.app"` after install (Gatekeeper
  has no notarization ticket to check).
- **Windows:** SmartScreen shows an "unknown publisher" warning;
  users choose *More info → Run anyway*.

Adding signing later is config-only, no workflow changes: set the
standard Tauri secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`; Windows via
`certificateThumbprint` on a Windows runner) and the same pipeline
signs and notarizes. Until then, ship with the checksum file and
distribute it through the same channel as the installers.

## 5. Rollback

- A **bad tag** (build failed / wrong version): delete and re-push it —
  `git push origin :refs/tags/vX.Y.Z`, fix, re-tag. The workflow
  re-runs and updates the same release.
- A **published but broken release**: mark it a prerelease or delete
  the release in the GitHub UI (keep the tag), fix on `main`, cut the
  next patch tag. Never overwrite files on an existing release users
  may have checksummed.
- A **bad version on main**: `git revert` the release-bump commit.

## 6. Local parity builds

Same steps CI runs, for testing on your own machine:

```sh
# macOS (universal needs both targets: rustup target add aarch64-apple-darwin x86_64-apple-darwin)
cd apps/desktop/ui && npm ci && npm run build
cd .. && ui/node_modules/.bin/tauri build --target universal-apple-darwin   # or --debug for a quick local test
# → apps/desktop/src-tauri/target/<target>/release/bundle/dmg/*.dmg
```

## 7. Alignment with AGENTS.md

- Actions pinned by commit SHA (checkout/setup-node/upload-artifact
  reused from `ci.yml`; download-artifact and action-gh-release
  resolved from their git tags at adoption).
- Locked dependencies: `npm ci`, `cargo fetch --locked`, tag/version
  guard.
- Release integrity: SHA-256 checksum file on every release.
- `apps/desktop/THIRD_PARTY_NOTICES.md` ships in the repository.
- Deferred (tracked): SBOM generation and code signing/notarization —
  both are additive steps in this pipeline, not process changes.
