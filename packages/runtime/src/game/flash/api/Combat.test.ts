import { describe, expect, it } from "@effect/vitest";
import { EntityState } from "@lucent/game";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as TestClock from "effect/testing/TestClock";

import {
  makeFakeApi,
  makeFakeHostState,
  monster,
  player,
  type FakeHostState,
} from "../testing/FakeHost";

const frogzard = monster({ cell: "r2", monsterMapId: 2, name: "Frogzard" });

const killFrogzard = (onAttack: (state: FakeHostState) => void) =>
  Effect.gen(function* () {
    const state = makeFakeHostState();
    state.map = { id: 1, name: "battleon", roomNumber: 1 };
    state.players = [player({ entityId: 1, username: "Hero" })];
    state.selfEntityId = 1;
    state.monsters = [frogzard];
    const target = {} as Window;
    target.swf = {
      "combat.attackMonster": () => {
        onAttack(state);
        return true;
      },
      "world.getAvailableMonsterMapIds": () =>
        state.monsters.map((entry) => entry.monsterMapId),
    } as unknown as Window["swf"];
    const api = yield* makeFakeApi(target, state);

    const fight = yield* api.combat.kill("Frogzard").pipe(Effect.forkChild);
    yield* TestClock.adjust("1 second");
    return yield* Fiber.join(fight);
  });

describe("Combat.kill", () => {
  it.effect("ends in a death when the monster dies", () =>
    Effect.gen(function* () {
      const killed = yield* killFrogzard((state) => {
        state.monsters = [
          monster({
            cell: "r2",
            hp: 0,
            monsterMapId: 2,
            name: "Frogzard",
            state: EntityState.Dead,
          }),
        ];
      });
      expect(killed).toBe(true);
    }),
  );

  it.effect("ends without a death when the monster is gone", () =>
    Effect.gen(function* () {
      const killed = yield* killFrogzard((state) => {
        state.monsters = [];
      });
      expect(killed).toBe(false);
    }),
  );

  it.effect("ends without a death when the map changes", () =>
    Effect.gen(function* () {
      const killed = yield* killFrogzard((state) => {
        state.map = { id: 1, name: "battleon", roomNumber: 2 };
      });
      expect(killed).toBe(false);
    }),
  );
});
