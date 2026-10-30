import "./host/globals";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";

import {
  apiLayer as policyApiLayer,
  run as settingsPolicy,
} from "./game/automation/SettingsPolicy";
import { Api } from "./game/flash/api/Api";
import { layer as bridgeLayer } from "./game/flash/bridge/Bridge";
import { Gateway, layer as gatewayLayer } from "./game/flash/bridge/Gateway";
import type { Event } from "./game/flash/contract/Event";
import { layer as hostStateLayer } from "./game/flash/state/HostState";
import { turn, wake } from "./host/loop";

const gateway = gatewayLayer.pipe(Layer.provideMerge(bridgeLayer));
const host = hostStateLayer.pipe(Layer.provideMerge(gateway));
const services = policyApiLayer.pipe(Layer.provideMerge(host));
const botLayer = Layer.effectDiscard(
  Effect.flatMap(Gateway, (gateway) => gateway.start()),
).pipe(Layer.provideMerge(services));
const runtime = ManagedRuntime.make(botLayer);
runtime.runSync(Effect.void);
runtime.runPromise(settingsPolicy);

const onEvent = (listener: (event: Event) => void) =>
  runtime.runPromise(
    Effect.flatMap(Api, (api) =>
      api.events.on(undefined, (event) => Effect.sync(() => listener(event))),
    ),
  );

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake, onEvent }) });
