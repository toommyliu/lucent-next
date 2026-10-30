import "./host/globals";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";

import { callHost } from "./game/automation/HostCall";
import { run as settingsPolicy } from "./game/automation/SettingsPolicy";
import * as Api from "./game/flash/api/Api";
import { Bridge, type BridgeService } from "./game/flash/bridge/Bridge";
import * as Gateway from "./game/flash/bridge/Gateway";
import { HostState } from "./game/flash/state/HostState";
import { turn, wake } from "./host/loop";

const readiness = Schema.Struct({
  epoch: Schema.Number,
  failures: Schema.Record(Schema.String, Schema.String),
  missing: Schema.Array(
    Schema.Literals(["houseInventory", "inventory", "map", "player"]),
  ),
  state: Schema.Struct({
    houseInventory: Schema.Boolean,
    inventory: Schema.Boolean,
    map: Schema.Boolean,
    player: Schema.Boolean,
  }),
});

const invoke: BridgeService["invoke"] = (method, args, schema) =>
  Effect.sync(() => {
    const reply = callHost("swf", method, JSON.stringify(args ?? []));
    return reply === undefined
      ? Option.none()
      : Option.some(Schema.decodeUnknownSync(schema)(JSON.parse(reply)));
  });

const bridgeLayer = Layer.succeed(Bridge, {
  invoke,
  invokeJson: (method, args, schema) =>
    invoke(method, args, Schema.Unknown).pipe(
      Effect.map(
        Option.map((value) =>
          Schema.decodeUnknownSync(schema)(
            typeof value === "string" ? JSON.parse(value) : value,
          ),
        ),
      ),
    ),
});
const hostLayer = Layer.succeed(HostState, {
  read: (query) =>
    Effect.sync(() => {
      const value = JSON.parse(callHost("read", JSON.stringify(query)));
      if (query.kind === "readiness") Schema.decodeUnknownSync(readiness)(value);
      return value;
    }),
  command: (command) =>
    Effect.sync(() => {
      const value = JSON.parse(callHost("command", JSON.stringify(command)));
      if (command.kind === "refreshLoggedIn") {
        Schema.decodeUnknownSync(Schema.Boolean)(value);
      }
      return Option.some(value);
    }),
});
const gatewayLayer = Gateway.layer.pipe(Layer.provideMerge(bridgeLayer));
const servicesLayer = Api.layer.pipe(
  Layer.provideMerge(Layer.merge(hostLayer, gatewayLayer)),
);
const botLayer = Layer.effectDiscard(
  Effect.flatMap(Gateway.Gateway, (gateway) => gateway.start()),
).pipe(Layer.provideMerge(servicesLayer));

const runtime = ManagedRuntime.make(botLayer);
runtime.runSync(Effect.void);
runtime.runPromise(settingsPolicy);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
