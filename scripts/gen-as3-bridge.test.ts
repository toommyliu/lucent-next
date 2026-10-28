import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, test } from "node:test";
import { fileURLToPath } from "node:url";

import * as NodeServices from "@effect/platform-node/NodeServices";
import * as Effect from "effect/Effect";

import { generateBridgeArtifacts as generate } from "./gen-as3-bridge";

const generateBridgeArtifacts = (options: Parameters<typeof generate>[0]) =>
  Effect.runPromise(generate(options).pipe(Effect.provide(NodeServices.layer)));

const repoRoot = fileURLToPath(new URL("../", import.meta.url));
const roots: string[] = [];
const source = `package lucent.game
{
  [BridgeNamespace("inventory")]
  public class Inventory
  {
    [BridgeExport("find")]
    [BridgeTsParamType("id: FlashTypes.ItemSelector")]
    [BridgeTsReturnType("FlashTypes.Item | null")]
    public static function getItem(id:Object, equipped:Boolean = false):Object {}
    [BridgeIgnore]
    public static function helper():void {}
    [BridgeExport]
    public static function list(...ids):Array {}
    [BridgeExport]
    public static function details():Object {}
    [BridgeEvent("onChanged")]
    public function changed(items:Array):void {}
  }
}
`;

async function fixture(contents = source) {
  const root = await fs.mkdtemp(join(tmpdir(), "lucent-bridge-"));
  roots.push(root);
  const file = join(root, "as3/src/lucent/game/Inventory.as");
  await fs.mkdir(dirname(file), { recursive: true });
  await fs.writeFile(file, contents);
  return {
    repoRoot: root,
    swfDtsPath: join(root, "types/swf.d.ts"),
    as3OutputPath: join(root, "as3/src/lucent/generated/BridgeRegistryGenerated.as"),
  };
}

function cli(args: string[], cwd = repoRoot) {
  return spawnSync(process.execPath, [
    "--import", import.meta.resolve("tsx"), join(repoRoot, "scripts/gen-as3-bridge.ts"), ...args,
  ], { cwd, encoding: "utf8", timeout: 15_000 });
}

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => fs.rm(root, { recursive: true, force: true })));
});

test("generates registrations and declarations from exports, events, and type overrides", async () => {
  const options = await fixture();
  await generateBridgeArtifacts(options);
  assert.equal(await fs.readFile(options.as3OutputPath, "utf8"), `// AUTO-GENERATED FILE. DO NOT EDIT.
package lucent.generated
{
  import lucent.Externalizer;
  import lucent.game.Inventory;

  public class BridgeRegistryGenerated
  {
    public static function register(external:Externalizer):void
    {
      external.externalize("inventory.details", Inventory.details);
      external.externalize("inventory.find", Inventory.getItem);
      external.externalize("inventory.list", Inventory.list);
    }
  }
}
`);
  assert.equal(await fs.readFile(options.swfDtsPath, "utf8"), `// AUTO-GENERATED FILE. DO NOT EDIT.

import type * as FlashTypes from "./Types";

declare global {
  interface Window {
    swf: {
      "inventory.details": () => Record<string, unknown> | null;
      "inventory.find": (
        id: FlashTypes.ItemSelector,
        equipped?: boolean,
      ) => FlashTypes.Item | null;
      "inventory.list": (...ids: unknown[]) => unknown[];
    };
    onChanged?: (items: unknown[]) => void;
  }
}
`);
});

test("the checked-in registry matches the loader's AS3 sources", async () => {
  const result = await generateBridgeArtifacts({ repoRoot, check: true });
  assert.equal(result.as3Registry, await fs.readFile(result.as3OutputPath, "utf8"));
});

test("declarations without FlashTypes remain a module and quote event names when required", async () => {
  const options = await fixture(source
    .replace('    [BridgeTsParamType("id: FlashTypes.ItemSelector")]\n', "")
    .replace('    [BridgeTsReturnType("FlashTypes.Item | null")]\n', "")
    .replace('BridgeEvent("onChanged")', 'BridgeEvent("inventory.changed")'));
  const result = await generateBridgeArtifacts(options);
  assert.match(result.swfDts, /export \{\};/);
  assert.doesNotMatch(result.swfDts, /import type/);
  assert.match(result.swfDts, /"inventory.changed"\?: \(items: unknown\[\]\) => void;/);
});

