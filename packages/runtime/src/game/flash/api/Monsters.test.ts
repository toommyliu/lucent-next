import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as TestClock from "effect/testing/TestClock";

import {
  emitExtension,
  item,
  makeFakeApi,
  makeFakeHostState,
  monster,
  player,
} from "../testing/FakeHost";

const bone = {
  eventDrop: false,
  icon: "",
  item: item("monster-drop", { itemId: 9, name: "Bone" }),
  questGated: false,
  questObjectives: [],
  rarity: 10,
  rarityName: "Common",
  rateBoostPercent: null,
  ratePercent: 50,
  requiredQuestIds: [],
  requiredQuests: [],
  stackSize: 1,
  variableQuantity: false,
};

describe("Monsters", () => {
  it.effect(
    "returns a monster with the drops that arrived while it waited",
    () =>
      Effect.gen(function* () {
        const state = makeFakeHostState();
        state.map = { id: 1, name: "battleon", roomNumber: 1 };
        state.players = [player({ entityId: 1, username: "Hero" })];
        state.selfEntityId = 1;
        state.monsters = [monster({ monsterMapId: 2, name: "Frogzard" })];
        const target = {} as Window;
        target.swf = {
          "world.requestMonsterDrops": (monsterMapId: number) => {
            state.monsters = state.monsters.map((entry) =>
              entry.monsterMapId === monsterMapId
                ? { ...entry, drops: [bone] }
                : entry,
            );
            emitExtension(target, {
              cmd: "monsterDrops",
              items: {},
              MonMapID: monsterMapId,
            });
            return true;
          },
        } as unknown as Window["swf"];
        const api = yield* makeFakeApi(target, state);

        const lookup = yield* api.monsters
          .get("Frogzard")
          .pipe(Effect.forkChild);
        yield* TestClock.adjust("1 second");
        const found = yield* Fiber.join(lookup);

        expect(found?.drops.map((drop) => drop.item.name)).toEqual(["Bone"]);
      }),
  );
});
