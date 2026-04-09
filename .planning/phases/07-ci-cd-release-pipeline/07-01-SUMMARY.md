---
phase: 07-ci-cd-release-pipeline
plan: 01
subsystem: infra
tags: [github-actions, release, ci-cd, homebrew, macos, cross-arch]

requires:
  - phase: 06-project-rename
    provides: aliast-daemon binary and aliast.plugin.zsh ready for distribution
provides:
  - Tag-triggered GitHub Actions release workflow
  - Matrix build on native macOS runners (macos-15 ARM64, macos-15-intel x86_64)
  - GitHub Release with tarballed binaries, plugin file, and SHA256SUMS
affects: [08-homebrew-tap-formula, release-automation, distribution]

tech-stack:
  added:
    - GitHub Actions (workflow)
    - dtolnay/rust-toolchain@stable
    - Swatinem/rust-cache@v2
    - actions/upload-artifact@v4
    - actions/download-artifact@v4
    - softprops/action-gh-release@v2
  patterns:
    - Native-runner matrix build per macOS architecture
    - Artifact-passing handoff between build and release jobs
    - Flat-tarball packaging for Homebrew formula consumption

key-files:
  created:
    - .github/workflows/release.yml
  modified: []

key-decisions:
  - "Native macOS runners (macos-15 ARM64, macos-15-intel x86_64) instead of cross-compilation for arch-specific builds"
  - "Hand-written GitHub Actions workflow (~68 lines) instead of cargo-dist per stack decision D-02"
  - "Flat tarball structure so brew install extracts aliast-daemon directly at formula install time"

patterns-established:
  - "Release workflow: tag trigger -> matrix build -> artifact upload -> single release job packages and publishes"
  - "Binary path target/release/<bin> (no --target flag) because each runner natively produces the correct architecture"
  - "sha256sum on ubuntu-latest release job for predictable tooling availability"

requirements-completed: [CI-01, CI-02, CI-03]

duration: 4min
completed: 2026-04-04
---

# Phase 7 Plan 01: CI/CD Release Pipeline Summary

**Tag-triggered GitHub Actions release workflow with native-runner matrix build (macos-15 + macos-15-intel) and softprops/action-gh-release publishing aliast-daemon tarballs, the zsh plugin, and SHA256SUMS**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-04-04T21:27:05Z
- **Completed:** 2026-04-04T21:30:00Z (approx, after human verification)
- **Tasks:** 2 (1 automated + 1 human-verify checkpoint)
- **Files modified:** 1

## Accomplishments

- Created `.github/workflows/release.yml` triggered on `v*.*.*` tag push
- Matrix build produces arch-specific binaries on native macOS runners (no cross-compilation complexity)
- Release job assembles flat tarballs, copies the plugin file, generates SHA256SUMS, and publishes a GitHub Release via softprops/action-gh-release@v2
- Verified end-to-end: user confirmed workflow triggered on a test tag push, both matrix builds completed, and the release contained all 4 expected assets

## Task Commits

Each task was committed atomically:

1. **Task 1: Create release workflow** - `ed88595` (feat)
2. **Task 2: Verify release pipeline end-to-end** - human-verify checkpoint, user approved (no file change required)

**Plan metadata:** (closure commit hash filled at time of commit)

## Files Created/Modified

- `.github/workflows/release.yml` - Tag-triggered release workflow: matrix build on macos-15 (ARM64) and macos-15-intel (x86_64), artifact passing, tarball packaging with plugin + SHA256SUMS, GitHub Release creation

## Decisions Made

- **Native macOS runners over cross-compilation:** `macos-15` (ARM64) and `macos-15-intel` (x86_64) each build their own architecture natively. No `--target` flag, no lipo, no universal binary complexity. Binary lands at `target/release/aliast-daemon` directly.
- **Hand-written workflow over cargo-dist:** ~68 lines of hand-written YAML vs cargo-dist's 2000+ line generated scripts. macOS-only with one binary doesn't justify cargo-dist's complexity.
- **Flat tarball structure:** `tar -C artifacts/... aliast-daemon` produces archives where `aliast-daemon` extracts directly into cwd, matching what the Phase 8 Homebrew formula expects.
- **`sha256sum` runs on ubuntu-latest** (release job) because it is standard GNU coreutils there; macOS has `shasum` instead.
- **Explicit `permissions: contents: write`** at workflow level to avoid 403 errors when softprops/action-gh-release creates the release.
- **Unique artifact names** via `${{ matrix.target }}` suffix to satisfy upload-artifact v4 requirements (no duplicate names across matrix entries).

## Deviations from Plan

None - plan executed exactly as written. The workflow YAML matches the plan's embedded template verbatim.

## Issues Encountered

None during execution. The human-verify checkpoint was the required gate and passed on first attempt.

## User Setup Required

None - the workflow uses `GITHUB_TOKEN` supplied by the runner context and requires no additional secrets. Publishing a release only needs the explicit `permissions: contents: write` already set at the workflow level.

## Notes on Requirements Tracking

Plan frontmatter declares `requirements: [CI-01, CI-02, CI-03]` but these IDs are not yet present in `REQUIREMENTS.md` (which currently enumerates only the v1.0 INFRA/SUGG/NL/TERM families). `requirements mark-complete` reported them as `not_found`. Phase 7's success criteria map 1:1 to the `must_haves.truths` in the plan frontmatter and are all satisfied:

1. Tag push to `v*.*.*` triggers the release workflow automatically -- verified
2. Workflow produces both `aliast-daemon-aarch64-apple-darwin.tar.gz` and `aliast-daemon-x86_64-apple-darwin.tar.gz` -- verified
3. GitHub Release contains tarballed binaries, plugin file, and SHA256SUMS -- verified

If CI-01/02/03 need to live in `REQUIREMENTS.md` as first-class v1.1 milestone requirements, add them in a follow-up pass; the execution is complete.

## Next Phase Readiness

- Phase 8 (Homebrew Tap + Formula) is unblocked. Formula can point at release URLs of the form `https://github.com/ChrisWoo0443/AliasT/releases/download/v<version>/aliast-daemon-<arch>-apple-darwin.tar.gz`.
- Flat tarball layout matches the expectations of a standard Homebrew `bin.install "aliast-daemon"` line.
- SHA256SUMS is available to copy the per-arch checksums into the formula's `sha256` fields.
- No blockers for Phase 8.

## Self-Check: PASSED

- `.github/workflows/release.yml` -- FOUND
- `.planning/phases/07-ci-cd-release-pipeline/07-01-SUMMARY.md` -- FOUND
- Commit `ed88595` -- FOUND in `git log`

---
*Phase: 07-ci-cd-release-pipeline*
*Completed: 2026-04-04*
