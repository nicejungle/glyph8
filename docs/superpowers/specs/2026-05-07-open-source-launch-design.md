# Open-Source Launch — Design Spec

**Date:** 2026-05-07
**Owner:** nicejungle
**Repo:** `nicejungle/glyph8` (to be created, public)

## Goal

Take the existing Rust workspace (`glyph8`, a CLI NES emulator) from a private
local project to a presentable open-source project on GitHub, with:

1. Standard open-source repository hygiene (license, contribution docs, CI).
2. The repo pushed and public at `github.com/nicejungle/glyph8`.
3. A landing site published at `https://nicejungle.github.io/glyph8/` via
   GitHub Pages.

This is *launch hygiene*, not a product change. No emulator behavior is
modified. Stage 1.B beta is the snapshot we're shipping.

## Non-goals

- No new emulator features, no audio, no Stage 1.C input fixes — the launch
  ships current state with honest "beta limitations" framing.
- No release automation (cargo publish, GitHub Releases pipeline) — out of
  scope; can be added later.
- No multi-page docs site, no mdBook — single landing page only.
- No JS framework, no build step for the website.

## Deliverables

### 1. Repo metadata

- `Cargo.toml` workspace `repository` field: replace
  `https://github.com/<owner>/glyph8` with `https://github.com/nicejungle/glyph8`.
- Add `description`, `keywords`, `categories` to each crate's `Cargo.toml`
  (or workspace `[workspace.package]`) so `cargo publish` later is one step
  away. Keywords e.g. `["nes", "emulator", "cli", "terminal", "tui"]`,
  categories `["emulators", "command-line-utilities"]`.
- Add `homepage` field pointing to the Pages URL.

### 2. License files

Project already declares `MIT OR Apache-2.0` in `Cargo.toml`. Add the two
canonical license texts at repo root:

- `LICENSE-MIT` — standard MIT, copyright `2026 nicejungle`.
- `LICENSE-APACHE` — Apache-2.0 full text.
- Update `README.md` License section to point to both files and explain
  dual-license intent.

The bundled ROMs already carry their own licenses (`nestest.nes` public
domain, `boing.nes` CC BY 4.0). Add a `tests/roms/README.md` (or inline in
existing one) confirming attribution for `boing.nes` is preserved.

### 3. README upgrade

Keep current content; add at the top:

- One-line tagline + animated demo placeholder (`docs/assets/demo.gif` —
  placeholder file, real GIF can be dropped in later).
- Badges row: CI status, license (MIT OR Apache-2.0), MSRV (read from
  `rust-toolchain.toml` if present, else hardcode to current stable).
- "Status: Stage 1.B beta — see [Roadmap](#roadmap)" callout, replacing the
  current implicit framing.
- "Roadmap" section pulled from existing stage notes (1.B current, 1.C input,
  1.D audio, 1.E renderer modes, 1.F status bar UI).
- "License" section pointing to both license files.
- Link to website (GitHub Pages URL) in the header.

Existing "Beta limitations", "Controls", "Quick start" sections stay as-is.

### 4. Standard open-source files

- `CONTRIBUTING.md` — how to build (`cargo build --workspace`), test
  (`cargo test --workspace` + reference to `docs/qa-checklist.md`), lint
  (`cargo clippy --workspace -- -D warnings` + `cargo fmt --check`), the
  PR flow, and how stage-based development works (point to
  `docs/superpowers/`).
- `CODE_OF_CONDUCT.md` — Contributor Covenant 2.1, contact email left as
  `<owner email — TODO before publishing>`. Note: I'll prompt the user to
  fill this in before push.
- `CHANGELOG.md` — Keep a Changelog format. Single section `[0.1.0-alpha] -
  2026-05-07` summarizing what's done in Stage 1.A + 1.B.
