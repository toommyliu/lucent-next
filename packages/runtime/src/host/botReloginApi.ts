import type { AutoReloginRecords, QueryOutputs, CommandOutputs } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";

import type { AutoReloginApi } from "../game/automation/AutoRelogin";
import { makeAuth } from "../game/flash/api/Auth";
import { makeMap } from "../game/flash/api/Map";
import { makePlayerReadiness } from "../game/flash/api/Player";
import { makeWaitApi } from "../game/flash/api/Wait";
import type { ApiService } from "../game/flash/api/Api";
import type { BridgeService } from "../game/flash/bridge/Bridge";
import type { GatewayService } from "../game/flash/bridge/Gateway";
import type { HostStateService } from "../game/flash/state/HostState";

const Server = Schema.Struct({
  chat: Schema.Number, count: Schema.Number, language: Schema.String, max: Schema.Number,
  memberOnly: Schema.Boolean, name: Schema.String, online: Schema.Boolean, full: Schema.Boolean,
});
const Servers = Schema.Array(Server);
const Credentials = Schema.Struct({ username: Schema.String, password: Schema.String });
const Readiness = Schema.Struct({
  epoch: Schema.Number,
  failures: Schema.Record(Schema.String, Schema.String),
  missing: Schema.Array(Schema.Literals(["houseInventory", "inventory", "map", "player"])),
  state: Schema.Struct({ houseInventory: Schema.Boolean, inventory: Schema.Boolean, map: Schema.Boolean, player: Schema.Boolean }),
});
const Records = Schema.Struct({
  config: Schema.Struct({ enabled: Schema.Boolean, delayMs: Schema.Number, server: Schema.NullOr(Schema.String) }),
  status: Schema.Struct({
    captured: Schema.Boolean, attemptsRemaining: Schema.NullOr(Schema.Number), lastError: Schema.NullOr(Schema.String),
    phase: Schema.Union([
      Schema.Struct({ tag: Schema.Literals(["armed", "awaiting-session", "awaiting-player-ready", "awaiting-server-select", "failed", "logging-in", "selecting-server", "waiting-delay", "waiting-kick"]) }),
      Schema.Struct({ tag: Schema.Literal("backoff"), delayMs: Schema.Number }),
    ]),
  }),
});

export const readReloginRecords = (host: Pick<typeof __host, "read">): AutoReloginRecords =>
  Schema.decodeUnknownSync(Records)(JSON.parse(host.read('{"kind":"autoRelogin"}')));

export const makeBotReloginApi = (
  api: ApiService,
  gateway: GatewayService,
  native: Pick<typeof __host, "swf" | "read" | "command">,
): AutoReloginApi => {
  const call = (method: string, args: readonly unknown[] | undefined) => Effect.sync((): unknown => {
    const json = JSON.stringify(args ?? []);
    const reply = native.swf(method, json);
    return reply === undefined ? undefined : JSON.parse(reply);
  });
  const bridge: BridgeService = {
    invoke: (method, args, schema) => call(method, args).pipe(
      Effect.flatMap(Schema.decodeUnknownEffect(schema)), Effect.orDie, Effect.map(Option.some),
    ),
    invokeJson: (method, args, schema) => call(method, args).pipe(
      Effect.map((value): unknown => typeof value === "string" ? JSON.parse(value) : value),
      Effect.flatMap(Schema.decodeUnknownEffect(schema)), Effect.orDie, Effect.map(Option.some),
    ),
  };
  const host: HostStateService = {
    read: (query) => Effect.sync(() => {
      const json = JSON.stringify(query);
      const value: unknown = JSON.parse(native.read(json));
      if (query.kind !== "readiness") throw new Error(`Unexpected relogin query: ${query.kind}`);
      return Schema.decodeUnknownSync(Readiness)(value) as QueryOutputs[typeof query.kind];
    }),
    command: (command) => Effect.sync(() => {
      const json = JSON.stringify(command);
      const value: unknown = JSON.parse(native.command(json));
      let decoded: unknown;
      switch (command.kind) {
        case "refreshCredentials": decoded = Schema.decodeUnknownSync(Credentials)(value); break;
        case "refreshServers": decoded = Schema.decodeUnknownSync(Servers)(value); break;
        case "refreshLoggedIn":
        case "login": decoded = Schema.decodeUnknownSync(Schema.Boolean)(value); break;
        case "logout": decoded = Schema.decodeUnknownSync(Schema.Null)(value); break;
        default: throw new Error(`Unexpected relogin command: ${command.kind}`);
      }
      return Option.some(decoded as CommandOutputs[typeof command.kind]);
    }),
  };
  const wait = makeWaitApi(bridge, gateway);
  const auth = makeAuth(bridge, host, wait);
  const map = makeMap(bridge, host, wait);
  return { auth, events: api.events, player: { isReady: makePlayerReadiness(host, auth, map, bridge) } };
};
