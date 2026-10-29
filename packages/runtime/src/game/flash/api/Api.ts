import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";

import { Bridge } from "../bridge/Bridge";
import { Gateway } from "../bridge/Gateway";
import { isProjectionEvent } from "../contract/Event";
import { HostState } from "../state/HostState";
import { makeStore } from "../state/Store";
import { makeAuth } from "./Auth";
import { makeAntiCounter } from "./internal/AntiCounter";
import { makeSessionEvents } from "./internal/SessionEvents";
import { makeBank } from "./Bank";
import { makeCombat } from "./Combat";
import { makeDrops } from "./Drops";
import { makeEvents } from "./Events";
import { makeHouse } from "./House";
import { makeInventory } from "./Inventory";
import { makeMap } from "./Map";
import { makeMonsters } from "./Monsters";
import { makePacket } from "./Packet";
import { makePlayer } from "./Player";
import { makePlayers } from "./Players";
import {
  makeProjectionReadiness,
  ProjectionReadiness,
} from "./ProjectionReadiness";
import { makeQuests } from "./Quests";
import { makeSettings } from "./Settings";
import { makeShops } from "./Shops";
import { makeTempInventory } from "./TempInventory";
import { makeWaitApi } from "./Wait";

const makeApiServices = Effect.gen(function* () {
  const bridge = yield* Bridge;
  const gateway = yield* Gateway;
  const host = yield* HostState;
  const store = yield* makeStore;
  const wait = makeWaitApi(bridge, gateway);
  const events = yield* makeEvents(gateway, wait);
  const packet = yield* makePacket(gateway, host, wait);
  const auth = makeAuth(bridge, host, wait);
  const house = makeHouse(bridge, host);
  const inventory = makeInventory(bridge, host, house, wait);
  const bank = yield* makeBank(bridge, host, auth, inventory, house, wait);
  const drops = yield* makeDrops(bridge, host, auth, wait);
  const map = makeMap(bridge, host, wait);
  const monsterServices = yield* makeMonsters(bridge, host, events, wait);
  const monsters = monsterServices.api;
  const players = makePlayers(host);
  const player = makePlayer(bridge, host, auth, inventory, map, wait);
  const sessionEvents = yield* makeSessionEvents(
    wait.until(player.isReady()).pipe(Effect.asVoid),
    gateway.publishEvent,
  );
  const projectionReadiness = makeProjectionReadiness(host);
  const quests = makeQuests(bridge, host, wait);
  const settings = yield* makeSettings(bridge, store);
  const antiCounter = makeAntiCounter(bridge, settings.isAntiCounterEnabled);
  const shops = makeShops(bridge, host, inventory, wait);
  const tempInventory = makeTempInventory(host);
  const combat = makeCombat(
    bridge,
    antiCounter,
    host,
    drops,
    events,
    inventory,
    map,
    monsterServices.lookup,
    player,
    players,
    settings,
    tempInventory,
    wait,
  );
  yield* gateway.start((event) =>
    isProjectionEvent(event)
      ? antiCounter.handleEvent(event)
      : sessionEvents
          .handleRuntimeEvent(event)
          .pipe(Effect.andThen(antiCounter.handleEvent(event))),
  );

  return {
    api: {
      auth,
      bank,
      combat,
      drops,
      events,
      house,
      inventory,
      map,
      monsterLookup: monsterServices.lookup,
      monsters,
      packet,
      player,
      players,
      quests,
      settings,
      shops,
      tempInventory,
      wait,
    },
    projectionReadiness,
  };
});

export type ApiService = Effect.Success<typeof makeApiServices>["api"];

export const makeApi = makeApiServices.pipe(Effect.map(({ api }) => api));

export class Api extends Context.Service<Api, ApiService>()(
  "lucent/renderer/flash/Api",
) {}

export const layer = Layer.effectContext(
  makeApiServices.pipe(
    Effect.map(({ api, projectionReadiness }) =>
      Context.make(Api, api).pipe(
        Context.add(ProjectionReadiness, projectionReadiness),
      ),
    ),
  ),
);