- `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md` (standard
  templates, customized to mention "include `glyph8 --headless` output and
  terminal emulator + size when reporting").
- `.github/ISSUE_TEMPLATE/config.yml` disabling blank issues.
- `.github/PULL_REQUEST_TEMPLATE.md` — short checklist (tests pass, fmt+clippy
  clean, QA checklist consulted if relevant).

### 5. CI workflow

`.github/workflows/ci.yml`:

- Triggers: `push` to `main`, `pull_request` to `main`.
- Matrix: `ubuntu-latest`, `macos-latest`. (Skip Windows for now — terminal
  rendering on Windows needs separate validation; can add later.)
- Steps:
  1. `actions/checkout@v4`
  2. `dtolnay/rust-toolchain@stable`
  3. `Swatinem/rust-cache@v2`
  4. `cargo fmt --all -- --check`
  5. `cargo clippy --workspace --all-targets -- -D warnings`
  6. `cargo test --workspace`
- Single concurrency group per branch, cancel-in-progress on PR.

### 6. Website

**Tech:** Single static HTML + CSS file, no JS framework, no build step.
Optional: a few lines of vanilla JS for the asciinema embed (if used).
Files live in `docs/site/` so the Pages workflow can deploy that directory
directly.

**Style:** Terminal / CRT aesthetic, echoing the project's "NES inside your
terminal" identity. Concretely:

- Dark background (`#0b0d0c` near-black).
- Monospace stack: `"JetBrains Mono", "Fira Code", "SF Mono", Menlo,
  Consolas, monospace`.
- Accent colors: phosphor green `#7fff9c` for primary text/headings,
  amber `#ffb454` for highlights/links, muted gray `#8a8f8c` for body.
- Subtle CRT scanline overlay (CSS `repeating-linear-gradient` at very low
  opacity, `pointer-events: none`, can be toggled off via `prefers-reduced-motion`).
- Soft glow / text-shadow on the hero title only (kept restrained — no
  cheesy flicker).
- All sections inside a single max-width column (~720px) with generous
  vertical spacing.

**Sections (top to bottom):**

1. **Hero** — `glyph8` ASCII-style title, tagline ("CLI NES emulator that
   renders to your terminal in 24-bit color halfblocks"), `cargo install`
   one-liner in a copy-clickable code block, "View on GitHub" link.
2. **Demo** — placeholder `<img>` for `demo.gif` or asciinema embed slot;
   captioned with the example commands.
3. **Features** — 4-6 short bullets: 24-bit halfblock renderer, adaptive
   sizing, headless determinism, Mednafen-style keymap, MIT OR Apache-2.0,
   pure Rust workspace.
4. **Install & Run** — block mirroring README Quick Start.
5. **Controls** — same table as README.
6. **Roadmap** — Stage 1.B (current) → 1.C → 1.D → 1.E → 1.F, each with one
   line.
7. **Status & Limitations** — short honest list (no audio, no key-release,
   no commercial ROMs bundled).
8. **License & Credits** — dual license note, ROM attributions, link to
   `boing.nes` author Brad Smith.
9. **Footer** — GitHub link, license, copyright line.

**Accessibility:**

- Body text contrast ≥ 4.5:1 against background.
- Links underlined (not color-only).
- `prefers-reduced-motion: reduce` disables scanline animation if any.
- Skip-link to main content.
- Semantic HTML: `<header>`, `<main>`, `<section>` with `aria-labelledby`,
  `<nav>`, `<footer>`.

**SEO / social:**

- `<title>`, `<meta description>`, OpenGraph tags.
- `og:image` placeholder at `docs/site/assets/og.png` (1200×630, can be a
  screenshot of the emulator in a terminal — placeholder file with a TODO
  note for now).
- `favicon.ico` from a tiny pixel-art "g8" — placeholder PNG fine for v1.

### 7. Pages deployment workflow

`.github/workflows/pages.yml`:

- Trigger: `push` to `main` affecting `docs/site/**`, plus `workflow_dispatch`.
- Permissions: `pages: write`, `id-token: write`.
- Uses `actions/configure-pages@v5` + `actions/upload-pages-artifact@v3`
  (uploading `docs/site/`) + `actions/deploy-pages@v4`.
- Single job; concurrency group `pages` cancel-in-progress disabled (we
  don't want partial deploy queueing chaos).
- Pages source must be set to "GitHub Actions" in repo settings; if the
  initial push doesn't auto-set this, the workflow will fail on first run
  and the `gh` CLI step (or manual settings click) handles it.

### 8. Push sequence

1. All file changes committed locally on the worktree branch.
2. Verify clean: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
3. Merge worktree branch into `main` (or open PR if user prefers — for an
   initial public push, direct commit on `main` before `gh repo create` is
   simplest; the worktree branch can just be the initial state of `main`).
4. `gh repo create nicejungle/glyph8 --public --source=. --push --description "..."`.
5. Wait for CI + Pages workflow to go green.
6. If Pages source isn't auto-set: `gh api -X POST repos/nicejungle/glyph8/pages -f build_type=workflow` (or instruct the user to flip it in Settings → Pages).
7. Add the live Pages URL to repo "About" via `gh repo edit --homepage`.

### 9. Repo "About" metadata

Set via `gh repo edit nicejungle/glyph8`:

- Description: `CLI NES emulator that renders to your terminal in 24-bit color halfblocks`
- Homepage: `https://nicejungle.github.io/glyph8/`
- Topics: `nes`, `emulator`, `rust`, `cli`, `terminal`, `tui`, `retro`,
  `halfblock`.

## Open questions / pre-flight gates

The following must be resolved before push:

- **Code of Conduct contact email** — placeholder needs a real address.
  Default: ask user; if they decline, use the GitHub username's noreply
  address (`<id+nicejungle@users.noreply.github.com>`) or a generic
  "open an issue" instruction. Resolve before push.
- **Author copyright line** — `Copyright (c) 2026 nicejungle` is the safe
  default; user can swap to a real name if preferred. Confirm with user
  before writing license files.
- **demo.gif** — ship with placeholder; real recording can come later
  without blocking launch.

## Out of scope (explicitly deferred)

- `cargo publish` to crates.io — needs unique crate names (`nes-core`,
  `nes-render` etc. may collide), and user hasn't asked for it.
- Release automation, semantic-release, conventional commits enforcement.
- Multi-page docs / API docs hosting (`cargo doc --no-deps` to Pages).
- Discussions, GitHub Sponsors, security policy (`SECURITY.md`) — can be
  added incrementally.
- Cross-platform CI for Windows.
- Translations of README / website.
