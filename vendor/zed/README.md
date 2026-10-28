# Vendored GPUI

The crates under `crates/` and `tooling/` are copied from zed-industries/zed at `1a246efd7e1b83ab568ec5e3e6c1a43a42e1abba`, with `lucent.patch` applied. Only the crates GPUI needs are here. The example GIF under `crates/gpui/examples/image` is left out, and license symlinks are copied as plain files.

To move to a newer Zed, run `pnpm vendor:zed update <rev>`. The script applies `lucent.patch` to the vendored commit, rebases it onto `<rev>`, and rebuilds this directory. If the rebase stops on a conflict, it leaves the checkout in place and prints the `pnpm vendor:zed sync` command to run after you finish the rebase.

To change the patches, commit on top of the vendored commit in a Zed checkout and run `pnpm vendor:zed sync <checkout>`.
