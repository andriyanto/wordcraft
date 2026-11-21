# egui crates kept in this repository

`epaint` and `egui-wgpu` are copies of those crates from
[`storytold/egui`](https://github.com/storytold/egui) at commit
`6d518fc799681571963842f95b55a6b68431b15a`, the commit the rest of egui is still taken from
(`[patch.crates-io]` in the workspace `Cargo.toml`). They are licensed `MIT OR Apache-2.0`, like
egui (`LICENSE-MIT`, `LICENSE-APACHE`).

They are kept here so the interface's text can be drawn the way Windows' ClearType draws text
(subpixel edges). Everything else is as it was at that commit, except:

- the manifests name their dependencies' versions instead of egui's workspace (the same
  versions), have no `[lints]` from it, and leave out `epaint`'s benchmarks;
- the changes marked `WordKu:` in the source, listed below.

## Changes

(none yet)

To take a newer egui commit, copy the two crates from it again and apply the marked changes.
