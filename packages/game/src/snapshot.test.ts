import { describe, expect, it } from "vitest";

import { toLiveAura, type AuraSnapshot } from "./aura";
import { toLiveItem, type ItemSnapshot } from "./item";
import { toLiveMonster, type MonsterSnapshot } from "./monster";
import { toLivePlayer, type PlayerSnapshot } from "./player";
import { toLiveQuest, type QuestSnapshot } from "./quest";
import { toLiveServer, type ServerSnapshot } from "./server";
import { toLiveShop, type ShopSnapshot } from "./shop";

const aura: AuraSnapshot = {
  category: "buff",
  duration: 8,
  expiresAt: 1_008_000,
  icon: "iwd1",
  kind: "active",
  name: "Focus",
  persistent: false,
  stack: 2,
  value: 0.5,
};

const sword: ItemSnapshot = {
  armor: false,
  banked: true,
  cape: false,
  category: "Sword",
  charItemId: 900,
  classItem: false,
  classRank: null,
  coins: false,
  context: "bank",
  cost: 1000,
  description: "A sword",
  enhancement: {
    level: 100,
    name: "Fighter",
    patternId: 2,
    procId: 14,
    special: "Dauntless",
  },
  equipmentSlot: "Weapon",
  equipped: false,
  file: "items/swords/sword.swf",
  helm: false,
  houseItem: false,
  itemId: 1,
  link: "Sword",
  maxStack: 1,
  memberOnly: false,
  meta: "",
  name: "Sword",
  pet: false,
  pool: "bag",
  quantity: 1,
  temporaryItem: false,
  wearable: true,
  weapon: true,
  worn: false,
};

const shopItem: ItemSnapshot = {
  armor: false,
  banked: false,
  cape: false,
  category: "Resource",
  classItem: false,
  classRank: null,
  coins: true,
  context: "shop",
  cost: 25,
  description: "",
  equipmentSlot: "",
  equipped: false,
  file: "",
  helm: false,
  houseItem: false,
  itemId: 30,
  link: "",
  memberOnly: false,
  meta: "",
  name: "Ore",
  pet: false,
  pool: "misc",
  quantity: 5,
  requirements: [{ itemId: 31, name: "Rock", quantity: 2 }],
  shopItemId: 300,
  temporaryItem: false,
  wearable: false,
  weapon: false,
  worn: false,
};

describe("Live models from host snapshots", () => {
  it("rebuilds an aura", () => {
    expect(toLiveAura(aura).toJSON()).toStrictEqual(aura);
  });

  it("rebuilds an item and derives its enhancement names", () => {
    expect(toLiveItem(sword).toJSON()).toStrictEqual(sword);
  });

  it("rebuilds a player with its auras", () => {
    const player: PlayerSnapshot = {
      afk: false,
      alive: true,
      auras: [aura],
      cell: "Enter",
      dead: false,
      entityId: 7,
      entityType: "player",
      hp: 500,
      hpPercent: 50,
      idle: true,
      inCombat: false,
      level: 100,
      maxHp: 1000,
      maxMp: 100,
      mp: 50,
      mpPercent: 50,
      pad: "Spawn",
      position: { x: 10, y: 20 },
      state: 1,
      username: "hero",
    };
    expect(toLivePlayer(player).toJSON()).toStrictEqual(player);
  });

  it("rebuilds a monster with its auras and drops", () => {
    const monster: MonsterSnapshot = {
      aggressive: true,
      alive: true,
      auras: [aura],
      cell: "r2",
      dead: false,
      drops: [
        {
          eventDrop: false,
          icon: "iiSword",
          item: { ...sword, banked: false, context: "monster-drop" },
          questGated: true,
          questObjectives: ["Slay frogs"],
          rarity: 30,
          rarityName: "Rare",
          rateBoostPercent: null,
          ratePercent: 5,
          requiredQuestIds: [12],
          requiredQuests: ["Frog Hunt"],
          stackSize: 1,
          variableQuantity: false,
        },
      ],
      hp: 25,
      hpPercent: 50,
      idle: false,
      inCombat: true,
      level: 3,
      maxHp: 50,
      maxMp: 0,
      monsterId: 1,
      monsterMapId: 2,
      mp: 0,
      mpPercent: 0,
      name: "Frogzard",
      race: "Amphibian",
      state: 2,
    };
    expect(toLiveMonster(monster).toJSON()).toStrictEqual(monster);
  });

  it("rebuilds a quest", () => {
    const quest: QuestSnapshot = {
      cadence: "daily",
      id: 4,
      name: "Frog Hunt",
      once: false,
      requirements: [{ itemId: 31, name: "Frog Leg", quantity: 5 }],
      rewards: [{ dropChance: 50, itemId: 1, name: "Sword", quantity: 1 }],
    };
    expect(toLiveQuest(quest).toJSON()).toStrictEqual(quest);
  });

  it("rebuilds a shop with its items", () => {
    const shop: ShopSnapshot = {
      house: false,
      id: 5,
      items: [shopItem],
      limited: false,
      merge: true,
      name: "Forge",
    };
    expect(toLiveShop(shop).toJSON()).toStrictEqual(shop);
  });

  it("rebuilds a server and derives whether it is full", () => {
    const server: ServerSnapshot = {
      chat: 2,
      count: 10,
      full: true,
      language: "en",
      max: 10,
      memberOnly: false,
      name: "Artix",
      online: true,
    };
    expect(toLiveServer(server).toJSON()).toStrictEqual(server);
  });
});
