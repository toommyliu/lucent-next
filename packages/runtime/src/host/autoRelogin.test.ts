import { expect, it, vi } from "vitest";
import * as Effect from "effect/Effect";
import { makeAutoReloginService } from "./autoRelogin";

it("normalizes non-finite delays before crossing the JSON boundary", async () => {
  const records = { config: { enabled: false, delayMs: 3000, server: null }, status: { phase: {tag:"armed"}, captured:false, username:null, lastError:null, attemptsRemaining:null } };
  const host = { command: vi.fn(() => JSON.stringify(records)), read: vi.fn() };
  expect(await Effect.runPromise(makeAutoReloginService(host).setDelay(Infinity))).toEqual({ enabled: false, delayMs: 3000, captured: false, attempting: false, waitingDelay: false });
  expect(host.command).toHaveBeenCalledWith('{"kind":"setAutoReloginDelay","delayMs":3000}');
});
