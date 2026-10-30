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
  const operation = operate({ kind: "acceptDrop", itemId: 1 } as never, { operate: () => "op" });
  const completed = operation.catch((error: unknown) => error);
  turn(JSON.stringify([
    { type: "answer", id: "a", answer: 7, error: null },
    { type: "operation", id: "op", answer: { kind: "done" }, error: null },
    { type: "eventslost" },
  ]));
  expect(await first).toBe(7);
  expect(await completed).toEqual({ kind: "done" });
  expect(String(await second)).toContain("Host events were lost");
  vi.unstubAllGlobals();
});