test("check detects missing and stale files without writing; unchanged generation preserves files", async () => {
  const options = await fixture();
  await assert.rejects(generateBridgeArtifacts({ ...options, check: true }), /Unable to read generated file/);
  assert.equal(await fs.access(options.as3OutputPath).then(() => true, () => false), false);
  await generateBridgeArtifacts(options);
  await fs.utimes(options.as3OutputPath, new Date(0), new Date(0));
  await generateBridgeArtifacts(options);
  assert.equal((await fs.stat(options.as3OutputPath)).mtimeMs, 0);
  await generateBridgeArtifacts({ ...options, check: true });
  await fs.writeFile(options.swfDtsPath, "stale\n");
  await assert.rejects(generateBridgeArtifacts({ ...options, check: true }), /Generated file is out of date: types[/\\]swf.d.ts/);
  assert.equal(await fs.readFile(options.swfDtsPath, "utf8"), "stale\n");
});

for (const [name, contents, expected] of [
  ["missing export annotation", source.replace('[BridgeExport("find")]', ""), /Missing BridgeExport\/BridgeIgnore/],
  ["conflicting annotations", source.replace('[BridgeExport("find")]', "[BridgeExport]\n    [BridgeIgnore]"), /both BridgeExport and BridgeIgnore/],
  ["duplicate exports", source.replace("[BridgeExport]\n    public static function list", '[BridgeExport("find")]\n    public static function list'), /Duplicate bridge export name "inventory.find"/],
  ["duplicate events", source.replace("[BridgeIgnore]", '[BridgeEvent("onChanged")]\n    [BridgeIgnore]'), /Duplicate bridge event name "onChanged"/],
  ["unknown parameter override", source.replace("id: FlashTypes.ItemSelector", "missing: string"), /references unknown parameter "missing"/],
  ["duplicate parameter override", source.replace('[BridgeTsParamType("id: FlashTypes.ItemSelector")]', '[BridgeTsParamType("id: string")]\n    [BridgeTsParamType("id: number")]'), /Duplicate BridgeTsParamType/],
  ["empty return override", source.replace("FlashTypes.Item | null", " "), /BridgeTsReturnType metadata requires a non-empty value/],
  ["inline import override", source.replace("FlashTypes.Item | null", "import('./Types').Item"), /should not use import\(\) expressions/],
] as const) {
  test(`rejects ${name} before writing artifacts`, async () => {
    const options = await fixture(contents);
    await assert.rejects(generateBridgeArtifacts(options), expected);
    assert.equal(await fs.access(options.as3OutputPath).then(() => true, () => false), false);
  });
}

test("CLI help describes check mode without running generation", async () => {
  const options = await fixture();
  const result = cli(["--repo-root", options.repoRoot, "--help"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /--check/);
  assert.doesNotMatch(result.stdout, /Generated as3/);
  assert.equal(await fs.access(options.as3OutputPath).then(() => true, () => false), false);
});

test("CLI rejects unknown options without generating artifacts", async () => {
  const options = await fixture();
  const result = cli(["--repo-root", options.repoRoot, "--unknown-option"]);
  assert.equal(result.status, 1, result.stdout);
  assert.match(result.stdout + result.stderr, /unknown-option/);
  assert.equal(await fs.access(options.as3OutputPath).then(() => true, () => false), false);
});

test("CLI generates and checks artifacts from another working directory", async () => {
  const options = await fixture();
  const args = ["--repo-root", options.repoRoot, "--swf-dts", options.swfDtsPath];
  const generated = cli(args, tmpdir());
  assert.equal(generated.status, 0, generated.stderr);
  assert.match(generated.stdout, /Exports: 3\nEvents: 1/);
  assert.match(await fs.readFile(options.swfDtsPath, "utf8"), /onChanged\?: \(items: unknown\[\]\) => void/);
  const checked = cli([...args, "--check"], tmpdir());
  assert.equal(checked.status, 0, checked.stderr);
  assert.match(checked.stdout, /Bridge artifacts are up to date/);
  await fs.writeFile(options.as3OutputPath, "stale\n");
  const stale = cli([...args, "--check"], tmpdir());
  assert.equal(stale.status, 1);
  assert.match(stale.stderr, /Generated file is out of date: as3[/\\]src[/\\]lucent[/\\]generated/);
  assert.equal(await fs.readFile(options.as3OutputPath, "utf8"), "stale\n");
});

test("CLI reports missing flag values before generating artifacts", async () => {
  const options = await fixture();
  const result = cli(["--repo-root", options.repoRoot, "--swf-dts"]);
  assert.equal(result.status, 1);
  assert.match(result.stdout + result.stderr, /swf-dts/);
  assert.equal(await fs.access(options.as3OutputPath).then(() => true, () => false), false);
});
