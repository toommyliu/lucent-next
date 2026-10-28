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
import { Argument, Command, Prompt } from "effect/cli";
import { ChildProcess, ChildProcessSpawner } from "effect/process";

const RUFFLE_REMOTE = "https://github.com/toommyliu/ruffle";
const ROOT = fileURLToPath(new URL("..", import.meta.url));
const PIN = /git = "https:\/\/github\.com\/toommyliu\/ruffle", rev = "([0-9a-f]{40})"/g;

class BumpRuffleError extends Data.TaggedError("BumpRuffleError")<{
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
      return yield* new BumpRuffleError({
        message: `${command} ${args.join(" ")} exited with code ${exitCode}`,
      });
    }
    return stdout;
  },
  Effect.scoped,
);

const git = (cwd: string, args: ReadonlyArray<string>) => run(cwd, "git", args);

const lines = (output: string) => output.split("\n").filter((line) => line !== "");

const commits = (output: string) =>
  lines(output).map((line) => {
    const [hash = "", subject = ""] = line.split("\t");
    return { hash, subject };
  });

const repin = (text: string, from: string, to: string) =>
  text.replaceAll(from, to).replaceAll(`#${from.slice(0, 8)}"`, `#${to.slice(0, 8)}"`);

const fetchRuffle = (repo: string, refspecs: ReadonlyArray<string>) =>
  git(repo, ["fetch", "--quiet", "--filter=tree:0", "--no-tags", RUFFLE_REMOTE, ...refspecs]);

const revParse = (repo: string, rev: string) =>
  git(repo, ["rev-parse", "--verify", "--quiet", `${rev}^{commit}`]).pipe(
    Effect.map((output) => Option.some(output.trim())),
    Effect.catchTag("BumpRuffleError", () => Effect.succeedNone),
  );

const resolve = Effect.fn("resolve")(function* (repo: string, rev: string) {
  for (const candidate of [`refs/remotes/ruffle/${rev}`, rev]) {
    const found = yield* revParse(repo, candidate);
    if (Option.isSome(found)) {
      return found.value;
    }
  }
  yield* fetchRuffle(repo, [rev]).pipe(
    Effect.mapError(
      () => new BumpRuffleError({ message: `${RUFFLE_REMOTE} has no commit, branch, or tag ${rev}` }),
    ),
  );
  return (yield* git(repo, ["rev-parse", "FETCH_HEAD^{commit}"])).trim();
});

const pick = Effect.fn("pick")(function* (repo: string, pin: string) {
  const branches = lines(
    yield* git(repo, [
      "for-each-ref",
      "--sort=-committerdate",
      "--format=%(refname:strip=3)%09%(objectname)%09%(committerdate:short)%09%(subject)",
      "refs/remotes/ruffle",
    ]),
  ).map((line) => {
    const [name = "", rev = "", date = "", subject = ""] = line.split("\t");
    return { name, rev, date, subject };
  });
  const branch = yield* Prompt.run(
    Prompt.Select({
      message: `Ruffle is pinned at ${pin.slice(0, 10)}. Pick a branch`,
      choices: branches.map((branch) => ({
        title: branch.name,
        value: branch,
        description: `${branch.rev.slice(0, 10)} ${branch.date} ${branch.subject}`,
      })),
    }),
  );

  const history = lines(
    yield* git(repo, ["log", "--first-parent", "--format=%H%x09%cs%x09%s", `${pin}..${branch.rev}`]),
  ).map((line) => {
    const [rev = "", date = "", subject = ""] = line.split("\t");
    return { rev, date, subject };
  });
  if (history.length === 0) {
    return yield* new BumpRuffleError({ message: `The pin already has every commit on ${branch.name}` });
  }
  return yield* Prompt.run(
    Prompt.AutoComplete({
      message: `Pick a commit on ${branch.name}`,
      choices: history.map((commit) => ({
        title: `${commit.rev.slice(0, 10)} ${commit.subject}`,
        value: commit.rev,
        description: commit.date,
      })),
    }),
  );
});

