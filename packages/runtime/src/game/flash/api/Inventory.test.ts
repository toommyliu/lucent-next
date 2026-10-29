import { describe, expect, it } from "@effect/vitest";
import type { ItemSnapshot } from "@lucent/game";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as PubSub from "effect/PubSub";
import * as Queue from "effect/Queue";
import * as TestClock from "effect/testing/TestClock";
import * as Option from "effect/Option";

import type { EquipEnhancementSelector } from "../../EnhancementSelectors";
import type { BridgeService } from "../bridge/Bridge";
import type { Event } from "../contract/Event";
import { makeWait } from "../protocol/Wait";
import type { Packet } from "../contract/Packet";
import {
  editItem,
  item,
  makeFakeHost,
  makeFakeHostState,
} from "../testing/FakeHost";
import { makeHouse } from "./House";
import { makeInventory } from "./Inventory";
import type { Wait } from "./Wait";

const equipment = ({
  enhancementLevel,
  enhancementPatternId,
  enhancementProcId,
  equipmentSlot,
  equipped = false,
  itemId,
}: {
  readonly enhancementLevel: number;
  readonly enhancementPatternId: number;
  readonly enhancementProcId?: number;
  readonly equipmentSlot: string;
  readonly equipped?: boolean;
  readonly itemId: number;
}) =>
  item("inventory", {
    category: equipmentSlot === "ar" ? "Class" : "Equipment",
    enhancement: {
      level: enhancementLevel,
      patternId: enhancementPatternId,
      ...(enhancementProcId === undefined ? {} : { procId: enhancementProcId }),
    },
    equipped,
    equipmentSlot,
    itemId,
    name: `Item ${itemId}`,
  });

const consumable = (
  itemId: number,
  link: string,
  equipped = false,
  category = "Item",
  actionId = 400,
): ItemSnapshot =>
  item("inventory", {
    category,
    description: "Consumable",
    equipped,
    file: "consumable.swf",
    itemId,
    link,
    meta: String(actionId),
    name: `Consumable ${itemId}`,
    quantity: 2,
  });

const makeHarness = (items: readonly ItemSnapshot[], autoConfirm = true) =>
  Effect.gen(function* () {
    const state = makeFakeHostState();
    state.items.inventory = [...items];
    state.hydration.inventory = 1;

    const packets = yield* PubSub.unbounded<Packet>();
    const events = yield* PubSub.unbounded<Event>();
    const sentEquips = yield* Queue.unbounded<number>();
    const confirmEquip = (data: unknown) =>
      PubSub.publish(packets, {
        command: "seia",
        direction: "extension",
        encoding: "json",
        raw: "",
        data,
      });
    let slotItemId: number | undefined;
    const equippedItemIds: number[] = [];
    const unequippedItemIds: number[] = [];
    const usedItemIds: number[] = [];
    const bridge = {
      invoke: (method: string, args: readonly unknown[] | undefined) =>
        Effect.gen(function* () {
          if (method === "inventory.equip") {
            const selector = args?.[0] as { readonly itemId: number };
            slotItemId = selector.itemId;
            equippedItemIds.push(selector.itemId);
            yield* Queue.offer(sentEquips, selector.itemId);
            if (
              items.find((candidate) => candidate.itemId === selector.itemId)
                ?.category === "Item"
            ) {
              if (autoConfirm)
                yield* confirmEquip({
                  iRes: 1,
                  o: { id: 400, cd: 10000, tgt: "h" },
                });
            } else {
              yield* PubSub.publish(packets, {
                command: "equipItem",
                direction: "extension",
                encoding: "json",
                raw: "",
                data: { ItemID: selector.itemId, uid: 1 },
              });
            }
            return Option.some(true);
          }
          if (method === "combat.getConsumableSkillItem") {
            return Option.some(
              slotItemId === undefined
                ? null
                : { itemId: slotItemId, ready: false },
            );
          }
          if (method === "inventory.unequipConsumable") {
            const selector = args?.[0] as { readonly itemId: number };
            unequippedItemIds.push(selector.itemId);
            slotItemId = undefined;
            return Option.some(true);
          }
          if (method === "inventory.use") {
            const selector = args?.[0] as { readonly itemId: number };
            const selected = state.items.inventory.find(
              (candidate) => candidate.itemId === selector.itemId,
            );
            if (selected === undefined) return Option.some(false);
            usedItemIds.push(selector.itemId);
            state.items.inventory = state.items.inventory.map((candidate) =>
              candidate === selected
                ? editItem(candidate, { quantity: candidate.quantity - 1 })
                : candidate,
            );
            return Option.some(true);
          }
          if (method === "player.getUserId") return Option.some(1);
          if (method === "player.isMember") return Option.some(true);
          return Option.none();
        }),
    } as unknown as BridgeService;
    const host = makeFakeHost(state, {
      swf: {
        "combat.getConsumableSkillItem": () =>
          slotItemId === undefined ? null : { itemId: slotItemId },
      } as unknown as Window["swf"],
    });
    const house = makeHouse(bridge, host);
    const wait = {
      ...makeWait({
        subscribePackets: PubSub.subscribe(packets),
        subscribeEvents: PubSub.subscribe(events),
      }),
      forGameAction: () => Effect.succeed(true),
      isGameActionAvailable: () => Effect.succeed(true),
    } satisfies Wait;

    return {
      confirmEquip,
      nextEquip: Queue.take(sentEquips),
      equippedItemIds,
      inventory: makeInventory(bridge, host, house, wait),
      unequippedItemIds,
      usedItemIds,
    };
  });

