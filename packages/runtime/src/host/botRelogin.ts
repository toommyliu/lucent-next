import type { AutoReloginRecords } from "@lucent/game/protocol";
import * as Deferred from "effect/Deferred";
import * as Effect from "effect/Effect";
import * as FiberMap from "effect/FiberMap";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
import * as Semaphore from "effect/Semaphore";
import * as Stream from "effect/Stream";

import { makeAutoRelogin, type AutoRelogin } from "../game/automation/AutoRelogin";
import { callReloginHost, makeReloginSupervisor } from "../game/automation/AutoReloginFailureRule";
import { Api } from "../game/flash/api/Api";
import { Gateway } from "../game/flash/bridge/Gateway";
import { publicReloginState } from "./autoRelogin";
import { makeBotReloginApi, readReloginRecords } from "./botReloginApi";

const decodeRequest = Schema.decodeUnknownOption(Schema.Union([
  Schema.Struct({ kind: Schema.Literal("autoRelogin.setEnabled"), enabled: Schema.Boolean }),
  Schema.Struct({ kind: Schema.Literal("autoRelogin.setDelay"), delayMs: Schema.Number }),
  Schema.Struct({ kind: Schema.Literal("autoRelogin.setServer"), server: Schema.NullOr(Schema.String) }),
]));

export const botRelogin = Layer.effectDiscard(Effect.gen(function* () {
  const host = __host;
  const api = makeBotReloginApi(yield* Api, yield* Gateway, host);
  const fibers = yield* FiberMap.make<string>();
  const ready = yield* Deferred.make<AutoRelogin>();
  const permit = yield* Semaphore.make(1);
  const runFork = Effect.runForkWith(yield* Effect.context<never>());
  let terminated = false;
  const supervise = makeReloginSupervisor((error) => {
    if (terminated) return;
    terminated = true;
    void Promise.reject(error);
  });
  const launch = <A, E>(effect: Effect.Effect<A, E>) => { runFork(supervise(effect)); };
  const publish = (records: AutoReloginRecords) => Effect.sync(() => {
    const json = JSON.stringify(records);
    callReloginHost(() => host.publishAutoRelogin(json));
  });
  const answer = (id: string, value: unknown, error: string | null) => Effect.sync(() => {
    const json = JSON.stringify([id, value, error]);
    callReloginHost(() => host.answer(json));
  });

  window.onBotAsk = (id, payload) => {
    const request = decodeRequest(payload);
    if (Option.isNone(request)) { launch(answer(id, null, "Invalid bot request")); return; }
    const apply = Effect.gen(function* () {
      const relogin = yield* Deferred.await(ready);
      const input = request.value;
      switch (input.kind) {
        case "autoRelogin.setEnabled": yield* relogin.setEnabled(input.enabled); break;
        case "autoRelogin.setDelay": yield* relogin.setDelay(input.delayMs); break;
        case "autoRelogin.setServer": yield* relogin.setServer(input.server ?? undefined); break;
      }
      const records = yield* relogin.getRecords();
      yield* publish(records);
      yield* answer(id, publicReloginState(records), null);
    });
    launch(permit.withPermits(1)(supervise(apply)));
  };
  window.onAutoReloginEventsLost = () => launch(Effect.gen(function* () {
    yield* Effect.sync(() => readReloginRecords(host));
    const relogin = yield* Deferred.await(ready);
    yield* publish(yield* relogin.getRecords());
  }));

  yield* supervise(Effect.gen(function* () {
    const stored = yield* Effect.sync(() => readReloginRecords(host));
    const relogin = yield* makeAutoRelogin(api, fibers, stored.config, supervise);
    yield* Deferred.succeed(ready, relogin);
    launch(Stream.runForEach(relogin.recordChanges, () => supervise(relogin.getRecords().pipe(Effect.flatMap(publish)))));
    if (stored.config.enabled) launch(relogin.setEnabled(true));
  })).pipe(Effect.forkScoped);
}));
