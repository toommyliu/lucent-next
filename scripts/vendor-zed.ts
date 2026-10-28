import { fileURLToPath } from "node:url";

import * as NodeRuntime from "@effect/platform-node/NodeRuntime";
import * as NodeServices from "@effect/platform-node/NodeServices";
import * as Console from "effect/Console";
import * as Data from "effect/Data";
import * as Effect from "effect/Effect";
import * as FileSystem from "effect/FileSystem";
import * as Option from "effect/Option";
import * as Path from "effect/Path";
import * as Stream from "effect/Stream";
import { Argument, Command, Flag } from "effect/cli";
import { ChildProcess, ChildProcessSpawner } from "effect/process";

const ZED_REMOTE = "https://github.com/zed-industries/zed";
const VENDOR_DIR = fileURLToPath(new URL("../vendor/zed", import.meta.url));
const EXCLUDED_PATHS = ["crates/gpui/examples/image/*.gif"];
const README_REV = /`([0-9a-f]{40})`/;
const MEMBERS = /^members = \[([^\]]*)\]\n/m;
const DEFAULT_MEMBERS = /^default-members = \[[^\]]*\]\n/m;

class VendorZedError extends Data.TaggedError("VendorZedError")<{
  readonly message: string;
}> {}

const run = Effect.fn("run")(
  function* (cwd: string, command: string, args: ReadonlyArray<string>) {
    const spawner = yield* ChildProcessSpawner.ChildProcessSpawner;
    const handle = yield* spawner.spawn(
      ChildProcess.make(command, args, { cwd, stderr: "inherit" }),
    );
    const stdout = yield* handle.stdout.pipe(Stream.decodeText(), Stream.mkString);
    const exitCode = yield* handle.exitCode;
    if (exitCode !== 0) {
      return yield* new VendorZedError({
        message: `${command} ${args.join(" ")} exited with code ${exitCode}`,
      });
    }
    return stdout;
  },
  Effect.scoped,
);

const git = (cwd: string, args: ReadonlyArray<string>) => run(cwd, "git", args);

const readVendoredRev = Effect.gen(function* () {
  const fs = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;
  const readme = yield* fs.readFileString(path.join(VENDOR_DIR, "README.md"));
  const rev = README_REV.exec(readme)?.[1];
  if (rev === undefined) {
    return yield* new VendorZedError({ message: "vendor/zed/README.md names no Zed commit" });
  }
  return { readme, rev };
});

