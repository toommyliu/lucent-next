import { expect, it } from "@effect/vitest";
import { vi } from "vitest";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Stream from "effect/Stream";
import * as TestClock from "effect/testing/TestClock";
import { Api, type ApiService } from "../game/flash/api/Api";
import { botRelogin } from "./botRelogin";

const fake = vi.hoisted(() => ({ make: vi.fn() }));
vi.mock("../game/automation/AutoRelogin", async (original) => ({
  ...await original<object>(), makeAutoRelogin: () => fake.make(),
}));

it.effect("a script disable wins over a retried boot enable", () => Effect.gen(function* () {
  let enabled = true;
  let enables = 0;
  const records = () => ({ config: { enabled, delayMs: 3000, server: null }, status: { phase: {tag:"armed"}, captured:false, username:null, lastError:null, attemptsRemaining:null } });
  const answers: unknown[] = [];
  vi.stubGlobal("window", {});
  vi.stubGlobal("__host", {
    read: () => JSON.stringify(records()),
    publishAutoRelogin: () => {},
    answer: (json: string) => answers.push(JSON.parse(json)),
    isAskPending: () => true,
  });
  fake.make.mockReturnValue(Effect.succeed({
    setEnabled: (value: boolean) => Effect.sync(() => {
      enabled = value;
      if (value && ++enables === 1) throw new Error("boot capture failed");
    }),
    getRecords: () => Effect.sync(records),
    recordChanges: Stream.never,
  }));
  yield* Layer.build(botRelogin.pipe(Layer.provide(Layer.succeed(Api, {} as ApiService))));
  yield* Effect.yieldNow;
  window.onBotAsk!("disable", {kind:"autoRelogin.setEnabled",enabled:false});
  yield* Effect.yieldNow;
  yield* TestClock.adjust("1 second");
  yield* Effect.yieldNow;
  expect(answers).toHaveLength(1);
  expect(enabled).toBe(false);
  vi.unstubAllGlobals();
}));
