import "./host/globals";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Option from "effect/Option";

import { layer as autoZone } from "./game/automation/AutoZone";
import { run as settingsPolicy } from "./game/automation/SettingsPolicy";
import { layer as servicesLayer } from "./game/flash/api/Api";
import { Bridge, layer as bridgeLayer } from "./game/flash/bridge/Bridge";
import { DiagnosticSink } from "./game/flash/bridge/DiagnosticSink";
import { Gateway, layer as gatewayLayer } from "./game/flash/bridge/Gateway";
import { layer as hostStateLayer } from "./game/flash/state/HostState";
import { botRelogin } from "./host/botRelogin";
import { turn, wake } from "./host/loop";

const causes = new Map<string, unknown>();
const takeCause = (operation: string) => {
  const cause = causes.get(operation);
  causes.delete(operation);
  return cause;
};
const diagnostics = Layer.succeed(DiagnosticSink, {
  report: (_phase, operation, cause) => {
    if (operation === "world.isLoaded" || operation === "player.isLoaded" || operation === "reapplySettings") {
      causes.set(operation, cause);
    }
  },
  takeCause,
});

const requireResult = (operation: string) =>
  <A>(result: Option.Option<A>) => {
    const cause = takeCause(operation);
    return Option.isNone(result)
      ? Effect.die(new Error(`Failed ${operation}`, { cause }))
      : Effect.void;
  };

const readinessBridge = Layer.effect(
  Bridge,
  Effect.map(Bridge, (bridge) => ({
    ...bridge,
    invoke: (method, args, schema) =>
      bridge.invoke(method, args, schema).pipe(
        Effect.tap((result) =>
          method === "world.isLoaded" || method === "player.isLoaded"
            ? requireResult(method)(result)
            : Effect.void,
        ),
      ),
  })),
);

const apiLayer = servicesLayer.pipe(Layer.provide(readinessBridge));

const gateway = gatewayLayer.pipe(Layer.provideMerge(bridgeLayer));
const host = hostStateLayer.pipe(Layer.provideMerge(gateway));
const services = apiLayer.pipe(Layer.provideMerge(host));
const botLayer = Layer.effectDiscard(
  Effect.flatMap(Gateway, (gateway) => gateway.start()),
).pipe(Layer.provideMerge(services), Layer.provideMerge(diagnostics));
const runtime = ManagedRuntime.make(Layer.mergeAll(autoZone, botRelogin).pipe(Layer.provideMerge(botLayer)));
runtime.runSync(Effect.void);
runtime.runPromise(settingsPolicy);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
