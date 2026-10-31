import { expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import { vi } from "vitest";

import * as Fiber from "effect/Fiber";
import { ask } from "./asks";
import { operate } from "./operations";

it("delivers answers before an event loss marker and fails only remaining requests", async () => {
  const timers = { setTimeout, setInterval, clearTimeout, clearInterval };
  vi.stubGlobal("__host", { now: () => 0 });
  const { turn } = await import("./loop");
  Object.assign(globalThis, timers);
  const host = { ask: vi.fn().mockReturnValueOnce("a").mockReturnValueOnce("b"), cancelAsk: vi.fn() };
  const a = Effect.runFork(ask({}, host));
  const b = Effect.runFork(ask({}, host));
  const first = Effect.runPromise(Fiber.join(a));
  const second = Effect.runPromise(Fiber.join(b)).catch((error: unknown) => error);
  const operation = Effect.runPromise(operate({ kind: "acceptDrop", selector: 1 }, { prepareOperation: () => "op", operate: vi.fn(), cancelOperation: vi.fn() }));
  const completed = operation.catch((error: unknown) => error);
  turn(JSON.stringify([
    { type: "answer", id: "a", answer: 7, error: null },
    { type: "operation", id: "op", answer: { kind: "acceptDrop", value: true }, error: null },
    { type: "eventslost" },
  ]));
  expect(await first).toBe(7);
  expect(await completed).toEqual({ kind: "acceptDrop", value: true });
  expect(String(await second)).toContain("Host events were lost");
  vi.unstubAllGlobals();
});
