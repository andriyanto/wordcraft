# Technical note: what this repository changed from the original

This note records what was changed in `andriyanto/wordcraft` since it was copied from the original project, why, how each change is designed, and how the result compares with the starting point. Written 13 November 2026. The Indonesian version is [`nota-teknis-perubahan.md`](nota-teknis-perubahan.md).

## 1. Scope

| | |
|---|---|
| Original repository | `storytold/wordcraft` |
| Starting point | commit `118eb8a` (11 November 2026), version 0.15.1 |
| This repository | `andriyanto/wordcraft`, developed independently from the starting point |
| Period of work | 11–13 November 2026, pull requests #6 to #29 |
| How the work was done | By the repository owner with an AI coding agent (Claude); commits name it as co-author |

Two limits on how to read this note:

- **"The original" means its state at commit `118eb8a`.** Changes the original project made after that were not examined and are not compared.
- **Performance figures were measured on release builds with synthetic documents** (plain text paragraphs) on the development machine. They show before and after; they are not a promise for real documents. The measuring tool is outside the repository.

The version number has not been changed: it is still 0.15.1, the same as the original.

## 2. Comparison at a glance

| Aspect | Original (`118eb8a`) | This repository |
|---|---|---|
| Size of `Paragraph` in memory | 928 bytes | 104 bytes |
| Size of `Run` in memory | 376 bytes | 16 bytes |
| Opening 999,000 empty paragraphs | 1,314 MB | 522 MB |
| Layout memory, 993 pages | 359 MB | 259 MB |
| Peak memory reading a DOCX, 20,000 formatted paragraphs | 182 MB | 117 MB |
| Relayout per keystroke, 993 pages | 56 ms | 11 ms (before incremental pagination) |
| Relayout per keystroke, 2,837 pages | 161 ms | 14–19 ms |
| Status-bar counts per keystroke, 2,837 pages | 85 ms | 2.3 ms |
| A 27 KB DOCX holding 2 million paragraphs | used 2.6 GB, process killed | refused with an error |
| Headless MCP server | reads and writes any file | confined to its working folder |
| Where the app looks for updates | `storytold/wordcraft` releases | `andriyanto/wordcraft` releases |
| Second document window on an Adreno GPU | listed but not shown | shown (through OpenGL) |
| After File › Export | stays on the Export page, says nothing | returns to the document with a notice |
| Live preview from the Home ribbon | none | font, size, colours, styles (PR #28) |

What the app outputs did not change: for every optimisation, rendered pages and conversions were compared byte for byte with the build before the change (section 9).

## 3. Security and robustness (PR #6)

The result of an initial audit of the code and its dependencies.

| Finding | Fix |
|---|---|
| A small DOCX could exhaust memory: 27 KB holding 2 million `<w:p/>` used 2.6 GB | A budget, `MAX_CONTENT` = 1,000,000 paragraphs and runs per file, nested documents included; past it the file is refused |
| An OLE object with no preview picture was dropped on read | The object and its data are kept, shown as an empty box, and written back on save |
| The headless MCP server could read and write any file | By default it is confined to the folder it was started in; full access only with `--allow-all-files` |
| Dependencies with advisories | `tokio` 1.54.1, `async-executor` 1.14.1, later `rustix` 1.1.6 (PR #9) |
| No regular advisory check | A daily `audit` workflow and a policy in `.cargo/audit.toml` |
| Shared workflows referenced at `@main` | Pinned to a commit hash |

Also added: four libFuzzer targets in `fuzz/` (docx, doc, other formats, metafile). They compile; they have never been run.

Still open: before refusing a bomb file, the reader still uses 1–2 GB for the XML tree. The `zip` advisory is ignored with a recorded reason, because the affected function is not called.

## 4. Document model memory: shared properties (PR #7)

**Problem.** Every paragraph, run, table and cell held a full copy of its formatting properties, though most are identical or empty.

**Design.** `Shared<T>` in `crates/doc/src/shared.rs`:

- The value sits behind an `Arc`. Reading goes through `Deref`; changing goes through `DerefMut`, which first copies the value if it is shared.
- The default (empty) value is one allocation for the whole process.
- Other values are looked up among the last 16 per thread, so consecutive paragraphs with the same formatting share one copy.
- Serialisation is exactly that of the wrapped value, so the JSON format is unchanged.

Used for `ParaProps`, `CharProps`, `CellProps` and `TableProps`.

| Structure | Before | After |
|---|---|---|
| Paragraph | 928 bytes | 104 bytes |
| Run | 376 bytes | 16 bytes |
| Table | 368 bytes | 88 bytes |
| Cell | 256 bytes | 64 bytes |

| Opening a file | Before | After |
|---|---|---|
| 999,000 empty paragraphs | 1,314 MB | 522 MB |
| 200,000 empty paragraphs | 324 MB | 165 MB |
| 400,000 table cells | 490 MB | 367 MB |

The limit: after many paragraphs are edited one by one, each has its own copy again and memory rises. The result stays correct.

## 5. Cost per keystroke

### 5.1 Status-bar counts (PR #10)

Every keystroke used to count the words of the whole document again. `CountCache` in `crates/doc/src/counts.rs` keeps the counts of each top-level block by the block's address and counts only the blocks that changed. Numbered paragraphs are still counted every time, because a label depends on the paragraphs before it.

| | 284 pages | 993 pages | 2,837 pages |
|---|---|---|---|
| Before | 7.1 ms | 28.5 ms | 85 ms |
| After | 0.08 ms | 0.57 ms | 2.3 ms |

A document that is one giant table is not helped: one table is one block.

### 5.2 Relayout without changing the algorithm (PRs #10, #12)

Profiling showed work repeated for every paragraph on every pass. Four pieces were removed:

| Repeated work | Fix |
|---|---|
| Resolving two paragraphs' styles to test for a border group | The answer is remembered by the address of the shared properties |
| Object placement walking every cluster | Skipped when the paragraph has no objects |
| Resolving a paragraph's style up to three times | A memo per property address while the style sheet is the same (`crates/layout/src/memo.rs`) |
| Hashing cache keys with SipHash | A fast hasher for the layout's internal maps |

| Relayout per keystroke | 284 pages | 993 pages | 2,837 pages |
|---|---|---|---|
| Original | 13.9 ms | 56 ms | 161 ms |
| After PR #10 | 4.6 ms | 21 ms | 73 ms |
| After PR #12 | 3.0 ms | 11 ms | 55 ms |

Memoising by address is only possible because of section 4: equal properties are now really one object.

Tried and dropped: freeing the old layout on another thread. It measured slower, so it is not used.

## 6. Layout memory (PR #11)

Of the 359 MB a 993-page layout took, 256 MB was the *cluster* lists (one entry per character), and almost a third of that was spare room.

- The spare capacity of the cluster, glyph and line-position lists is released once a paragraph is laid out (`ParaLayout::release_spare_room`).
- `Cluster` is packed from 64 to 44 bytes: its byte range and kind are stored in 32 bits and read through `start()`, `end()` and `kind()`.

| Document | Before | After |
|---|---|---|
| 284 pages | 118 MB | 91 MB |
| 993 pages | 359 MB | 259 MB |
| 2,837 pages | 981 MB | 701 MB |

Speed is unchanged.

## 7. DOCX reader (PR #13)

The reader builds the XML tree of a whole part before turning it into a document. Two kinds of waste were removed in `crates/docx/src/xml.rs`:

- Child and attribute lists are trimmed as soon as their element has been read.
- `xml::Name`: one copy of each name, shared by every element and attribute of that name.

| Document (peak memory) | Before | After |
|---|---|---|
| 20,000 formatted paragraphs (15 MB of XML) | 182 MB | 117 MB |
| 900,000 runs in one paragraph | 891 MB | 379 MB |
| 400,000 table cells | 346 MB | 136 MB |
| 999,000 empty paragraphs | 240 MB | 201 MB |

The complete answer is to read as a stream with no tree, which means rewriting the reader. That has not been done.

## 8. Incremental pagination (PRs #14, #15, #16)

This is the largest design change. In the original, every keystroke lays the whole document out again from the first block.

### 8.1 The idea

An edit changes nothing before it, and often does not move the page breaks after it. So a pass need only:

1. **start** from the last safe point before the edit, and
2. **stop** once its state is exactly what the previous pass's was.

The absolute requirement: the result must be the same as laying out from scratch.

### 8.2 Stages

| Stage | PR | What it adds | Effect on speed |
|---|---|---|---|
| 0 | #14 | Test tooling: every layout compared with a from-scratch layout | None |
| 1 | #14 | Pagination state that stands alone and can be kept | None |
| 2 | #15 | Start from the mark before the edit | Edits near the end |
| 3 | #16 | Stop at a mark once the state is the same again | Edits anywhere |
| 4 | not done | Inserted and removed paragraphs can stop early too | — |

### 8.3 Parts

| Part | Where | Role |
|---|---|---|
| `verify_against_full`, `same_layout` | `crates/layout/src/verify.rs` | Compare a result with the from-scratch layout, page by page |
| `replay_blocks` | `crates/layout/src/verify.rs` | Places each block twice from a checkpoint; proves a checkpoint is complete |
| `Checkpoint` | `crates/layout/src/pass.rs` | Pagination state at a block boundary, with the page being filled |
| `Mark` | `crates/layout/src/resume.rs` | A checkpoint kept every 8 pages for the next pass |
| `Kept`, `Given` | `crates/layout/src/resume.rs` | The last pass's results, and every input besides the document's content |
| `Tail` | `crates/layout/src/resume.rs` | The rest of the last pass, taken when a pass stops early |
| `Items` | `crates/layout/src/placed.rs` | A page's item list, shared until changed, so kept pages are not copied |

### 8.4 When a mark may be started from

All of these must be unchanged since the previous pass:

- the blocks before the mark, and the 12 after it that pagination looks ahead at (spacing between paragraphs, joined borders, keep-with-next chains);
- every input besides the document's content: view options, styles, section properties, the other stories (footnotes, headers, footers), settings, footnote numbering.

Blocks are compared by address. That is sound because a block that is held in common is never changed in place, only replaced by a changed copy.

### 8.5 When a pass may stop

- The number of blocks and the section boundaries are the same.
- The blocks from two before the mark to the end of the document are the same blocks.
- The state on reaching the mark is exactly the previous pass's state at that mark: every value of the page builder, the counters, and the page being filled.

### 8.6 When a pass still walks everything

- The text of a footnote, header or footer was edited.
- The document has linked text boxes or pictures wrapped along their outline.
- A section whose columns are being balanced.
- The Web, Draft and Outline views.
- Documents that need repeated passes: `NUMPAGES` in the body, floats above their anchors, footnotes numbered per page.
- After 256 incremental passes in a row, one full pass runs so the paragraph cache can be cleaned.

### 8.7 Results (2,837 pages)

| Per keystroke | Before stage 2 | Stage 2 | Stage 3 |
|---|---|---|---|
| Typing at the start of the document | 62–65 ms | 64–67 ms | 14–19 ms |
| Typing at the end of the document | 64 ms | 18 ms | 15–19 ms |

The remaining 15 ms or so is no longer pagination but the per-page work after it: the line index (about 9 ms) and headers and footers (about 3 ms).

### 8.8 Known limits

- An edit that moves a page break still lays out until a page break is back where it was, which can mean to the end of the document.
- Inserting or removing a paragraph cannot stop early, because a page item records which block it belongs to (stage 4).
- States are compared through each value's `Debug` output; a picture's contour prints as a 64-bit digest.
- The "two blocks before the mark" margin is a safety margin; no test fails when it is removed.

## 9. How the changes were verified

| Method | Used for | What it shows |
|---|---|---|
| Old CLI output against new, byte for byte | Every optimisation | Rendering, page counts and conversions (DOCX, ODT, RTF, HTML, TXT, JSON) are unchanged; most recently 136 of 136 identical |
| Random-edit tests with the comparison on | Incremental pagination | After every edit the layout equals the from-scratch layout |
| Replay mode (`WORDCRAFT_LAYOUT_REPLAY=1`) | Stage 1 | A checkpoint holds all of the state |
| Resume mode (`WORDCRAFT_LAYOUT_RESUME=1`) | Stages 2 and 3 | Every layout test still holds when each layout is made again from a mark in its middle |
| Deliberately planted bugs | Stages 0, 2, 3 | The tests really catch a mistake |
| Mutated broken files (85 files) | DOCX reader | No crash and no hang |

Test failures present on the development machine and not caused by these changes: three layout hyphenation tests, one engine language-pack test and one UI glyph test. They need language data and fonts that machine does not have; they pass in CI.

## 10. Platform and interface changes

| PR | Problem | Change | Tested on a device |
|---|---|---|---|
| #24 | An update prompt every time the app starts | The update source is this repository's releases | Not confirmed |
| #25 | On a Snapdragon laptop a second document window is not shown | OpenGL is used when the only GPU is a Qualcomm Adreno | Confirmed by the owner |
| #26 | Open/Save dialogs bring the first window forward | A dialog is parented to the first window only when that window asked | Confirmed by the owner |
| #27 | No sign of success after exporting; the window stays on the Export page | Returns to the document and shows a notice | Not confirmed |
| #28 | The ribbon's lists have no live preview | Preview for font, size, colours and styles on the Home tab | No; the PR was open when this was written |
| #29 | About names only the original project | This repository's links, a disclaimer, contributor and model added | No; the PR was open when this was written |

Design notes:

- **#25 is a workaround.** The cause in the DirectX 12 path on the Adreno driver is not known. A machine without OpenGL keeps DirectX 12.
- **#26 has a cost.** A dialog asked for from a second window is not modal to that window. Parenting it to the right window needs a per-document window handle, which is not available without `unsafe` code (forbidden in this workspace).
- **Automatic updates on macOS will be refused**, because the update library requires the original project's Apple signature.

## 11. CI and dependencies

| Event | Action |
|---|---|
| CI red in every workflow | The cause was Actions billing on a private repository; the repository was made public |
| Two old clippy errors in the Linux clipboard module | Fixed (PR #8) |
| The `harfrust` 0.15 and `skrifa` 0.49 bump (PR #18) left `main` unable to compile | Reverted (PR #23); porting `crates/fonts` to the new API has not been done |
| The `vello_cpu` 0.3 bump (PR #19) fails to compile | Closed; needs code changes |
| Stable Rust moved to 1.100; clippy rejects five loops in test code | The loops were rewritten (part of PR #24) |

## 12. What still depends on the original project

- The shared libraries in `storytold/craft-libs` (windows, updates, printing, i18n) and the original project's fork of `egui`.
- The shared CI and release workflows in `storytold/craft-common-actions`.
- The translation catalogs and language-pack downloads from `storytold/craft-i18n`.
- The website and Discord links in the About window, and the app details in the Windows installer.

## 13. Departures from the repository's inherited rules

`AGENTS.md` holds working rules inherited from the original project. Three things done here do not follow them, recorded so that the choice is a conscious one:

| Rule in `AGENTS.md` | What was done here |
|---|---|
| `contributors/contributors.json` is never edited by hand; names are not taken from git configuration | PR #29 adds rows to that file by hand and uses the name from the commits, at the owner's request |
| A pull request does not say an agent wrote it | Commits and pull requests here name Claude as co-author |
| Workflows call the shared workflows at `@main`; CI is not changed in a feature PR | Workflows are pinned to a commit hash (PR #6) and an `audit` workflow was added |

Those rules were written for the original project. This repository is developed on its own and its owner has chosen not to follow them, so `AGENTS.md` should be changed to describe how work is really done here.

## 14. Releases and installing

- There is no official release from this repository yet. Test builds for Windows ARM64 are made by the "Windows ARM64" workflow and downloaded from its artifact, which expires after 7 days.
- The installer is not signed, so Windows SmartScreen warns. Signing has been put off.
- The release workflow uses signing certificates this repository does not have; whether it runs without them is not known.

## 15. Work identified for later

| Work | Benefit |
|---|---|
| Make the per-page work after pagination (line index, headers and footers) incremental | Cuts the remaining 15 ms or so per keystroke |
| Stage 4 of incremental pagination | Inserted and removed paragraphs stop early |
| Port `crates/fonts` to `harfrust` 0.15 / `skrifa` 0.49, and rendering to `vello_cpu` 0.3 | The dependency updates that were held back can land |
| A streaming DOCX reader | No XML tree in memory |
| The second window hidden on the first run after installing | A guess: the app was started by the installer; not proven |
| Run the fuzz targets | They exist and have never been run |
| Audit `gridcraft` | Not started |

## 16. Pull requests

| PR | Title | State |
|---|---|---|
| #6 | Audit fixes: DOCX memory limit, OLE without a preview, confined MCP, dependency audit | Merged |
| #7 | Shared formatting properties | Merged |
| #8 | Two clippy errors in the Linux clipboard module | Merged |
| #9 | `rustix` 1.1.6 so the audit workflow passes | Merged |
| #10 | Per-keystroke work: relayout and status-bar counts | Merged |
| #11 | Layout memory −28% | Merged |
| #12 | Relayout per keystroke: style memo and fast hasher | Merged |
| #13 | DOCX reader: peak memory −36% to −60% | Merged |
| #14 | Incremental pagination, stages 0–1 | Merged |
| #15 | Incremental pagination, stage 2 | Merged |
| #16 | Incremental pagination, stage 3 | Merged |
| #23 | Revert the font bump (#18) | Merged |
| #24 | Updates: look at this repository's releases | Merged |
| #25 | OpenGL on Windows machines with an Adreno GPU | Merged |
| #26 | File dialogs do not bring the first window forward | Merged |
| #27 | Export: back to the document, with a notice | Merged |
| #28 | Ribbon: live preview on the Home tab | Open when this was written |
| #29 | About: this repository, its owner and Claude Fable 5.2 | Open when this was written |

Pull requests #17 to #22 are dependency updates from Dependabot: #17, #20, #21 and #22 were merged; #18 was merged and then reverted; #19 was closed.
