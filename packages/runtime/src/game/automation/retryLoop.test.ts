import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Fiber from "effect/Fiber";
import * as TestClock from "effect/testing/TestClock";
import { vi } from "vitest";

import { makeRetryAttempt, makeRetryLoop } from "./retryLoop";

const advance = (millis: number) =>
  TestClock.adjust(millis).pipe(Effect.andThen(Effect.yieldNow));

describe("retryLoop", () => {
  it.effect("returns the successful attempt and propagates interruption while retrying", () =>
    Effect.gen(function* () {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const info = vi.spyOn(console, "info").mockImplementation(() => {});
      yield* Effect.addFinalizer(() => Effect.sync(() => vi.restoreAllMocks()));
      const retry = makeRetryAttempt("refresh");
      let attempts = 0;
      const refresh = yield* retry(() => {
        attempts++;
        if (attempts <= 2) throw new Error("host unavailable");
        return Effect.succeed({ enabled: true, map: "ledgermayne" });
      }, 500).pipe(Effect.forkScoped);
      yield* advance(1000);
      expect(yield* Fiber.join(refresh)).toEqual({ enabled: true, map: "ledgermayne" });
      expect(attempts).toBe(3);
      expect(warn).toHaveBeenCalledTimes(1);
      expect(info.mock.calls).toEqual([["refresh recovered"]]);
      yield* advance(1000);
      expect(attempts).toBe(3);

      const pending = yield* retry(() => {
        attempts++;
        return Effect.fail("still unavailable");
      }, 500).pipe(Effect.forkScoped);
      yield* advance(500);
      expect(attempts).toBe(5);
      yield* Fiber.interrupt(pending);
      yield* advance(1000);
      expect(attempts).toBe(5);
      expect(warn).toHaveBeenCalledTimes(2);
      expect(info).toHaveBeenCalledTimes(1);
      expect(yield* retry(() => Effect.succeed(42), 500)).toBe(42);
      expect(info).toHaveBeenCalledTimes(1);
    }),
  );

  it.effect("keeps separate retry factories' failure streaks independent", () =>
    Effect.gen(function* () {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const info = vi.spyOn(console, "info").mockImplementation(() => {});
      yield* Effect.addFinalizer(() => Effect.sync(() => vi.restoreAllMocks()));
      let firstFails = true;
      let secondFails = false;
      let firstAttempts = 0;
      let secondAttempts = 0;
      yield* makeRetryLoop("first")(() => {
        firstAttempts++;
        return firstFails
          ? Effect.fail(new Error("first unavailable"))
          : Effect.void;
      }, 500).pipe(Effect.forkScoped);
      yield* makeRetryLoop("second")(() => {
        secondAttempts++;
        return secondFails
          ? Effect.fail(new Error("second unavailable"))
          : Effect.void;
      }, 500).pipe(Effect.forkScoped);

      yield* advance(1000);
      expect([firstAttempts, secondAttempts]).toEqual([3, 3]);
      expect(warn.mock.calls.map(([message]) => message)).toEqual([
        "first tick failed",
      ]);
      expect(info.mock.calls).toEqual([]);

      secondFails = true;
      yield* advance(500);
      expect(warn.mock.calls.map(([message]) => message)).toEqual([
        "first tick failed",
        "second tick failed",
      ]);
      firstFails = false;
      yield* advance(500);
      expect(info.mock.calls).toEqual([["first recovered"]]);

      firstFails = true;
      yield* advance(500);
      expect(warn.mock.calls.map(([message]) => message)).toEqual([
        "first tick failed",
        "second tick failed",
        "first tick failed",
      ]);
      secondFails = false;
      yield* advance(500);
      expect(info.mock.calls).toEqual([
        ["first recovered"],
        ["second recovered"],
      ]);
      firstFails = false;
      yield* advance(500);
      expect(info.mock.calls).toEqual([
        ["first recovered"],
        ["second recovered"],
        ["first recovered"],
      ]);
      expect([firstAttempts, secondAttempts]).toEqual([8, 8]);
      expect(warn).toHaveBeenCalledTimes(3);
    }),
  );

  it.effect("retries ticks and logs once until every failing loop recovers", () =>
    Effect.gen(function* () {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const info = vi.spyOn(console, "info").mockImplementation(() => {});
      yield* Effect.addFinalizer(() => Effect.sync(() => vi.restoreAllMocks()));
      const repeat = makeRetryLoop("automation");
      let slowFails = true;
      let fastFails = true;
      let slowAttempts = 0;
      let fastAttempts = 0;
      const slow = yield* repeat(() => {
        slowAttempts++;
        if (slowFails) throw new Error("slow tick unavailable", { cause: "probe offline" });
        return Effect.void;
      }, 1000).pipe(Effect.forkScoped);
      const fast = yield* repeat(
        () => {
          fastAttempts++;
          return fastFails
            ? Effect.fail(new Error("fast tick unavailable"))
            : Effect.void;
        },
        500,
      ).pipe(Effect.forkScoped);
      yield* advance(2000);
      expect([slowAttempts, fastAttempts]).toEqual([3, 5]);
      expect(warn).toHaveBeenCalledTimes(1);
      expect(warn.mock.calls[0]?.[0]).toBe("automation tick failed");
      expect(warn.mock.calls[0]?.[1]).toContain("Error: slow tick unavailable");
      expect(warn.mock.calls[0]?.[1]).toContain("[cause]: Error: probe offline");
      expect(info).not.toHaveBeenCalled();

      fastFails = false;
      yield* advance(1000);
      expect([slowAttempts, fastAttempts]).toEqual([4, 7]);
      expect(warn).toHaveBeenCalledTimes(1);
      expect(info).not.toHaveBeenCalled();
      slowFails = false;
      yield* advance(1000);
      expect([slowAttempts, fastAttempts]).toEqual([5, 9]);
      expect(info.mock.calls).toEqual([["automation recovered"]]);

      fastFails = true;
      yield* advance(1000);
      expect(warn).toHaveBeenCalledTimes(2);
      expect(warn.mock.calls[1]?.[1]).toContain("Error: fast tick unavailable");
      fastFails = false;
      yield* advance(500);
      expect(info.mock.calls).toEqual([
        ["automation recovered"],
        ["automation recovered"],
      ]);
      yield* Fiber.interrupt(slow);
      yield* Fiber.interrupt(fast);
      const attempts = [slowAttempts, fastAttempts];
      yield* advance(2000);
      expect([slowAttempts, fastAttempts]).toEqual(attempts);
      expect(warn).toHaveBeenCalledTimes(2);
      expect(info).toHaveBeenCalledTimes(2);
    }),
  );

  it.effect("propagates interruption from a tick without logging or retrying", () =>
    Effect.gen(function* () {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const info = vi.spyOn(console, "info").mockImplementation(() => {});
      yield* Effect.addFinalizer(() => Effect.sync(() => vi.restoreAllMocks()));
      let attempts = 0;
      const loop = yield* makeRetryLoop("automation")(() => {
        attempts++;
        return Effect.interrupt;
      }, 500).pipe(Effect.forkScoped);
      yield* Fiber.await(loop);
      yield* advance(2000);
      expect(attempts).toBe(1);
      expect(warn).not.toHaveBeenCalled();
      expect(info).not.toHaveBeenCalled();
    }),
  );
});
