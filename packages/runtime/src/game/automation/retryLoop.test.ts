import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as TestClock from "effect/testing/TestClock";
import { vi } from "vitest";

import { makeRetryLoop } from "./retryLoop";

const advance = (millis: number) =>
  TestClock.adjust(millis).pipe(Effect.andThen(Effect.yieldNow));

describe("retryLoop", () => {
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
