import type { AutoReloginRecords } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";
import * as FiberMap from "effect/FiberMap";
import * as Layer from "effect/Layer";
import * as Semaphore from "effect/Semaphore";
import * as Stream from "effect/Stream";

import { makeAutoRelogin } from "../game/automation/AutoRelogin";
import { Api } from "../game/flash/api/Api";
import { publicReloginState } from "./autoRelogin";

export const botRelogin = Layer.effectDiscard(Effect.gen(function* () {
  const host = __host;
  const api = yield* Api;
  const fibers = yield* FiberMap.make<string>();
  const stored = JSON.parse(host.read('{"kind":"autoRelogin"}')) as AutoReloginRecords;
  const relogin = yield* makeAutoRelogin(api, fibers, stored.config);
  const permit = yield* Semaphore.make(1);
  const runFork = Effect.runForkWith(yield* Effect.context<never>());
  const publish = (records: AutoReloginRecords) => Effect.sync(() => host.publishAutoRelogin(JSON.stringify(records)));
  yield* Stream.runForEach(relogin.recordChanges, () => relogin.getRecords().pipe(Effect.flatMap(publish))).pipe(Effect.forkScoped);
  if (stored.config.enabled) yield* relogin.setEnabled(true).pipe(Effect.forkScoped);

  window.onBotAsk = (id, payload) => {
    const apply = Effect.gen(function* () {
      if (typeof payload !== "object" || payload === null || !("kind" in payload)) throw new TypeError("Unknown bot request");
      switch (payload.kind) {
        case "autoRelogin.setEnabled":
          if (!("enabled" in payload) || typeof payload.enabled !== "boolean") throw new TypeError("Invalid enabled value");
          yield* relogin.setEnabled(payload.enabled);
          break;
        case "autoRelogin.setDelay":
          if (!("delayMs" in payload) || typeof payload.delayMs !== "number") throw new TypeError("Invalid delay");
          yield* relogin.setDelay(payload.delayMs);
          break;
        case "autoRelogin.setServer":
          if (!("server" in payload) || (payload.server !== null && typeof payload.server !== "string")) throw new TypeError("Invalid server");
          yield* relogin.setServer(payload.server ?? undefined);
          break;
        default: throw new TypeError("Unknown bot request");
      }
      const records = yield* relogin.getRecords();
      yield* publish(records);
      host.answer(JSON.stringify([id, publicReloginState(records), null]));
    });
    runFork(permit.withPermits(1)(apply).pipe(Effect.catchCause(() => Effect.sync(() => {
      host.answer(JSON.stringify([id, null, "The bot request failed"]));
    }))));
  };
  window.onAutoReloginEventsLost = () => {
    runFork(Effect.sync(() => host.read('{"kind":"autoRelogin"}')).pipe(
      Effect.andThen(relogin.getRecords()), Effect.flatMap(publish),
    ));
  };
}));
