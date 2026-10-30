import { expect, it, vi } from "vitest";
import * as Cause from "effect/Cause";
import * as Effect from "effect/Effect";
import * as Exit from "effect/Exit";

import { retryReloginTick } from "./AutoReloginFailureRule";

it("propagates interruption without retrying or reporting a failure streak", async () => {
  const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
  try {
    for (const cause of [Cause.interrupt(), Cause.combine(Cause.interrupt(), Cause.die(new Error("cleanup failure")))]) {
      let attempts = 0;
      const exit = await Effect.runPromiseExit(retryReloginTick(Effect.suspend(() => {
        attempts++;
        return Effect.failCause(cause);
      })));
      expect(Exit.isFailure(exit)).toBe(true);
      expect(attempts).toBe(1);
      expect(warn).not.toHaveBeenCalled();
    }
  } finally {
    warn.mockRestore();
  }
});
