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

const Records = Schema.Struct({
  config: Schema.Struct({ enabled: Schema.Boolean, delayMs: Schema.Number, server: Schema.NullOr(Schema.String) }),
  status: Schema.Struct({
    username: Schema.NullOr(Schema.String), captured: Schema.Boolean, attemptsRemaining: Schema.NullOr(Schema.Number), lastError: Schema.NullOr(Schema.String),
    phase: Schema.Union([
      Schema.Struct({ tag: Schema.Literals(["armed", "awaiting-session", "awaiting-player-ready", "awaiting-server-select", "failed", "logging-in", "selecting-server", "waiting-delay", "waiting-kick"]) }),
      Schema.Struct({ tag: Schema.Literal("backoff"), delayMs: Schema.Number }),
    ]),
  }),
});

const readReloginRecords = (host: Pick<typeof __host, "read">): AutoReloginRecords =>
  Schema.decodeUnknownSync(Records)(JSON.parse(host.read('{"kind":"autoRelogin"}')));

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
  const answer = (id: string, value: unknown, error: string | null) => Effect.sync(() => {
    const json = JSON.stringify([id, value, error]);
    host.answer(json);
  });

  window.onBotAsk = (id, payload) => {
    if (!Queue.offerUnsafe(queue, { id, payload })) {
      host.answer(JSON.stringify([id, null, "The bot request queue is full"]));
    }
  };
  yield* Effect.forever(Effect.gen(function* () {
    const { id, payload } = yield* Queue.take(queue);
    const relogin = yield* Deferred.await(ready);
    if (!host.isAskPending(id)) return;
    const request = decodeRequest(payload);
    if (Option.isNone(request)) { yield* answer(id, null, "Invalid bot request"); return; }
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
    }).pipe(Effect.catchCause(cause => Cause.hasInterrupts(cause) ? Effect.failCause(cause) : answer(id, null, "The bot request failed")));
  })).pipe(Effect.forkScoped);
  window.onAutoReloginEventsLost = () => launch(Effect.gen(function* () {
    yield* Effect.sync(() => readReloginRecords(host));
    const relogin = yield* Deferred.await(ready);
    yield* publish(yield* relogin.getRecords());
  }));

  yield* supervise(Effect.gen(function* () {
    const stored = yield* Effect.sync(() => readReloginRecords(host));
    const relogin = yield* makeAutoRelogin(api, fibers, stored.config);
    yield* Deferred.succeed(ready, relogin);
    launch(Stream.runForEach(relogin.recordChanges, () => supervise(relogin.getRecords().pipe(Effect.flatMap(publish)))));
    if (stored.config.enabled) launch(relogin.setEnabled(true));
  })).pipe(Effect.forkScoped);
}));
