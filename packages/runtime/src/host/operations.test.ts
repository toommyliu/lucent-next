import { afterEach, expect, it, vi } from "vitest";
import * as Effect from "effect/Effect";

import { makeFakeApi, makeFakeHostState } from "../game/flash/testing/FakeHost";
import { operate, completeOperation, startOperation } from "./operations";

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
  startOperation("lost");
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
  startOperation("slow");
  await vi.advanceTimersByTimeAsync(15_000);
  expect(result).toBeUndefined();
  completeOperation("slow", { kind: "acceptDrop", value: true }, null);
  await finished;
  expect(result).toBe(true);
  expect(vi.getTimerCount()).toBe(0);
});

it("keeps queued accepts waiting beyond the execution deadline", async () => {
  vi.useFakeTimers();
  vi.stubGlobal("__host", { operate: () => "queued" });
  let result: boolean | undefined;
  const finished = Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const api = yield* makeFakeApi({ swf: {} } as Window, makeFakeHostState());
    result = yield* api.drops.accept(70);
  })));
  await vi.advanceTimersByTimeAsync(36_000);
  expect(result).toBeUndefined();
  completeOperation("queued", { kind: "acceptDrop", value: true }, null);
  await finished;
  expect(result).toBe(true);
});

it("fails queued and running operations when their event stream overflows", async () => {
  const host = { now: () => 0, operate: () => "queued-lost", ask: () => "ask-lost", cancelAsk: vi.fn() };
  vi.stubGlobal("__host", host);
  vi.stubGlobal("window", {});
  const { turn } = await import("./loop");
  const { ask } = await import("./asks");
  let askError: unknown;
  const asking = Effect.runPromise(ask({}, host)).catch(cause => { askError = cause; });
  let error: unknown;
  void operate({ kind: "acceptDrop", dropId: 1 } as never, host).catch(cause => { error = cause; });
  turn('[{"type":"eventslost"}]');
  await Promise.resolve();
  await asking;
  expect(String(askError)).toContain("Host events were lost; pending requests failed");
  expect(host.cancelAsk).toHaveBeenCalledWith("ask-lost");
  expect(error).toBeInstanceOf(Error);
  expect((error as Error).message).toBe("Host events were lost; pending requests failed");
});
