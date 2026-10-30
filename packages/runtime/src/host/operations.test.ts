import { afterEach, expect, it, vi } from "vitest";
import * as Effect from "effect/Effect";

import { makeFakeApi, makeFakeHostState } from "../game/flash/testing/FakeHost";
import { completeOperation } from "./operations";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

it("answers false when the host loses an accept answer", async () => {
  vi.useFakeTimers();
  const submit = vi.fn(() => "lost");
  vi.stubGlobal("__host", { operate: submit });
  let result: boolean | undefined;
  const finished = Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const api = yield* makeFakeApi({ swf: {} } as Window, makeFakeHostState());
    result = yield* api.drops.accept(70);
  })));
  await vi.waitFor(() => expect(submit).toHaveBeenCalledOnce());
  await vi.advanceTimersByTimeAsync(15_000);
  expect(result).toBeUndefined();
  await vi.advanceTimersByTimeAsync(20_000);
  expect(result).toBe(false);
  await finished;
  completeOperation("lost", { kind: "acceptDrop", value: true }, null);
  expect(result).toBe(false);
});

it("keeps a slow real accept alive and clears its deadline after the answer", async () => {
  vi.useFakeTimers();
  const submit = vi.fn(() => "slow");
  vi.stubGlobal("__host", { operate: submit });
  let result: boolean | undefined;
  const finished = Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const api = yield* makeFakeApi({ swf: {} } as Window, makeFakeHostState());
    result = yield* api.drops.accept(70);
  })));
  await vi.waitFor(() => expect(submit).toHaveBeenCalledOnce());
  await vi.advanceTimersByTimeAsync(15_000);
  expect(result).toBeUndefined();
  completeOperation("slow", { kind: "acceptDrop", value: true }, null);
  await finished;
  expect(result).toBe(true);
  expect(vi.getTimerCount()).toBe(0);
});
