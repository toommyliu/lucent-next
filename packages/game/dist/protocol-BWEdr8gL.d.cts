type ServerData = {
  chat: number;
  count: number;
  language: string;
  max: number;
  memberOnly: boolean;
  name: string;
  online: boolean;
};
type ServerSnapshot = {
  full: boolean;
  chat: number;
  count: number;
  language: string;
  max: number;
  memberOnly: boolean;
  name: string;
  online: boolean;
};
type HydrationMethod = "player.getUserId" | "player.getData" | "world.isLoaded" | "player.getCell" | "player.getPad";
type HydrationRequest = {
  method: HydrationMethod;
  args: Array<unknown>;
};
type HydrationResult = {
  "status": "unavailable";
} | {
  "status": "value";
  "value": unknown;
};
type ReadinessKey = "houseInventory" | "inventory" | "map" | "player";
type ReadinessFlags = {
  houseInventory: boolean;
  inventory: boolean;
  map: boolean;
  player: boolean;
};
type ReadinessSnapshot = {
  epoch: number;
  failures: { [key in ReadinessKey]?: string; };
  missing: Array<ReadinessKey>;
  state: ReadinessFlags;
};
type PacketDirection = "client" | "server" | "extension";
type PacketEncoding = "string" | "json";
type Packet = {
  "direction": "client";
  command: string;
  params: ReadonlyArray<string>;
  raw: string;
  encoding: PacketEncoding;
} | {
  "direction": "server";
  command: string;
  data: unknown;
  raw: string;
  encoding: PacketEncoding;
} | {
  "direction": "extension";
  command: string;
  data: unknown;
  raw: string;
  encoding: PacketEncoding;
};
type ItemContainer = "bank" | "drop" | "house" | "inventory" | "shop" | "temporary";
type ItemContext = "bank" | "drop" | "house" | "inventory" | "monster-drop" | "shop" | "temporary";
type Enhancement = {
  dps?: number;
  id?: number;
  level?: number;
  patternId?: number;
  procId?: number;
  range?: number;
  rarity?: number;
};
type ItemRequirement = {
  itemId: number;
  name: string;
  quantity: number;
};
type ItemData = {
  category: string;
  charItemId?: number;
  coins: boolean;
  context: ItemContext;
  cost: number;
  description: string;
  enhancement?: Enhancement;
  equipped: boolean;
  equipmentSlot: string;
  file: string;
  houseItem: boolean;
  itemId: number;
  link: string;
  maxStack?: number;
  memberOnly: boolean;
  meta: string;
  name: string;
  quantity: number;
  requirements?: Array<ItemRequirement>;
  shopItemId?: number;
  temporaryItem: boolean;
  wearable: boolean;
  worn: boolean;
};
type EnhancementSnapshot = {
  name?: string;
  special?: string;
  dps?: number;
  id?: number;
  level?: number;
  patternId?: number;
  procId?: number;
  range?: number;
  rarity?: number;
};
type ItemPool = "bag" | "misc" | "class" | "house";
type ItemSnapshotFields = {
  armor: boolean;
  banked: boolean;
  cape: boolean;
  classItem: boolean;
  classRank: number | null;
  enhancement?: EnhancementSnapshot;
  helm: boolean;
  pet: boolean;
  pool: ItemPool;
  weapon: boolean;
};
type ItemSnapshot = Omit<ItemData, "enhancement"> & ItemSnapshotFields;
type MapState = {
  id: number;
  name: string;
  roomNumber: number;
};
type Point = {
  x: number;
  y: number;
};
type EntityKind = "player" | "monster";
type AuraKind = "active" | "passive";
type CounterSource = "aura" | "message";
type MessageSource = "animation" | "aura";
type LocationChange = {
  "kind": "cell";
} | {
  "kind": "position";
} | {
  "kind": "walk";
  destination: Point;
};
type AuraEvent = {
  duration?: number;
  icon?: string;
  kind: AuraKind;
  name: string;
  sourceId?: number;
  sourceType?: EntityKind;
  targetId: number;
  targetType: EntityKind;
};
type ProjectionEvent = {
  "type": "login";
} | {
  "type": "logout";
} | {
  "type": "player-afk";
  afk: boolean;
  entityId: number;
  username: string;
} | {
  "type": "join-map";
  map: MapState;
} | {
  "type": "item-drop";
  item: ItemSnapshot;
} | {
  "type": "quest-complete";
  questId: number;
} | {
  "type": "monster-death";
  monsterMapId: number;
} | {
  "type": "monster-respawn";
  monsterMapId: number;
} | {
  "type": "player-death";
  entityId: number;
  username: string;
} | {
  "type": "players-changed";
} | {
  "type": "aura-added";
  stack: number;
  duration?: number;
  icon?: string;
  kind: AuraKind;
  name: string;
  sourceId?: number;
  sourceType?: EntityKind;
  targetId: number;
  targetType: EntityKind;
} | {
  "type": "aura-updated";
  stack: number;
  duration?: number;
  icon?: string;
  kind: AuraKind;
  name: string;
  sourceId?: number;
  sourceType?: EntityKind;
  targetId: number;
  targetType: EntityKind;
} | {
  "type": "aura-removed";
  duration?: number;
  icon?: string;
  kind: AuraKind;
  name: string;
  sourceId?: number;
  sourceType?: EntityKind;
  targetId: number;
  targetType: EntityKind;
} | {
  "type": "counter-attack-start";
  durationMs?: number;
  monsterMapId: number;
  source: CounterSource;
  triggerId: string;
  triggerText: string;
} | {
  "type": "counter-attack-end";
  monsterMapId: number;
  source: CounterSource;
  triggerId: string;
  triggerText: string;
} | {
  "type": "player-location";
  cell: string;
  entityId: number;
  pad: string;
  position: Point;
  username: string;
} & ({
  "kind": "cell";
} | {
  "kind": "position";
} | {
  "kind": "walk";
  destination: Point;
}) | {
  "type": "update-message";
  animation?: string;
  message: string;
  monsterMapId?: number;
  source: MessageSource;
} | {
  "type": "zone";
  map: string;
  zone: string;
};
type AuraValue = number | string;
type AuraData = {
  category?: string;
  duration: number;
  expiresAt?: number;
  icon?: string;
  kind: AuraKind;
  name: string;
  persistent: boolean;
  stack: number;
  value?: AuraValue;
};
type EntityState = typeof EntityState[keyof typeof EntityState];
type EntityData = {
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
};
type EntitySnapshotFields = {
  alive: boolean;
  auras: Array<AuraData>;
  dead: boolean;
  hpPercent: number;
  idle: boolean;
  inCombat: boolean;
  mpPercent: number;
};
type EntitySnapshot = {
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
  alive: boolean;
  auras: Array<AuraData>;
  dead: boolean;
  hpPercent: number;
  idle: boolean;
  inCombat: boolean;
  mpPercent: number;
};
type PlayerData = {
  afk: boolean;
  entityId: number;
  entityType: string;
  level: number;
  pad: string;
  position: Point;
  username: string;
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
};
type PlayerSnapshot = {
  afk: boolean;
  entityId: number;
  entityType: string;
  level: number;
  pad: string;
  position: Point;
  username: string;
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
  alive: boolean;
  auras: Array<AuraData>;
  dead: boolean;
  hpPercent: number;
  idle: boolean;
  inCombat: boolean;
  mpPercent: number;
};
type MonsterData = {
  aggressive: boolean;
  level: number;
  monsterId: number;
  monsterMapId: number;
  name: string;
  race: string;
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
};
type MonsterSnapshot = {
  drops: Array<MonsterDrop>;
  aggressive: boolean;
  level: number;
  monsterId: number;
  monsterMapId: number;
  name: string;
  race: string;
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
  alive: boolean;
  auras: Array<AuraData>;
  dead: boolean;
  hpPercent: number;
  idle: boolean;
  inCombat: boolean;
  mpPercent: number;
};
type QuestCadence = "daily" | "monthly" | "none" | "weekly";
type QuestItem = {
  itemId: number;
  name: string;
  quantity: number;
  maxStack?: number;
  temporaryItem?: boolean;
};
type QuestReward = {
  dropChance?: number;
  itemId: number;
  name: string;
  quantity: number;
  maxStack?: number;
  temporaryItem?: boolean;
};
type QuestData = {
  cadence: QuestCadence;
  id: number;
  name: string;
  once: boolean;
  requirements: Array<QuestItem>;
  rewards: Array<QuestReward>;
};
type ShopData = {
  house: boolean;
  id: number;
  items: Array<ItemSnapshot>;
  limited: boolean;
  merge: boolean;
  name: string;
};
type MonsterDrop = {
  eventDrop: boolean;
  icon: string;
  item: ItemSnapshot;
  questGated: boolean;
  questObjectives: Array<string>;
  rarity: number;
  rarityName: string;
  rateBoostPercent: number | null;
  ratePercent: number | null;
  requiredQuestIds: Array<number>;
  requiredQuests: Array<string>;
  stackSize: number;
  variableQuantity: boolean;
};
declare const EntityState: {
  readonly Dead: 0;
  readonly Idle: 1;
  readonly InCombat: 2;
};
export { PacketDirection as A, ReadinessFlags as B, LocationChange as C, MonsterDrop as D, MonsterData as E, ProjectionEvent as F, ShopData as G, ReadinessSnapshot as H, QuestCadence as I, QuestData as L, PlayerData as M, PlayerSnapshot as N, MonsterSnapshot as O, Point as P, QuestItem as R, ItemSnapshotFields as S, MessageSource as T, ServerData as U, ReadinessKey as V, ServerSnapshot as W, ItemContext as _, CounterSource as a, ItemRequirement as b, EntityData as c, EntitySnapshotFields as d, EntityState as f, ItemContainer as g, HydrationResult as h, AuraValue as i, PacketEncoding as j, Packet as k, EntityKind as l, HydrationRequest as m, AuraEvent as n, Enhancement as o, HydrationMethod as p, AuraKind as r, EnhancementSnapshot as s, AuraData as t, EntitySnapshot as u, ItemData as v, MapState as w, ItemSnapshot as x, ItemPool as y, QuestReward as z };