const parseMembers = (manifest: string, source: string) => {
  const list = MEMBERS.exec(manifest)?.[1];
  if (list === undefined) {
    return Effect.fail(new VendorZedError({ message: `${source} has no workspace members list` }));
  }
  return Effect.succeed((list.match(/"[^"]+"/g) ?? []).map((quoted) => quoted.slice(1, -1)));
};

const sync = Effect.fn("sync")(
  function* (checkout: string, base: Option.Option<string>) {
    const fs = yield* FileSystem.FileSystem;
    const path = yield* Path.Path;
    const vendored = yield* readVendoredRev;

    if ((yield* git(checkout, ["status", "--porcelain", "--untracked-files=no"])) !== "") {
      return yield* new VendorZedError({ message: `${checkout} has uncommitted changes` });
    }
    const baseRev = (yield* git(checkout, [
      "rev-parse",
      "--verify",
      `${Option.getOrElse(base, () => vendored.rev)}^{commit}`,
    ])).trim();
    yield* git(checkout, ["merge-base", "--is-ancestor", baseRev, "HEAD"]).pipe(
      Effect.mapError(
        () => new VendorZedError({ message: `${baseRev} is not an ancestor of HEAD in ${checkout}` }),
      ),
    );

    const members = yield* parseMembers(
      yield* fs.readFileString(path.join(VENDOR_DIR, "Cargo.toml")),
      "vendor/zed/Cargo.toml",
    );
    const staging = yield* fs.makeTempDirectoryScoped({
      directory: path.dirname(VENDOR_DIR),
      prefix: ".zed-",
    });
    const next = path.join(staging, "zed");
    const archive = path.join(staging, "zed.tar");
    yield* git(checkout, [
      "archive",
      `--output=${archive}`,
      "HEAD",
      "--",
      "Cargo.toml",
      "LICENSE-APACHE",
      ...members,
      ...EXCLUDED_PATHS.map((excluded) => `:(exclude)${excluded}`),
    ]);
    yield* fs.makeDirectory(next);
    yield* run(next, "tar", ["-xf", archive]);

    const symlinks = (yield* git(checkout, ["ls-tree", "-r", "-z", "HEAD", "--", ...members]))
      .split("\0")
      .flatMap((entry) => {
        const [meta, file] = entry.split("\t");
        return meta?.startsWith("120000 ") && file !== undefined ? [file] : [];
      });
    for (const link of symlinks) {
      yield* fs.remove(path.join(next, link));
      yield* fs.copyFile(path.join(checkout, link), path.join(next, link));
    }

    const manifestPath = path.join(next, "Cargo.toml");
    const manifest = yield* fs.readFileString(manifestPath);
    const kept = (yield* parseMembers(manifest, "Zed's Cargo.toml"))
      .filter((member) =>
        members.some((vendoredMember) => member === vendoredMember || member.startsWith(`${vendoredMember}/`)),
      )
      .sort();
    yield* fs.writeFileString(
      manifestPath,
      manifest
        .replace(MEMBERS, `members = [${kept.map((member) => `"${member}"`).join(", ")}]\n`)
        .replace(DEFAULT_MEMBERS, ""),
    );
    yield* fs.writeFileString(
      path.join(next, "lucent.patch"),
      yield* git(checkout, [
        "format-patch",
        "--stdout",
        "--zero-commit",
        "--full-index",
        "--no-signature",
        `${baseRev}..HEAD`,
      ]),
    );
    yield* fs.writeFileString(
      path.join(next, "README.md"),
      vendored.readme.replace(README_REV, `\`${baseRev}\``),
    );
    const patches = (yield* git(checkout, ["rev-list", "--count", `${baseRev}..HEAD`])).trim();

    yield* fs.rename(VENDOR_DIR, path.join(staging, "previous"));
    yield* fs.rename(next, VENDOR_DIR);
    yield* Console.log(`Vendored Zed ${baseRev} with ${patches} patches`);
  },
  Effect.scoped,
);

const update = Effect.fn("update")(function* (rev: string) {
  const fs = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;
  const spawner = yield* ChildProcessSpawner.ChildProcessSpawner;
  const vendored = yield* readVendoredRev;
  const checkout = yield* fs.makeTempDirectory({ prefix: "lucent-zed-" });
  const removeCheckout = fs.remove(checkout, { recursive: true }).pipe(Effect.ignore());

  const { newRev, exitCode } = yield* Effect.gen(function* () {
    yield* git(checkout, ["init", "--quiet"]);
    yield* git(checkout, ["fetch", "--quiet", "--depth=1", ZED_REMOTE, vendored.rev]);
    yield* git(checkout, ["checkout", "--quiet", "--detach", "FETCH_HEAD"]);
    yield* git(checkout, ["am", "--quiet", path.join(VENDOR_DIR, "lucent.patch")]);
    yield* git(checkout, ["fetch", "--quiet", "--depth=1", ZED_REMOTE, rev]);
    const newRev = (yield* git(checkout, ["rev-parse", "FETCH_HEAD"])).trim();
    const exitCode = yield* spawner.exitCode(
      ChildProcess.make("git", ["rebase", "--quiet", "--onto", newRev, vendored.rev], {
        cwd: checkout,
        stdout: "inherit",
        stderr: "inherit",
      }),
    );
    return { newRev, exitCode };
  }).pipe(Effect.onError(() => removeCheckout));

  if (exitCode !== 0) {
    return yield* new VendorZedError({
      message: [
        `The patches do not rebase cleanly onto ${newRev}.`,
        `Resolve the conflicts in ${checkout} and run git rebase --continue, then run:`,
        `pnpm vendor:zed sync ${checkout} --base ${newRev}`,
      ].join("\n"),
    });
  }
  yield* sync(checkout, Option.some(newRev)).pipe(Effect.ensuring(removeCheckout));
});

const updateCommand = Command.make(
  "update",
  {
    rev: Argument.String("rev").pipe(
      Argument.withDescription("Zed commit, branch, or tag to move to"),
    ),
  },
  ({ rev }) => update(rev),
).pipe(
  Command.withDescription("Rebase lucent.patch onto a newer Zed and vendor the result"),
);

const syncCommand = Command.make(
  "sync",
  {
    checkout: Argument.Directory("checkout", { mustExist: true }).pipe(
      Argument.withDescription("Zed checkout with the Lucent patches committed on top of the base"),
    ),
    base: Flag.String("base").pipe(
      Flag.withDescription("Zed commit the patches sit on (defaults to the vendored commit)"),
      Flag.optional,
    ),
  },
  ({ checkout, base }) => sync(checkout, base),
).pipe(
  Command.withDescription("Vendor the crates and patches from a Zed checkout"),
);

Command.make("vendor-zed").pipe(
  Command.withSubcommands([updateCommand, syncCommand]),
  Command.run({ version: "1.0.0" }),
  Effect.catchTag("VendorZedError", (error) =>
    Console.error(error.message).pipe(
      Effect.andThen(
        Effect.sync(() => {
          process.exitCode = 1;
        }),
      ),
    ),
  ),
  Effect.provide(NodeServices.layer),
  NodeRuntime.runMain,
);