describe("Inventory.equipByEnhancement", () => {
  it.effect(
    "supports the enhancement selector forms accepted by enhanceItem",
    () =>
      Effect.gen(function* () {
        const best = equipment({
          enhancementLevel: 30,
          enhancementPatternId: 2,
          enhancementProcId: 14,
          equipmentSlot: "Weapon",
          itemId: 2,
        });
        const { equippedItemIds, inventory } = yield* makeHarness([
          equipment({
            enhancementLevel: 26,
            enhancementPatternId: 2,
            enhancementProcId: 14,
            equipmentSlot: "Weapon",
            itemId: 1,
          }),
          best,
          equipment({
            enhancementLevel: 35,
            enhancementPatternId: 10,
            enhancementProcId: 14,
            equipmentSlot: "Weapon",
            itemId: 3,
          }),
        ]);
        const selectors: readonly EquipEnhancementSelector[] = [
          { enhancement: "dauntless" },
          { enhancement: "forge", special: "dauntless" },
          { enhancement: "fighter", special: "dauntless" },
        ];

        for (const selector of selectors) {
          expect(yield* inventory.equipByEnhancement(selector)).toBe(true);
        }
        expect(equippedItemIds).toEqual([
          best.itemId,
          best.itemId,
          best.itemId,
        ]);
      }),
  );

  it.effect("filters by slot and ranks by enhancement level", () =>
    Effect.gen(function* () {
      const { equippedItemIds, inventory } = yield* makeHarness([
        equipment({
          enhancementLevel: 30,
          enhancementPatternId: 10,
          equipmentSlot: "he",
          itemId: 10,
        }),
        equipment({
          enhancementLevel: 40,
          enhancementPatternId: 10,
          equipmentSlot: "ba",
          itemId: 11,
        }),
        equipment({
          enhancementLevel: 35,
          enhancementPatternId: 10,
          equipmentSlot: "he",
          itemId: 12,
        }),
      ]);

      expect(
        yield* (
          inventory.equipByEnhancement as (
            selector: unknown,
          ) => Effect.Effect<boolean>
        )({
          enhancement: " forge ",
          slot: " HELM ",
        }),
      ).toBe(true);
      expect(equippedItemIds).toEqual([12]);
    }),
  );

  it.effect("rejects ambiguous or malformed selectors", () =>
    Effect.gen(function* () {
      const { equippedItemIds, inventory } = yield* makeHarness([
        equipment({
          enhancementLevel: 30,
          enhancementPatternId: 10,
          equipmentSlot: "he",
          itemId: 10,
        }),
      ]);
      const equipByEnhancement = inventory.equipByEnhancement as (
        selector: unknown,
      ) => Effect.Effect<boolean>;

      expect(yield* equipByEnhancement({ enhancement: "forge" })).toBe(false);
      expect(
        yield* equipByEnhancement({ enhancement: " ", slot: "helm" }),
      ).toBe(false);
      expect(
        yield* equipByEnhancement({
          enhancement: "forge",
          slot: "armor",
        }),
      ).toBe(false);
      expect(equippedItemIds).toEqual([]);
    }),
  );
});

