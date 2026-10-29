import { afterEach, expect, it, vi } from "vitest";

const { initializedWithHost } = vi.hoisted(() => ({
  initializedWithHost: [] as string[],
}));

vi.mock("../game/scripting/ScriptRunner", async (importOriginal) => {
  const original = await importOriginal<
    typeof import("../game/scripting/ScriptRunner")
  >();
  const Layer = await import("effect/Layer");
  return {
    ...original,
    layer: Layer.suspend(() => {
      initializedWithHost.push(typeof __host);
      return original.layer;
    }),
  };
});

afterEach(() => vi.unstubAllGlobals());

it("builds the script runner during boot while host internals exist", async () => {
  vi.stubGlobal("window", { swf: {} });
  vi.stubGlobal("__host", {});
  await import("./scripts");
  expect(initializedWithHost).toEqual(["object"]);
});
