import type { AutoReloginRecords } from "@lucent/game/protocol";
import * as Cause from "effect/Cause";
import type * as Scope from "effect/Scope";
import * as Deferred from "effect/Deferred";
import * as Effect from "effect/Effect";
import * as FiberMap from "effect/FiberMap";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
import * as Queue from "effect/Queue";
import * as Stream from "effect/Stream";

import { makeAutoRelogin, type AutoRelogin } from "../game/automation/AutoRelogin";
import { makeRetryAttempt } from "../game/automation/retryLoop";
import { Api } from "../game/flash/api/Api";
import { publicReloginState } from "./autoRelogin";

const decodeRequest = Schema.decodeUnknownOption(Schema.Union([
  Schema.Struct({ kind: Schema.Literal("autoRelogin.setEnabled"), enabled: Schema.Boolean }),
  Schema.Struct({ kind: Schema.Literal("autoRelogin.setDelay"), delayMs: Schema.Number }),
  Schema.Struct({ kind: Schema.Literal("autoRelogin.setServer"), server: Schema.NullOr(Schema.String) }),
]));

const readReloginRecords = (host: Pick<typeof __host, "read">): AutoReloginRecords =>
  JSON.parse(host.read('{"kind":"autoRelogin"}')) as AutoReloginRecords;

export const botRelogin = Layer.effectDiscard(Effect.gen(function* () {
  const host = __host;
  const api = yield* Api;
  const fibers = yield* FiberMap.make<string>();
  const ready = yield* Deferred.make<AutoRelogin>();
  const queue = yield* Queue.dropping<{ id: string; payload: unknown }>(1024);
  const runFork = Effect.runForkWith(yield* Effect.context<Scope.Scope>());
  const retry = makeRetryAttempt("auto relogin");
  const supervise = <A, E, R>(effect: Effect.Effect<A, E, R>) => retry(() => effect, 500);
  const launch = <A, E>(effect: Effect.Effect<A, E, Scope.Scope>) => { runFork(supervise(effect).pipe(Effect.forkScoped)); };
  const publish = (records: AutoReloginRecords) => Effect.sync(() => {
    const json = JSON.stringify(records);
    host.publishAutoRelogin(json);
  });
  const answer = (id: string, value: unknown, error: "queueFull" | "invalidRequest" | "failed" | null) => Effect.sync(() => {
    const json = JSON.stringify([id, value, error]);
    host.answer(json);
  });

  window.onBotAsk = (id, payload) => {
    if (!Queue.offerUnsafe(queue, { id, payload })) {
      host.answer(JSON.stringify([id, null, "queueFull"]));
    }
  };
  yield* Effect.forever(Effect.gen(function* () {
    const { id, payload } = yield* Queue.take(queue);
    const relogin = yield* Deferred.await(ready);
    if (!host.isAskPending(id)) return;
    const request = decodeRequest(payload);
    if (Option.isNone(request)) { yield* answer(id, null, "invalidRequest"); return; }
    yield* Effect.gen(function* () {
      const input = request.value;
      switch (input.kind) {
        case "autoRelogin.setEnabled": yield* relogin.setEnabled(input.enabled); break;
        case "autoRelogin.setDelay": yield* relogin.setDelay(input.delayMs); break;
        case "autoRelogin.setServer": yield* relogin.setServer(input.server ?? undefined); break;
      }
      const records = yield* relogin.getRecords();
      yield* publish(records);
      yield* answer(id, publicReloginState(records), null);
    }).pipe(Effect.catchCause(cause => Cause.hasInterrupts(cause) ? Effect.failCause(cause) : answer(id, null, "failed")));
  })).pipe(Effect.forkScoped);
  yield* supervise(Effect.gen(function* () {
    const stored = yield* Effect.sync(() => readReloginRecords(host));
    const relogin = yield* makeAutoRelogin(api, fibers, stored.config, retry);
    yield* Deferred.succeed(ready, relogin);
    launch(Stream.runForEach(relogin.recordChanges, () => supervise(relogin.getRecords().pipe(Effect.flatMap(publish)))));
    if (stored.config.enabled) launch(Effect.gen(function* () {
      const latest = yield* Effect.sync(() => readReloginRecords(host));
      if (latest.config.enabled) yield* relogin.setEnabled(true);
    }));
  })).pipe(Effect.forkScoped);
}));
