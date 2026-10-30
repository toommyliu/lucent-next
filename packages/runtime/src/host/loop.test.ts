import { expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as TestClock from "effect/testing/TestClock";
import { vi } from "vitest";

import type { Position } from "@lucent/game";
import { layer as autoZone } from "../game/automation/AutoZone";
import { Api, type ApiService } from "../game/flash/api/Api";
import type { Event, EventSelector } from "../game/flash/contract/Event";
import { HostState } from "../game/flash/state/HostState";
import { makeFakeHost, makeFakeHostState } from "../game/flash/testing/FakeHost";

it.effect("keeps the current host auto zone config when an older item precedes event loss", () =>
  Effect.gen(function* () {
    vi.stubGlobal("window", {});
    vi.stubGlobal("__host", { now: () => 0 });
    yield* Effect.addFinalizer(() => Effect.sync(() => vi.unstubAllGlobals()));
    const timers = { setTimeout, setInterval, clearTimeout, clearInterval };
    const { turn } = yield* Effect.tryPromise(async () => {
      try {
        return await import("./loop");
      } finally {
        Object.assign(globalThis, timers);
      }
    });

    const state = makeFakeHostState();
    const walks: Position[] = [];
    let handleEvent: ((event: Event) => Effect.Effect<void>) | undefined;
    const api = {
      events: {
        on: (
          _selector: EventSelector | undefined,
          handler: (event: Event) => Effect.Effect<void>,
        ) => {
          handleEvent = handler;
          return Effect.succeed(() => undefined);
        },
      },
      map: { getName: () => Effect.succeed("queeniona") },
      player: {
        auras: { get: () => Effect.succeed(null) },
        walkTo: ({ x, y }: Position) => Effect.sync(() => {
          walks.push({ x, y });
          return true;
        }),
      },
    } as unknown as ApiService;
    yield* Layer.build(autoZone).pipe(
      Effect.provideService(Api, api),
      Effect.provideService(HostState, makeFakeHost(state, window)),
    );

    state.autoZone = { enabled: true, map: "queeniona" };
    expect(turn(JSON.stringify([
      { type: "autozone", config: { enabled: true, map: "ledgermayne" } },
      { type: "eventslost" },
    ]))).toEqual([]);
    yield* TestClock.adjust("500 millis");
    yield* handleEvent!({ type: "zone", map: "queeniona", zone: "" });
    yield* TestClock.adjust("500 millis");
    expect(walks).toEqual([{ x: 490, y: 320 }]);
  }),
);

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
