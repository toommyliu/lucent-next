import { expect, it, vi } from "vitest";
import * as Effect from "effect/Effect";
import { completeAsk } from "./asks";
import { makeAutoReloginService } from "./autoRelogin";

it("normalizes non-finite delays before crossing the JSON boundary", async () => {
  const host = { ask: vi.fn(() => "delay"), cancelAsk: vi.fn(), read: vi.fn() };
  const finished = Effect.runPromise(makeAutoReloginService(host).setDelay(Infinity));
  await vi.waitFor(() => expect(host.ask).toHaveBeenCalledWith('{"kind":"autoRelogin.setDelay","delayMs":3000}'));
  completeAsk("delay", { enabled: false, delayMs: 3000, captured: false, attempting: false, waitingDelay: false }, null);
  expect((await finished).delayMs).toBe(3000);
});
