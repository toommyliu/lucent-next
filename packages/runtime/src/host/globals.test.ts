import { afterEach, describe, expect, it, vi } from "vitest";

import {
  AbortController as RuntimeAbortController,
  AbortSignal as RuntimeAbortSignal,
  Event as RuntimeEvent,
  EventTarget as RuntimeEventTarget,
  TextEncoder as RuntimeTextEncoder,
} from "./globals";

afterEach(() => vi.unstubAllGlobals());

describe("runtime globals", () => {
  it("fires listeners and onabort in registration order like Node", () => {
    const orders = [];
    for (const Controller of [AbortController, RuntimeAbortController]) {
      const controller = new Controller();
      const calls: string[] = [];
      controller.signal.addEventListener("abort", () => calls.push("first"));
      controller.signal.onabort = () => calls.push("replaced");
      controller.signal.addEventListener("abort", () => calls.push("last"));
      controller.signal.onabort = () => calls.push("onabort");
      controller.abort("stopped");
      controller.abort("again");
      orders.push(calls);
      expect(controller.signal.reason).toBe("stopped");
    }
    expect(orders).toEqual([
      ["first", "onabort", "last"],
      ["first", "onabort", "last"],
    ]);
  });

  it("retains the onabort registration slot after clearing it like Node", () => {
    const orders = [];
    for (const Controller of [AbortController, RuntimeAbortController]) {
      const controller = new Controller();
      const calls: string[] = [];
      controller.signal.onabort = () => calls.push("removed");
      controller.signal.addEventListener("abort", () => calls.push("listener"));
      controller.signal.onabort = null;
      controller.signal.onabort = () => calls.push("onabort");
      controller.abort();
      orders.push(calls);
    }
    expect(orders).toEqual([
      ["onabort", "listener"],
      ["onabort", "listener"],
    ]);
  });

  it("retains default, explicit and static abort reasons", () => {
    const controller = new RuntimeAbortController();
    controller.signal.throwIfAborted();
    expect(controller.signal.aborted).toBe(false);
    controller.abort();
    expect(controller.signal.aborted).toBe(true);
    expect(controller.signal.reason).toMatchObject({
      name: "AbortError",
      message: "This operation was aborted",
    });
    expect(() => controller.signal.throwIfAborted()).toThrow(
      "This operation was aborted",
    );
    const signal = RuntimeAbortSignal.abort(null);
    expect(signal.aborted).toBe(true);
    expect(signal.reason).toBe(null);
    expect(() => signal.throwIfAborted()).toThrow();
    expect(RuntimeAbortSignal.abort().reason).toMatchObject({
      name: "AbortError",
      message: "This operation was aborted",
    });
  });

  it("honors once, listener removal and additions during dispatch", () => {
    const target = new RuntimeEventTarget();
    const calls: string[] = [];
    const removed = () => calls.push("removed");
    const added = () => calls.push("added");
    target.addEventListener("tick", () => {
      calls.push("first");
      target.removeEventListener("tick", removed);
      target.addEventListener("tick", added);
    });
    const once = () => calls.push("once");
    target.addEventListener("tick", once, { once: true });
    target.addEventListener("tick", once);
    target.addEventListener("tick", removed);
    target.dispatchEvent(new RuntimeEvent("tick"));
    target.dispatchEvent(new RuntimeEvent("tick"));
    expect(calls).toEqual(["first", "once", "first", "added"]);
  });

  it("formats console values and forwards each log level", async () => {
    const records: unknown[] = [];
    vi.stubGlobal("__host", {
      log: (level: string, text: string) => records.push([level, text]),
    });
    vi.resetModules();
    const { runtimeConsole } = await import("./globals");
    const error = new Error("broken");
    error.stack = "Error: broken\n    at farm.js:2:3";
    const cycle: { self?: unknown } = {};
    cycle.self = cycle;
    runtimeConsole.log("ready", { count: 2 }, error, cycle);
    runtimeConsole.info("info");
    runtimeConsole.warn("warn");
    runtimeConsole.error("error");
    runtimeConsole.debug("debug");
    expect(records).toEqual([
      [
        "info",
        'ready {"count":2} Error: broken\n    at farm.js:2:3 {"self":"[Circular]"}',
      ],
      ["info", "info"],
      ["warn", "warn"],
      ["error", "error"],
      ["debug", "debug"],
    ]);
  });

  it("encodes UTF-8 including lone surrogates", () => {
    expect([...new RuntimeTextEncoder().encode("aé😀\ud800")]).toEqual([
      97, 195, 169, 240, 159, 152, 128, 239, 191, 189,
    ]);
  });
});
