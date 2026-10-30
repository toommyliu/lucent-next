import { afterEach, expect, it, vi } from "vitest";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import { Api, type ApiService } from "../game/flash/api/Api";
import { Gateway } from "../game/flash/bridge/Gateway";
import { botRelogin } from "./botRelogin";

vi.mock("./botReloginApi", () => ({
  makeBotReloginApi: (api: ApiService) => api,
  readReloginRecords: (host: typeof __host) => JSON.parse(host.read("{}")),
}));

const records = { config: { enabled: false, delayMs: 3000, server: null }, status: {
  phase: { tag: "armed" }, captured: false, attemptsRemaining: null, lastError: null,
} };
let runtime: ManagedRuntime.ManagedRuntime<never, never> | undefined;
afterEach(async () => { await runtime?.dispose(); vi.unstubAllGlobals(); });
const boot = async (getServers: ApiService["auth"]["getServers"]) => {
  const answers: unknown[][] = [];
  const live = new Set<string>();
  vi.stubGlobal("window", {});
  vi.stubGlobal("__host", {
    read: () => JSON.stringify(records), publishAutoRelogin: () => {},
    answer: (json: string) => answers.push(JSON.parse(json)),
    isAskPending: (id: string) => live.has(id),
  });
  const api = { auth: { getServers }, events: { on: () => Effect.succeed(() => {}) } } as unknown as ApiService;
  runtime = ManagedRuntime.make(botRelogin.pipe(Layer.provide(Layer.mergeAll(
    Layer.succeed(Api, api), Layer.succeed(Gateway, {} as never),
  ))));
  await runtime.runPromise(Effect.void);
  await vi.waitFor(() => expect(window.onBotAsk).toBeTypeOf("function"));
  const ask = (id: string, payload: unknown) => { live.add(id); window.onBotAsk!(id, payload); };
  return { answers, live, ask };
};

it("skips cancelled and expired asks before applying the surviving FIFO writes", async () => {
  const applied: number[] = [];
  let calls = 0;
  const { answers, live, ask } = await boot(() => Effect.gen(function* () {
    applied.push(++calls);
    if (calls === 1) yield* Effect.sleep(100);
    return [];
  }));
  ask("active", { kind: "autoRelogin.setServer", server: "one" });
  await vi.waitFor(() => expect(applied).toEqual([1]));
  ask("cancelled", { kind: "autoRelogin.setServer", server: "cancelled" });
  ask("expired", { kind: "autoRelogin.setServer", server: "expired" });
  ask("second", { kind: "autoRelogin.setServer", server: "two" });
  ask("third", { kind: "autoRelogin.setServer", server: "three" });
  live.delete("cancelled"); live.delete("expired");
  await vi.waitFor(() => expect(answers.map(a => a[0])).toEqual(["active", "second", "third"]));
  expect(applied).toEqual([1, 2, 3]);
  expect(answers.map(a => (a[1] as {server:string}).server)).toEqual(["one", "two", "three"]);
});

it("bounds asks already pulled while an apply is blocked", async () => {
  const { answers, ask } = await boot(() => Effect.never);
  ask("active", { kind: "autoRelogin.setServer", server: "one" });
  await new Promise(resolve => setTimeout(resolve, 20));
  for (let i = 0; i < 1100; i++) ask(String(i), { kind: "autoRelogin.setDelay", delayMs: i });
  await vi.waitFor(() => expect(answers.length).toBeGreaterThan(0));
  expect(answers.every(a => a[1] === null && a[2] === "The bot request queue is full")).toBe(true);
});

it("answers a failed apply once and frees the slot for the next ask", async () => {
  let applies = 0;
  const { answers, ask } = await boot(() => Effect.suspend(() => {
    applies++; return Effect.die(new Error("private boundary failure"));
  }));
  ask("bad", { kind: "autoRelogin.setServer", server: "one" });
  ask("good", { kind: "autoRelogin.setDelay", delayMs: 42 });
  await vi.waitFor(() => expect(answers.map(a => a[0])).toEqual(["bad", "good"]), { timeout: 200 });
  expect(answers[0]).toEqual(["bad", null, "The bot request failed"]);
  expect((answers[1]![1] as {delayMs:number}).delayMs).toBe(42);
  expect(applies).toBe(1);
});