describe("Inventory consumable equipment", () => {
  it.effect("projects local equip and unequip mutations", () =>
    Effect.gen(function* () {
      const first = consumable(100, "potion");
      const second = consumable(101, "scroll", true);
      const harness = yield* makeHarness([first, second]);

      expect(yield* harness.inventory.equip(first.itemId)).toBe(true);
      expect((yield* harness.inventory.get(first.itemId))?.equipped).toBe(true);
      expect((yield* harness.inventory.get(second.itemId))?.equipped).toBe(
        false,
      );
      expect(harness.equippedItemIds).toEqual([first.itemId]);

      expect(yield* harness.inventory.unequipConsumable(first.itemId)).toBe(
        true,
      );
      expect((yield* harness.inventory.get(first.itemId))?.equipped).toBe(
        false,
      );
      expect(harness.unequippedItemIds).toEqual([first.itemId]);
    }),
  );

  it.effect(
    "does not let a timed-out equip's late response confirm the next equip",
    () =>
      Effect.gen(function* () {
        const potion = consumable(100, "potion");
        const scroll = consumable(101, "scroll", false, "Item", 482);
        const harness = yield* makeHarness([potion, scroll], false);
        const first = yield* harness.inventory
          .equip(potion.itemId)
          .pipe(Effect.forkChild);
        yield* harness.nextEquip;
        yield* TestClock.adjust("5 seconds");
        expect(yield* Fiber.join(first)).toBe(false);
        expect((yield* harness.inventory.get(potion.itemId))?.equipped).toBe(
          false,
        );
        const second = yield* harness.inventory
          .equip(scroll.itemId)
          .pipe(Effect.forkChild);
        expect(yield* harness.nextEquip).toBe(scroll.itemId);
        yield* harness.confirmEquip({
          iRes: 1,
          o: { id: 400, cd: 60000, tgt: "s" },
        });
        yield* TestClock.adjust("100 millis");
        expect(second.pollUnsafe()).toBeUndefined();
        expect((yield* harness.inventory.get(potion.itemId))?.equipped).toBe(
          false,
        );
        yield* harness.confirmEquip({
          iRes: 1,
          o: { id: 482, cd: 20000, tgt: "h" },
        });
        expect(yield* Fiber.join(second)).toBe(true);
        expect((yield* harness.inventory.get(scroll.itemId))?.equipped).toBe(
          true,
        );
        expect(yield* harness.inventory.unequipConsumable(scroll.itemId)).toBe(
          true,
        );
      }),
  );

  it.effect("uses but does not equip direct-use tonics", () =>
    Effect.gen(function* () {
      const tonic = consumable(102, "ToNiC");
      const harness = yield* makeHarness([tonic]);

      expect(yield* harness.inventory.equip(tonic.itemId)).toBe(false);
      expect(yield* harness.inventory.use(tonic.itemId)).toBe(true);
      const used = yield* harness.inventory.get(tonic.itemId);
      expect(used?.equipped).toBe(false);
      expect(used?.quantity).toBe(1);
      expect(harness.equippedItemIds).toEqual([]);
      expect(harness.usedItemIds).toEqual([tonic.itemId]);
    }),
  );

  it.effect("uses server-use boosts", () =>
    Effect.gen(function* () {
      const boost = consumable(103, "none", false, "ServerUse");
      const harness = yield* makeHarness([boost]);

      expect(yield* harness.inventory.use(boost.itemId)).toBe(true);
      expect((yield* harness.inventory.get(boost.itemId))?.quantity).toBe(1);
      expect(harness.usedItemIds).toEqual([boost.itemId]);
    }),
  );
});
