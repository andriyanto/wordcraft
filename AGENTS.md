## About this repository

This is `andriyanto/wordcraft`: WordCraft developed on its own by its owner, from a copy of [`storytold/wordcraft`](https://github.com/storytold/wordcraft) at commit `118eb8a` (11 November 2026). It does not track the original and is not bound by the original project's house rules; the rules below are this repository's own. What has changed since the copy, and why, is in [`docs/technical-note-changes.md`](docs/technical-note-changes.md).

## Working rules

1. **The owner decides.** The owner merges pull requests and deletes their branches. An agent works on a branch, opens a pull request and reports; it does not merge.
2. **`main` changes through a pull request.** Do not push to it.
3. **Verify, don't guess.** Open a definition before using it. Confirm the cause in the code, even when someone names a location.
4. **Keep fixes narrow and tested.** Fix what was asked, where the bug is. Add a test that fails without the fix.
5. **Review the diff before opening a pull request.** Identifiers exist as used, the test fails without the fix, nothing unrelated changed, and the description is true.
6. **Report only what ran.** Say what you tested and what you could not run or see (a device, a platform, the desktop app). Without a local build or test, say so and rely on CI.
7. **Be honest about who did the work.** Commits and pull requests made with an agent say so: the agent is named as co-author. Never hide it.
8. **Stop and ask only about** licensing outside the allowlist, publishing, releases, a new remote, secrets, spending money, destructive git, sending anything to people outside this repository, and anything else hard to undo. Otherwise keep working. `plan/` is a gitignored notebook, not rules. Without it, do not ask for it and do not stop.
9. **Keep secrets and personal details out** of the repository, logs and requests.
10. **Guards say why.** A check that looks removable says what it prevents. Read that before changing it.
11. **Instructions must be readable by everyone.** No notes about one machine, and no links to files a reader cannot open.
12. **Security findings in someone else's code go to them privately first**, the way their security policy asks, before a fix is public here.

## Rust rules

1. **Never crash.** Non-test code does not panic and has no `unsafe`. Return errors, treat input as hostile, and bound recursion and allocations. New production crate roots copy the `deny` attribute from an existing crate: `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::unimplemented`, `clippy::todo`, `clippy::unreachable`. Every crash fix comes with a test that panicked before the fix.
2. **Clean room.** Learn from observed behaviour and public specifications. Do not copy another product's code, assets, strings or artwork. Check a dependency's licence before adding it.
3. **Assets and fonts.** Every asset is original or suitably licensed, and is recorded in `ATTRIBUTION.md` in the same change. Font files are not kept in this repository.
4. **Rust only.** No handwritten JavaScript or TypeScript, and no Tauri, Electron or webview shell. Preserve the existing web build.
5. **Time goes through `chrono`.** Do not hand-roll calendar conversion.
6. **The output must not change by accident.** A change meant to save time or memory is compared with the build before it: rendered pages and conversions byte for byte, and for the layout, against the document laid out from scratch (`wordcraft_layout::verify_against_full`).

## Licence allowlist

Dependencies and assets use SPDX identifiers. An expression is allowed when every identifier is in this list. Adding an identifier is the owner's decision.

- `ISC`
- `MIT`
- `BSD-2-Clause`
- `BSD-3-Clause`
- `Apache-2.0`
- `OFL-1.1`
- `Ubuntu-font-1.0`
- `Bitstream-Vera`
- `CC0-1.0`
- `CC-BY-4.0`
- `CC-BY-SA-4.0`
- `LicenseRef-Public-Domain`

`LicenseRef-Storyteller-Trademark` is not in this list. It covers Storyteller brand marks only, where a repository already says so.

# WordCraft — instructions for agents

WordCraft is a clean-room, open-source, Rust-native word processor targeting Microsoft Word parity — and going further on speed, openness and agent control. It runs natively on macOS, Windows, Linux and BSD, and on the web via WASM. Sibling projects of the original: `../photocraft` (Photoshop), `../vectorcraft` (Illustrator), `../filmcraft` (Premiere), `../lightcraft` (Lightroom), `../pdfcraft` (Acrobat), `../effectcraft` (After Effects), `../designcraft` (InDesign).

## Start every session here
Read [`docs/technical-note-changes.md`](docs/technical-note-changes.md) for what this repository changed and what is still open, then [`ROADMAP.md`](ROADMAP.md), [`docs/gaps.md`](docs/gaps.md) and [`docs/roadmap.md`](docs/roadmap.md) (those three are the original project's, as of the copy). `plan/` is an optional gitignored notebook. Do not ask for it and do not stop when it is absent.

## Never crash
People trust WordCraft with their writing; a crash loses their work. **This outranks feature work**: never ship a feature through a panic path, and fix a crash before building on top of it.
- **No panics in non-test code:** no `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`; no `unsafe` (`unsafe_code = "forbid"`).
- **Errors are `Result<T, E>`** through the crate's error type and `?`. An unfinished feature returns an error or is disabled; it never panics.
- **Input-derived numbers are hostile** (files, commands, MCP/control params): `get()` instead of `[i]`, slice strings only at char boundaries, checked/saturating arithmetic, clamp sizes, cap allocations, bound recursion.
- **Last-resort guard:** `Session::run` turns an escaped panic into an error and restores the document. It's a safety net, not a licence.
- **Prove it:** every crash fix lands with a regression test. `crates/engine/src/tests.rs::hostile_params_never_panic` runs every command with junk params — keep it green.
- Production crate roots carry `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`; new crates start with it. Tests may `unwrap` (`clippy.toml`).

## Assets: absolutely no Microsoft (or Adobe/Avid/Autodesk) iconography or images
**This rule is non-negotiable.** Breaking it is the worst mistake you can make in this repo.
- Never copy, trace, screenshot-crop or recreate icons, images, templates, themes, clip art, fonts, cursors, sounds or any other asset from Microsoft Word/Office or any Microsoft, Adobe, Avid or Autodesk product. Not even "temporarily".
- Every asset must be **original** (drawn in code — prefer this: `crates/ui-egui/src/icons.rs`), **public domain / CC0**, **OSI open source**, or **Creative Commons that allows redistribution**, or licensed open source by a contributor who made it.
- **Every asset file has a row in [`ATTRIBUTION.md`](ATTRIBUTION.md)** (author, source, licence). `cargo xtask assets` (part of `cargo xtask ci`) fails otherwise. Add the row in the same commit as the file.
- Fonts: new font files go to [`storytold/craft-fonts`](https://github.com/storytold/craft-fonts), never this repo; it's an optional build input (`CRAFT_FONTS_DIR`). Spelling word lists and hyphenation patterns live in [`storytold/craft-i18n`](https://github.com/storytold/craft-i18n) `language-data/`, never this repo; they are embedded from `CRAFT_I18N_DIR` (pinned by `craft-i18n.rev`, which CI checks out). Without it the app builds, but English spelling falls back to an installed Hunspell English or the built-in Moby word list (with no American/British variant table) and pattern hyphenation is off (Bulgarian, Turkish and Russian still hyphenate by rule), so proofing tests need it. Documents asking for proprietary fonts (Calibri, Aptos, Cambria…) are rendered with installed system fonts or metric-compatible open substitutes (`crates/fonts/src/word.rs`) — never bundle proprietary fonts.
- Templates, sample documents and themes are our own text and colours (`crates/engine/src/sample.rs`, `cmd/design.rs`). Demo images are public domain, with sources recorded.

## Clean room
- Microsoft Word is installed on the dev machine and may be **observed** black-box: run it, use its UI with synthetic documents, take screenshots **of its window only** (by window id — the machine runs other work; never capture the whole desktop) stored only under `plan/word/screenshots/` (gitignored, never committed).
- Never read, disassemble or copy anything inside the Word bundle (file names/listings only). Never commit files produced by Word. Never copy Microsoft wording beyond feature names.
- File formats come from public specs: ECMA-376 / ISO 29500 (OOXML), the published RTF spec, OASIS ODF 1.3. Never copy GPL/LGPL/AGPL code (LibreOffice, AbiWord, pandoc…).

## Architecture
| Layer | Crate | Job |
|---|---|---|
| L0 | `geom` | units, rects, measurement parsing |
| L1 | `doc`, `fonts` | document model + edit primitives; font DB, shaping, Word font substitution |
| L2 | `layout`, `docx`, `formats` | line breaking, pagination, tables, headers/footers, hit testing; OOXML I/O; TXT/MD/HTML/RTF/ODT |
| L3 | `render`, `pdf` | vello_cpu rasteriser; krilla PDF |
| L4 | `engine` | Session, undo, **command registry**, Word feature catalog, samples, I/O dispatch |
| L5 | `mcp` | MCP server (headless or bridged to the app) |
| L6 | `ui-egui` | Word-style egui front end (swappable) |
| apps | `apps/wordcraft`, `apps/wordcraft-cli`, `apps/wordcraft-web` | binaries |
| external | `craft-proof` (craft-libs, used as `wordcraft-proof`, sits at L1) | proofing: spelling, grammar, hyphenation; its word lists and patterns are craft-i18n's `language-data/` (`CRAFT_I18N_DIR`) |

- **Everything is a command.** User-visible behaviour = a `CommandSpec` in `crates/engine/src/cmd/*.rs` (id, label, ribbon location, shortcut, params doc, `enabled`, `run`) + tests. UI-only commands (`ui.*`) live in `crates/ui-egui/src/lib.rs`. The ribbon, shortcuts, command search, CLI, control channel and MCP all dispatch by id. Programmatic calls never open dialogs.
- **Parity is measured:** `crates/engine/src/catalog.rs` lists Word's ribbon/menu features with command ids. `cargo xtask parity` writes `docs/parity-checklist.md`; a test enforces a floor that only rises. Implement missing ids to raise it.
- **Layering** is enforced by `cargo xtask layers`. Nothing below L6 depends on egui/eframe/winit/rfd. The UI is thin and reads `Session` state; colours come from `theme::Tokens`.
- **Rust only** (no handwritten JS/TS). **Never break wasm** (`cargo xtask wasm`).

## Quality gates
Before every commit: `cargo xtask ci` (fmt, clippy -D warnings, tests, assets, layers, wasm). Commit after every arc of work that builds, with a task id in the message (`M2.1: line spacing dialog`).

## Typing behaves like Word
Typing, Enter, Backspace, Tab, lists, AutoCorrect and AutoFormat follow Word's observed behaviour in [`docs/typing-parity.md`](docs/typing-parity.md), pinned by `crates/engine/src/tests/typing.rs` (keys go through the same commands as the keyboard). Change behaviour, doc and tests together; a typing fix lands with a test there. Compare headlessly (`ui_shot`); don't drive GUIs with keystrokes on the shared machine.

## Running and looking at the app
- `cargo run --release -p wordcraft -- --sample --control 7981` (sample document + control channel).
- Drive it with JSON lines on `127.0.0.1:7981`, each with the window's key from `~/.config/wordcraft/control-key.local-7981` (macOS: `~/Library/Application Support/WordCraft/`), e.g. `{"id":1,"key":"…","method":"engine.execute","params":{"command":"text.insert","params":{"text":"Hello"}}}` then `{"id":2,"key":"…","method":"ui.screenshot","params":{"path":"/tmp/shot.png"}}`. Methods: `crates/ui-egui/src/control.rs`; docs: `docs/control-protocol.md` (Keys).
- **Headless window screenshots** (no focus stealing, works with a locked screen): `cargo run --release -p wordcraft-ui-egui --example ui_shot -- script.jsonl` (see the example's header).
- **For UI work, look at the result** (screenshot, read the PNG) and compare with Word side by side.
- CLI: `wordcraft-cli convert in.docx out.pdf`, `wordcraft-cli run --template sample --cmd 'select.text={"text":"Studio"}' --cmd format.bold --save out.docx`, `wordcraft-cli commands`, `wordcraft-cli parity`.
- MCP: `wordcraft-cli mcp` (headless) or `wordcraft-cli mcp --connect 127.0.0.1:7981` (drives the running app). See `docs/mcp.md`.
- Shell gotcha: `mv`/`cp` may be aliased interactive — use `/bin/mv -f` / `/bin/cp -f`.
- Parallel agents: separate `CARGO_TARGET_DIR` per agent; edit only the crates you own; delete your target dir when done (disk).

## Roadmap
Progress docs: `ROADMAP.md` (stage, headline numbers, progress log), `docs/target-app-parity.md` (the assessment), `docs/gaps.md` (ranked work list), `docs/roadmap.md` (milestones, Current focus), `docs/architecture.md`, and the parity checklists `docs/file-format-parity.md`, `docs/layout-parity.md`, `docs/typing-parity.md`, `docs/ui-parity.md`, `docs/hardware-parity.md`, `docs/localization-parity.md`. When work lands, update the gap, the parity doc and the progress log, and bump each touched doc's status line and revision history.

## Contributor credits (About window)

- About ▸ Contributors/Models are compiled into the binary from `contributors/contributors.json` (commit stats) and `contributors/people.toml` (the names people chose for themselves). See `docs/contributors.md`.
- In this repository `contributors.json` is a snapshot kept by hand: the original project's rows are left as they are, and this repository's own rows (its owner, and the model that helped) are added from its git history. When you update it, count from the history and say in the pull request where each number comes from. A release workflow that regenerates the file replaces the snapshot.
- Add or change a person's entry in `contributors/people.toml` only when that person asked, and record where they asked in `consent_source`. Never guess a name.

## CI workflows

- Most workflows in `.github/workflows/` call the shared workflows in [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions), **pinned to a commit hash**, not `@main`: a change there reaches this repository only through a reviewed bump of that hash.
- This repository may have workflows of its own where no shared one does the job (`audit.yml`).
- Change CI in a pull request of its own, not inside a feature or fix, and say what the change is for.
- Dependabot pull requests are merged only when the `gate` job is green; a bump that needs code changes is closed until the code is ported.

New workspace crates include `workspace = true` under `[lints]` in `Cargo.toml`. Production crate roots declare the crate-level lint attributes before modules; crate-level `//!` documentation may precede them.
