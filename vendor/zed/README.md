# Vendored GPUI

The crates under `crates/` and `tooling/` are copied from zed-industries/zed at `1a246efd7e1b83ab568ec5e3e6c1a43a42e1abba`, with `lucent.patch` applied. Only the crates GPUI needs are here, and the example GIF under `crates/gpui/examples/image` is removed.

To move to a newer Zed: clone it at the new revision, `git am lucent.patch`, fix conflicts, then copy the same crate directories back here and regenerate `lucent.patch` with `git format-patch`.