const bump = Effect.fn("bump")(function* (rev: Option.Option<string>) {
  const fs = yield* FileSystem.FileSystem;
  const path = yield* Path.Path;
  const spawner = yield* ChildProcessSpawner.ChildProcessSpawner;
  const manifestPath = path.join(ROOT, "Cargo.toml");
  const lockPath = path.join(ROOT, "Cargo.lock");

  if ((yield* git(ROOT, ["status", "--porcelain", "--", "Cargo.toml", "Cargo.lock"])) !== "") {
    return yield* new BumpRuffleError({ message: "Cargo.toml or Cargo.lock has uncommitted changes" });
  }
  const manifest = yield* fs.readFileString(manifestPath);
  const pins = new Set(Array.from(manifest.matchAll(PIN), (match) => match[1]));
  const [pin] = pins;
  if (pin === undefined || pins.size !== 1) {
    return yield* new BumpRuffleError({
      message: `Cargo.toml must pin every Ruffle crate at one revision, found ${pins.size}`,
    });
  }

  const repo = yield* fs.makeTempDirectoryScoped({ prefix: "lucent-ruffle-" });
  yield* git(repo, ["init", "--quiet"]);
  yield* Console.log(`Fetching ${RUFFLE_REMOTE}`);
  yield* fetchRuffle(repo, [pin, "+refs/heads/*:refs/remotes/ruffle/*"]);

  const target = yield* Option.match(rev, {
    onSome: (rev) => resolve(repo, rev),
    onNone: () => pick(repo, pin),
  });
  if (target === pin) {
    return yield* Console.log(`Ruffle is already pinned at ${pin}`);
  }

  const incoming = commits(
    yield* git(repo, ["log", "--no-merges", "--format=%h%x09%s", `${pin}..${target}`]),
  );
  const outgoing = commits(
    yield* git(repo, ["log", "--no-merges", "--format=%h%x09%s", `${target}..${pin}`]),
  );
  const outgoingSubjects = new Set(outgoing.map((commit) => commit.subject));
  const incomingSubjects = new Set(incoming.map((commit) => commit.subject));
  const added = incoming.filter((commit) => !outgoingSubjects.has(commit.subject));
  const dropped = outgoing.filter((commit) => !incomingSubjects.has(commit.subject));
  const rebased = incoming.length - added.length;

  yield* Console.log(`\n${target.slice(0, 10)} adds ${added.length} commits:`);
  for (const commit of added) {
    yield* Console.log(`  ${commit.hash} ${commit.subject}`);
  }
  if (rebased > 0) {
    yield* Console.log(`${rebased} commits on the pin were rewritten and keep their subjects.`);
  }
  if (dropped.length > 0) {
    yield* Console.log(`${dropped.length} commits on the pin are not in ${target.slice(0, 10)}:`);
    for (const commit of dropped) {
      yield* Console.log(`  ${commit.hash} ${commit.subject}`);
    }
  }

  const confirmed = yield* Prompt.run(
    Prompt.Confirm({ message: `Pin Ruffle at ${target.slice(0, 10)}?`, initial: true }),
  );
  if (!confirmed) {
    return;
  }

  const lock = yield* fs.readFileString(lockPath);
  if (!lock.includes(pin)) {
    return yield* new BumpRuffleError({ message: `Cargo.lock does not mention ${pin}` });
  }
  const repinnedLock = repin(lock, pin, target);
  const cargoMetadata = (args: ReadonlyArray<string>, stderr: "ignore" | "inherit") =>
    spawner.exitCode(
      ChildProcess.make("cargo", ["metadata", "--format-version=1", ...args], {
        cwd: ROOT,
        stdout: "ignore",
        stderr,
      }),
    );
  const locked = yield* Effect.gen(function* () {
    yield* fs.writeFileString(manifestPath, repin(manifest, pin, target));
    yield* fs.writeFileString(lockPath, repinnedLock);
    if ((yield* cargoMetadata(["--locked"], "ignore")) === 0) {
      return true;
    }
    // `cargo update -p ruffle_core` re-resolves unrelated Windows dependency edges, so only let
    // cargo touch the lockfile when the revision swap alone leaves it inconsistent.
    yield* Console.log("The new revision changes Ruffle's dependencies. Letting cargo update Cargo.lock.");
    if ((yield* cargoMetadata([], "inherit")) !== 0) {
      return yield* new BumpRuffleError({
        message: "cargo cannot resolve the new revision. Cargo.toml and Cargo.lock are unchanged",
      });
    }
    return false;
  }).pipe(
    Effect.onError(() =>
      Effect.all([fs.writeFileString(manifestPath, manifest), fs.writeFileString(lockPath, lock)]).pipe(
        Effect.ignore(),
      ),
    ),
  );
  if (locked) {
    return yield* Console.log(`Pinned Ruffle at ${target}. Cargo.lock changed only the revision.`);
  }

  const previous = path.join(repo, "Cargo.lock");
  yield* fs.writeFileString(previous, repinnedLock);
  yield* Console.log("Review these lockfile changes beyond the revision:");
  yield* spawner.exitCode(
    ChildProcess.make("git", ["diff", "--no-index", "--no-color", "-U0", previous, lockPath], {
      cwd: ROOT,
      stdout: "inherit",
      stderr: "inherit",
    }),
  );
}, Effect.scoped);

Command.make(
  "bump-ruffle",
  {
    rev: Argument.String("rev").pipe(
      Argument.withDescription("Ruffle commit, branch, or tag to pin. Prompts for one when omitted"),
      Argument.optional,
    ),
  },
  ({ rev }) => bump(rev),
).pipe(
  Command.withDescription("Move the Ruffle pin to another commit on the fork"),
  Command.run({ version: "1.0.0" }),
  Effect.catchTag("BumpRuffleError", (error) =>
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
