import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";

import { Api, layer as servicesLayer } from "../flash/api/Api";
import { Bridge } from "../flash/bridge/Bridge";
import { HostState } from "../flash/state/HostState";
import { makeRetryLoop } from "./retryLoop";

const requireResult = (operation: string) =>
  <A>(result: Option.Option<A>) =>
    Option.isNone(result)
      ? Effect.die(new Error(`Failed ${operation}`))
      : Effect.void;

const bridgeLayer = Layer.effect(
  Bridge,
  Effect.map(Bridge, (bridge) => ({
    ...bridge,
    invoke: (method, args, schema) =>
      bridge.invoke(method, args, schema).pipe(Effect.tap(requireResult(method))),
  })),
);

const hostLayer = Layer.effect(
  HostState,
  Effect.map(HostState, (host) => ({
    ...host,
    command: (command) =>
      host.command(command).pipe(Effect.tap(requireResult(command.kind))),
  })),
);

export const apiLayer = servicesLayer.pipe(
  Layer.provide(bridgeLayer),
  Layer.provide(hostLayer),
);

export const run = Effect.gen(function* () {
  const api = yield* Api;
  const host = yield* HostState;
  const repeat = makeRetryLoop("settings policy");
  const tick = (actionsOnly: boolean) =>
    Effect.gen(function* () {
      if (!(yield* api.player.isReady())) return;
      const result = yield* host.command({ kind: "reapplySettings", actionsOnly });
      if (Option.isNone(result)) {
        return yield* Effect.die(new Error("Failed reapplySettings"));
      }
      if (result.value !== null) {
        return yield* Effect.die(new Error("Invalid reapplySettings reply"));
      }
    });
  yield* Effect.all(
    [
      repeat(() => tick(false), "1 second"),
      repeat(() => tick(true), "500 millis"),
    ],
    { concurrency: "unbounded", discard: true },
  );
});
