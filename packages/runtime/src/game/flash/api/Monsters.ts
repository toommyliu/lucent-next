import { toLiveMonster } from "@lucent/game";
import type { LiveMonster, MonsterQuery } from "@lucent/game";
import * as Deferred from "effect/Deferred";
import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import * as Queue from "effect/Queue";
import * as Schedule from "effect/Schedule";
import * as Schema from "effect/Schema";
import * as Stream from "effect/Stream";

import type { BridgeService } from "../bridge/Bridge";
import { packetData } from "../contract/Packet";
import { decodeMonsterDrops } from "../contract/payload/MonsterDrops";
import { isSelectable, type HostStateService } from "../state/HostState";
import type { Events } from "./Events";
import type { Wait } from "./Wait";

const monsterDropRequestSpacing = Schedule.spaced("350 millis").pipe(
  Schedule.jittered,
);

interface MonsterDropRequest {
  readonly done: Deferred.Deferred<void>;
  readonly key: string;
  readonly monsterMapId: number;
}

export const makeMonsters = Effect.fnUntraced(function* (
  bridge: BridgeService,
  host: HostStateService,
  events: Events,
  wait: Wait,
) {
  const readMonster = (selector: MonsterQuery) =>
    isSelectable(selector)
      ? host
          .read({ kind: "monster", selector })
          .pipe(
            Effect.map((monster) =>
              monster === null ? null : toLiveMonster(monster),
            ),
          )
      : Effect.succeed(null);
  const readMonsters = () =>
    host
      .read({ kind: "monsters" })
      .pipe(Effect.map((monsters) => monsters.map(toLiveMonster)));
  const readDrops = (monsterMapId: number) =>
    host.read({ kind: "monsterDrops", monsterMapId });

  const requests = yield* Queue.unbounded<MonsterDropRequest>();
  const inFlight = new Map<string, Deferred.Deferred<void>>();

  yield* Effect.addFinalizer(() => Queue.shutdown(requests));

  const requestKey = Effect.fn("Monsters.requestKey")(function* (
    monsterMapId: number,
  ) {
    const map = yield* host.read({ kind: "map" });
    return `${map.id}:${map.name}:${map.roomNumber}:${monsterMapId}`;
  });

  const isInCurrentCell = Effect.fn("Monsters.isInCurrentCell")(function* (
    monster: LiveMonster,
  ) {
    const player = yield* host.read({ kind: "me" });
    return player !== null && monster.isInCell(player.cell);
  });

  const executeRequest = Effect.fn("Monsters.executeDropRequest")(function* (
    request: MonsterDropRequest,
  ) {
    const operation = Effect.gen(function* () {
      if ((yield* requestKey(request.monsterMapId)) !== request.key) return;
      if ((yield* readDrops(request.monsterMapId)) !== null) return;

      const monster = yield* readMonster(request.monsterMapId);
      if (monster === null || !(yield* isInCurrentCell(monster))) return;

      yield* wait.forPacket(
        {
          command: "monsterDrops",
          direction: "extension",
          predicate: (packet) => {
            const decoded = decodeMonsterDrops(packetData(packet));
            return (
              Option.isSome(decoded) &&
              decoded.value.MonMapID === request.monsterMapId
            );
          },
          encoding: "json",
        },
        {
          timeout: "3 seconds",
          trigger: bridge
            .invoke(
              "world.requestMonsterDrops",
              [request.monsterMapId],
              Schema.Boolean,
            )
            .pipe(Effect.map(Option.getOrElse(() => false))),
        },
      );
    });

    yield* operation.pipe(
      Effect.ensuring(
        Deferred.succeed(request.done, undefined).pipe(
          Effect.andThen(
            Effect.sync(() => {
              if (inFlight.get(request.key) === request.done) {
                inFlight.delete(request.key);
              }
            }),
          ),
          Effect.asVoid,
        ),
      ),
    );
  });

  // Pace every native request through one queue while response waits overlap.
  yield* Stream.fromQueue(requests).pipe(
    Stream.schedule(monsterDropRequestSpacing),
    Stream.mapEffect((request) => executeRequest(request), {
      concurrency: "unbounded",
    }),
    Stream.runDrain,
    Effect.forkScoped,
  );

  const ensureDrops = Effect.fn("Monsters.ensureDrops")(function* (
    monster: LiveMonster,
  ) {
    if ((yield* readDrops(monster.monsterMapId)) !== null) return;
    if (!(yield* isInCurrentCell(monster))) return;

    const key = yield* requestKey(monster.monsterMapId);
    const active = inFlight.get(key);
    if (active !== undefined) {
      yield* Deferred.await(active);
      return;
    }

    const done = Deferred.makeUnsafe<void>();
    inFlight.set(key, done);
    yield* Queue.offer(requests, {
      done,
      key,
      monsterMapId: monster.monsterMapId,
    });
    yield* Deferred.await(done);
  });

  const ensureAllDrops = (monsters: readonly LiveMonster[]) =>
    Effect.forEach(monsters, ensureDrops, {
      concurrency: "unbounded",
      discard: true,
    });

  const getProjected = (selector: MonsterQuery) =>
    readMonster(selector).pipe(
      Effect.flatMap((current) =>
        current !== null || !isSelectable(selector)
          ? Effect.succeed(current)
          : host
              .command({ kind: "hydrateMonster", selector })
              .pipe(
                Effect.map((hydrated) =>
                  Option.isSome(hydrated) && hydrated.value !== null
                    ? toLiveMonster(hydrated.value)
                    : null,
                ),
              ),
      ),
    );

  const getAvailableProjected = () =>
    bridge
      .invoke(
        "world.getAvailableMonsterMapIds",
        undefined,
        Schema.Array(Schema.Number),
      )
      .pipe(
        Effect.flatMap(
          Option.match({
            onNone: () => Effect.succeed([]),
            onSome: (ids) =>
              readMonsters().pipe(
                Effect.map((monsters) => {
                  const available = new Set(ids);
                  return monsters.filter((monster) =>
                    available.has(monster.monsterMapId),
                  );
                }),
              ),
          }),
        ),
      );

  const ensureCurrentCell = Effect.fn("Monsters.ensureCurrentCell")(function* (
    monsters: readonly LiveMonster[],
  ) {
    const player = yield* host.read({ kind: "me" });
    if (player === null) return;
    yield* ensureAllDrops(
      monsters.filter((monster) => monster.isInCell(player.cell)),
    );
  });

  yield* events.on({ type: "join-map" }, () =>
    readMonsters().pipe(Effect.flatMap(ensureCurrentCell)),
  );
  yield* events.on({ type: "player-location" }, (event) =>
    host
      .read({ kind: "me" })
      .pipe(
        Effect.flatMap((player) =>
          player?.entityId === event.entityId
            ? readMonsters().pipe(Effect.flatMap(ensureCurrentCell))
            : Effect.void,
        ),
      ),
  );

  const reread = (monsters: readonly LiveMonster[]) => {
    const ids = new Set(monsters.map((monster) => monster.monsterMapId));
    return readMonsters().pipe(
      Effect.map((current) =>
        current.filter((monster) => ids.has(monster.monsterMapId)),
      ),
    );
  };

  const get = (selector: MonsterQuery) =>
    getProjected(selector).pipe(
      Effect.flatMap((monster) =>
        monster === null
          ? Effect.succeed(null)
          : ensureDrops(monster).pipe(
              Effect.andThen(readMonster(monster.monsterMapId)),
            ),
      ),
    );
  const getAll = () =>
    readMonsters().pipe(
      Effect.flatMap(ensureCurrentCell),
      Effect.andThen(readMonsters()),
    );
  const getAvailable = () =>
    getAvailableProjected().pipe(
      Effect.tap(ensureAllDrops),
      Effect.flatMap(reread),
    );
  const isAvailable = (selector: MonsterQuery) =>
    getProjected(selector).pipe(
      Effect.flatMap((monster) =>
        monster === null
          ? Effect.succeed(false)
          : bridge
              .invoke(
                "world.isMonsterAvailable",
                [monster.monsterMapId],
                Schema.Boolean,
              )
              .pipe(Effect.map(Option.getOrElse(() => false))),
      ),
    );

  return {
    api: {
      get,
      getAll,
      getAvailable,
      isAvailable,
    },
    lookup: {
      get: getProjected,
      getAll: readMonsters,
      getAvailable: getAvailableProjected,
    },
  };
});

export type MonsterServices = Effect.Success<ReturnType<typeof makeMonsters>>;
export type MonsterLookup = MonsterServices["lookup"];
export type Monsters = MonsterServices["api"];
