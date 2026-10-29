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
import { HostState, layer as hostStateLayer } from "./game/flash/state/HostState";
import { turn, wake } from "./host/loop";

const causes = new Map<string, unknown>();
const diagnostics = Layer.succeed(DiagnosticSink, {
  report: (_phase, operation, cause) => {
    causes.set(operation, cause);
  },
});

const requireResult = (operation: string) =>
  <A>(result: Option.Option<A>) => {
    const cause = causes.get(operation);
    causes.delete(operation);
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

const commandHost = Layer.effect(
  HostState,
  Effect.map(HostState, (host) => ({
    ...host,
    command: (command) =>
      host.command(command).pipe(Effect.tap(requireResult(command.kind))),
  })),
);

const apiLayer = servicesLayer.pipe(Layer.provide(readinessBridge));

const gateway = gatewayLayer.pipe(Layer.provideMerge(bridgeLayer));
const host = commandHost.pipe(
  Layer.provide(hostStateLayer),
  Layer.provideMerge(gateway),
);
const services = apiLayer.pipe(Layer.provideMerge(host));
const botLayer = Layer.effectDiscard(
  Effect.flatMap(Gateway, (gateway) => gateway.start()),
).pipe(Layer.provideMerge(services), Layer.provide(diagnostics));
const runtime = ManagedRuntime.make(autoZone.pipe(Layer.provideMerge(botLayer)));
runtime.runSync(Effect.void);
runtime.runPromise(settingsPolicy);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
