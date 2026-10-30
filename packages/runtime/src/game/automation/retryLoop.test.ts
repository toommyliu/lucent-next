import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as TestClock from "effect/testing/TestClock";
import { vi } from "vitest";

import { makeRetryAttempt } from "./retryLoop";

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

});
