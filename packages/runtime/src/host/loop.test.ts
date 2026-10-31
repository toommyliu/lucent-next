import { expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import { vi } from "vitest";

import { operate } from "./operations";

it("delivers operation answers before event loss and fails remaining requests", async () => {
  const timers = { setTimeout, setInterval, clearTimeout, clearInterval };
  vi.stubGlobal("__host", { now: () => 0 });
  const { turn } = await import("./loop");
  Object.assign(globalThis, timers);
  const operation = Effect.runPromise(operate({ kind: "acceptDrop", selector: 1 }, { prepareOperation: () => "op", operate: vi.fn(), cancelOperation: vi.fn() }));
  const completed = operation.catch((error: unknown) => error);
  const remaining = Effect.runPromise(operate({ kind: "acceptDrop", selector: 2 }, { prepareOperation: () => "lost", operate: vi.fn(), cancelOperation: vi.fn() })).catch((error: unknown) => error);
  turn(JSON.stringify([
    { type: "operation", id: "op", answer: { kind: "acceptDrop", value: true }, error: null },
    { type: "eventslost" },
  ]));
  expect(await completed).toEqual({ kind: "acceptDrop", value: true });
  expect(String(await remaining)).toContain("Host events were lost");
  vi.unstubAllGlobals();
});
