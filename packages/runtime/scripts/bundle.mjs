import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..");
const outfile = join(root, "assets", "runtime.js");
const check = process.argv.includes("--check");
const result = await build({
  absWorkingDir: root,
  entryPoints: [join(here, "..", "src", "host", "index.ts")],
  outfile,
  bundle: true,
  format: "iife",
  platform: "neutral",
  target: "es2022",
  mainFields: ["module", "main"],
  conditions: ["import", "default"],
  alias: {
    "@lucent/game/protocol": join(root, "packages", "game", "src", "protocol.ts"),
    "@lucent/game": join(root, "packages", "game", "src", "index.ts"),
  },
  legalComments: "none",
  keepNames: true,
  write: false,
  logLevel: "warning",
});
const generated = result.outputFiles[0].contents;
if (check) {
  const current = await readFile(outfile).catch(() => null);
  if (current === null || !current.equals(generated)) {
    console.error("Runtime bundle differs; run pnpm bundle:runtime");
    process.exitCode = 1;
  }
} else {
  await writeFile(outfile, generated);
}
