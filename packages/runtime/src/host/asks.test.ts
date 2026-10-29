import { expect, it, vi } from "vitest";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import { ask, completeAsk, serializePayload } from "./asks";

it("rejects functions, methods and nested functions before submission", async () => {
  const host = { ask: vi.fn(() => "1"), cancelAsk: vi.fn() };
  for (const payload of [() => {}, { method() {} }, { child: { action: () => {} } }, new Date(), { get value() { return 7; } }]) {
    expect(() => serializePayload(payload)).toThrow(TypeError);
    await expect(Effect.runPromise(ask(payload, host))).rejects.toThrow();
  }
  expect(host.ask).not.toHaveBeenCalled();
});

it("answers only the matching ask and cancels when its fiber stops waiting", async () => {
  const host = { ask: vi.fn().mockReturnValueOnce("a").mockReturnValueOnce("b"), cancelAsk: vi.fn() };
  const first = Effect.runFork(ask({ value: 1 }, host));
  const second = Effect.runFork(ask({ value: 2 }, host));
  await vi.waitFor(() => expect(host.ask).toHaveBeenCalledTimes(2));
  completeAsk("b", { value: 3 }, null);
  expect(await Effect.runPromise(Fiber.join(second))).toEqual({ value: 3 });
  await Effect.runPromise(Fiber.interrupt(first));
  expect(host.cancelAsk).toHaveBeenCalledWith("a");
  completeAsk("a", 99, null);
});
