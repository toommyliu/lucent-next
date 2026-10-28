import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const generator = fileURLToPath(
  new URL("../packages/scripting-api/gen/gen-script-types.ts", import.meta.url),
);
const contract = new URL(
  "../packages/scripting-api/script-api.d.ts",
  import.meta.url,
);

const check = (path: string) =>
  spawnSync(process.execPath, [generator, "--check", "--out", path], {
    encoding: "utf8",
  });

test("scripting API CLI checks bytes without writing", async (context) => {
  const dir = await fs.mkdtemp(join(tmpdir(), "lucent-script-types-"));
  context.after(() => fs.rm(dir, { recursive: true, force: true }));
  const path = join(dir, "script-api.d.ts");
  const published = (await fs.readFile(contract, "utf8")).replace(/\r\n/g, "\n");

  for (const contents of [published, published.replace(/\n/g, "\r\n")]) {
    await fs.writeFile(path, contents);
    await fs.utimes(path, new Date(0), new Date(0));
    const before = await fs.stat(path);
    const result = check(path);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(await fs.readFile(path, "utf8"), contents);
    assert.equal((await fs.stat(path)).mtimeMs, before.mtimeMs);
  }

  const stale = published.replace("interface ScriptApi", "interface ChangedApi");
  assert.notEqual(stale, published);
  const line = published
    .slice(0, published.indexOf("interface ScriptApi"))
    .split("\n").length;
  await fs.writeFile(path, stale);
  await fs.utimes(path, new Date(0), new Date(0));
  const changed = check(path);
  assert.equal(changed.status, 1, changed.stdout);
  assert.match(changed.stderr, new RegExp(`differ.*line ${line}\\b`, "i"));
  assert.equal(await fs.readFile(path, "utf8"), stale);
  assert.equal((await fs.stat(path)).mtimeMs, 0);

  await fs.rm(path);
  const missing = check(path);
  assert.equal(missing.status, 1, missing.stdout);
  assert.match(missing.stderr, /Unable to read.*script-api\.d\.ts.*ENOENT/);
  await assert.rejects(fs.access(path), { code: "ENOENT" });
});
