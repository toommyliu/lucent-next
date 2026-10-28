import { D as MonsterDrop$1, E as MonsterData$1, G as ShopData$1, I as QuestCadence$1, L as QuestData$1, M as PlayerData$1, N as PlayerSnapshot$1, O as MonsterSnapshot$1, P as Point, R as QuestItem$1, U as ServerData$1, W as ServerSnapshot$1, _ as ItemContext$1, b as ItemRequirement$1, c as EntityData$1, f as EntityState, o as Enhancement$1, r as AuraKind$1, s as EnhancementSnapshot, t as AuraData$1, u as EntitySnapshot$1, v as ItemData$1, x as ItemSnapshot$1, y as ItemPool$1, z as QuestReward$1 } from "./protocol-BWEdr8gL.cjs";
type DeepReadonly<T> = T extends object ? { readonly [Key in keyof T]: DeepReadonly<T[Key]>; } : T;
declare abstract class LiveModel<State extends object> {
  #private;
  constructor(data: State);
  protected get modelData(): State;
  /** @internal */
  update(patch: Partial<State>): void;
  /** @internal */
  replaceFrom(model: LiveModel<State>): void;
  /** @internal */
  snapshot(): State;
}
type AuraKind = AuraKind$1;
interface AuraQueryOptions {
  readonly kind?: AuraKind;
}
interface Aura {
  readonly category: string | undefined;
  readonly duration: number;
  readonly expiresAt: number | undefined;
  readonly icon: string | undefined;
  readonly kind: AuraKind;
  readonly name: string;
  readonly persistent: boolean;
  readonly stack: number;
  readonly value: number | string | undefined;
  toJSON(): AuraSnapshot;
}
type AuraData = AuraData$1;
type AuraSnapshot = Readonly<AuraData>;
declare class LiveAura extends LiveModel<AuraData> implements Aura {
  get category(): string | undefined;
  /** Full duration in seconds, or `0` when the aura has no timer. */
  get duration(): number;
  /** Expiry time in epoch milliseconds, or `undefined` when the aura has no timer. The aura stays until the server removes it, even after this time. */
  get expiresAt(): number | undefined;
  get icon(): string | undefined;
  get kind(): AuraKind;
  get name(): string;
  get persistent(): boolean;
  get stack(): number;
  get value(): number | string | undefined;
  toJSON(): AuraSnapshot;
}
/** @internal */
type AuraTiming = {
  readonly type: "untimed";
} | {
  readonly type: "seconds";
  readonly duration: number;
};
/** @internal */
interface AuraApplication {
  readonly name: string;
  readonly stack: number;
  readonly timing: AuraTiming;
  readonly persistent: boolean;
  readonly restartDuration: boolean;
  readonly icon?: string;
  readonly category?: string;
  readonly value?: number | string;
}
/** @internal */
interface AuraSeed {
  readonly name: string;
  readonly icon: string;
  readonly stack: number;
  readonly persistent: boolean;
  readonly timer: {
    readonly type: "untimed";
  } | {
    readonly type: "timed";
    readonly remainingSeconds: number;
    readonly fullSeconds: number;
  };
}
/** @internal */
type AuraWithdrawal = {
  readonly type: "remove";
  readonly name: string;
} | {
  readonly type: "decay";
  readonly name: string;
  readonly stack: number;
  readonly refreshSeconds?: number;
};
/** @internal */
interface PassiveAuraInput {
  readonly name: string;
  readonly category?: string;
  readonly icon?: string;
  readonly value?: number | string;
  readonly duration: number;
}
/** @internal */
type AuraMutation = {
  readonly type: "seed";
  readonly entries: readonly AuraSeed[];
} | {
  readonly type: "apply";
  readonly entries: readonly AuraApplication[];
} | {
  readonly type: "withdraw";
  readonly entries: readonly AuraWithdrawal[];
} | {
  readonly type: "set-stack";
  readonly entries: readonly {
    readonly name: string;
    readonly stack: number;
  }[];
} | {
  readonly type: "passives";
  readonly mode: "replace" | "merge";
  readonly entries: readonly PassiveAuraInput[];
} | {
  readonly type: "clear";
  readonly keepPersistent: boolean;
};
/** @internal */
type AuraDelta = {
  readonly type: "added";
  readonly after: AuraSnapshot;
} | {
  readonly type: "updated";
  readonly before: AuraSnapshot;
  readonly after: AuraSnapshot;
} | {
  readonly type: "removed";
  readonly before: AuraSnapshot;
};
/** @internal */
type ActiveAuraEdit = {
  readonly type: "seed";
  readonly entry: AuraSeed;
} | {
  readonly type: "apply";
  readonly entry: AuraApplication;
} | AuraWithdrawal | {
  readonly type: "set-stack";
  readonly name: string;
  readonly stack: number;
};
/** @internal */
declare function reduceActiveAura(before: AuraSnapshot | undefined, edit: ActiveAuraEdit, nowMs: number): AuraSnapshot | undefined;
type ItemContext = ItemContext$1;
/**
 * `"bag"` counts equipment and equippable consumables such as potions.
 * `"misc"` counts resources, quest items, notes, and other non-consumable items
 * of type Item.
 * Classes, house items, and Guild items count against neither. A misc item
 * already in the inventory never takes a new misc slot.
 */
type InventoryPool = Extract<ItemPool$1, "bag" | "misc">;
/**
 * The pool an item counts against in the inventory. `"class"` has no limit,
 * and `"house"` uses house slots.
 */
type ItemPool = ItemPool$1;
declare const getItemRarityName: (rarity: number) => string;
/** Returns the class rank for a cumulative class-point total. */
declare const getClassRankFromPoints: (classPoints: number) => number;
type Enhancement = Readonly<Enhancement$1> & {
  /** Enhancement label, such as `Fighter` or `Vim`. */
  readonly name?: EnhancementSnapshot["name"];
  /** Special label, such as `Dauntless` or `Ether`. */
  readonly special?: EnhancementSnapshot["special"];
};
interface ItemSelectorById {
  readonly itemId: number;
  readonly name?: never;
}
interface ItemSelectorByName {
  readonly itemId?: never;
  readonly name: string;
}
type ItemSelector = ItemSelectorById | ItemSelectorByName;
interface ShopItemSelectorById {
  readonly itemId?: never;
  readonly name?: never;
  readonly shopItemId: number;
}
type ShopItemSelector = ItemSelector | ShopItemSelectorById;
type ItemQuery = ItemSelector | number | string;
type ShopItemQuery = ShopItemSelector | number | string;
declare const toItemSelector: (query: ItemQuery) => ItemSelector;
declare const normalizeItemQuantity: (value: number | undefined) => number;
type ItemRequirement = Readonly<ItemRequirement$1>;
interface Item {
  readonly armor: boolean;
  readonly banked: boolean;
  readonly cape: boolean;
  readonly category: string;
  readonly charItemId: number | undefined;
  readonly classItem: boolean;
  /** Rank derived from class points, or `null` for a non-class item. */
  readonly classRank: number | null;
  readonly coins: boolean;
  readonly context: ItemContext;
  readonly cost: number;
  readonly description: string;
  readonly enhancement: Enhancement | undefined;
  readonly equipped: boolean;
  readonly equipmentSlot: string;
  readonly file: string;
  readonly helm: boolean;
  readonly houseItem: boolean;
  readonly itemId: number;
  readonly link: string;
  readonly maxStack: number | undefined;
  readonly memberOnly: boolean;
  readonly meta: string;
  readonly name: string;
  readonly pet: boolean;
  readonly pool: ItemPool;
  readonly quantity: number;
  /** Items consumed per merge for this shop offer. */
  readonly requirements: readonly ItemRequirement[];
  readonly shopItemId: number | undefined;
  readonly temporaryItem: boolean;
  readonly wearable: boolean;
  readonly weapon: boolean;
  readonly worn: boolean;
  matches(selector: ItemQuery | ShopItemQuery): boolean;
  toJSON(): ItemSnapshot;
}
type ItemData = Omit<ItemData$1, "enhancement" | "requirements" | "wearable" | "worn"> & Partial<Pick<ItemData$1, "wearable" | "worn">> & {
  enhancement?: Enhancement;
  requirements?: readonly ItemRequirement[];
};
type ItemSnapshot = DeepReadonly<ItemSnapshot$1>;
declare class LiveItem extends LiveModel<ItemData> implements Item {
  get armor(): boolean;
  get banked(): boolean;
  get cape(): boolean;
  get category(): string;
  get charItemId(): number | undefined;
  get classItem(): boolean;
  get classRank(): number | null;
  get coins(): boolean;
  get context(): ItemContext;
  get cost(): number;
  get description(): string;
  get enhancement(): Enhancement | undefined;
  get equipped(): boolean;
  get equipmentSlot(): string;
  get file(): string;
  get helm(): boolean;
  get houseItem(): boolean;
  get itemId(): number;
  get link(): string;
  get maxStack(): number | undefined;
  get memberOnly(): boolean;
  get meta(): string;
  get name(): string;
  get pet(): boolean;
  get pool(): ItemPool;
  get quantity(): number;
  get requirements(): readonly ItemRequirement[];
  get shopItemId(): number | undefined;
  get temporaryItem(): boolean;
  get weapon(): boolean;
  get wearable(): boolean;
  get worn(): boolean;
  matches(selector: ItemQuery | ShopItemQuery): boolean;
  toJSON(): ItemSnapshot;
}
type EnhancementSlot = "cape" | "class" | "helm" | "weapon";
interface EnhancementStrategy {
  readonly map?: "forge" | "museum";
  readonly patternId: number;
  readonly procId: number;
  readonly shopId: number;
  readonly slot: EnhancementSlot;
}
type EnhancementStrategyResolution = {
  readonly ok: true;
  readonly strategy: EnhancementStrategy;
} | {
  readonly ok: false;
  readonly reason: string;
};
declare const getItemEnhancementNames: (enhancement: Pick<Enhancement, "patternId" | "procId">) => Pick<Enhancement, "name" | "special">;
declare const formatItemEnhancement: (enhancement: Pick<Enhancement, "level" | "patternId" | "procId"> | undefined) => string | undefined;
declare const resolveEnhancementStrategy: (item: Pick<Item, "cape" | "classItem" | "helm" | "weapon">, enhancement: string, playerLevel: number, special?: string) => EnhancementStrategyResolution;
declare const matchesEnhancementShopItem: (item: Pick<Item, "category" | "enhancement" | "equipmentSlot">, strategy: EnhancementStrategy) => boolean;
declare const matchesAppliedEnhancement: (item: Pick<Item, "enhancement">, strategy: Pick<EnhancementStrategy, "patternId" | "procId">) => boolean;
declare const isEntityState: (value: unknown) => value is EntityState;
interface Entity {
  readonly alive: boolean;
  readonly auras: readonly Aura[];
  readonly cell: string;
  readonly dead: boolean;
  readonly hp: number;
  readonly hpPercent: number;
  readonly idle: boolean;
  readonly inCombat: boolean;
  readonly maxHp: number;
  readonly maxMp: number;
  readonly mp: number;
  readonly mpPercent: number;
  readonly state: EntityState;
  getAura(name: string, options?: AuraQueryOptions): Aura | null;
  hasAura(name: string, options?: AuraQueryOptions): boolean;
  isInCell(cell: string): boolean;
  toJSON(): EntitySnapshot;
}
type EntityData = EntityData$1;
type EntitySnapshot = DeepReadonly<EntitySnapshot$1>;
declare abstract class LiveEntity<State extends EntityData> extends LiveModel<State> implements Entity {
  #private;
  get alive(): boolean;
  get auras(): readonly LiveAura[];
  get cell(): string;
  get dead(): boolean;
  get hp(): number;
  get hpPercent(): number;
  get idle(): boolean;
  get inCombat(): boolean;
  get maxHp(): number;
  get maxMp(): number;
  get mp(): number;
  get mpPercent(): number;
  get state(): EntityState;
  getAura(name: string, options?: AuraQueryOptions): LiveAura | null;
  hasAura(name: string, options?: AuraQueryOptions): boolean;
  /** @internal */
  projectAuras(mutation: AuraMutation, nowMs: number): readonly AuraDelta[];
  /** @internal */
  writeAuraMonsterState(state: EntityState): readonly AuraDelta[];
  private auraKey;
  private commit;
  isInCell(cell: string): boolean;
  abstract toJSON(): EntitySnapshot;
}
interface Faction {
  readonly id: number;
  readonly name: string;
  readonly rank: number;
  readonly reputation: number;
  toJSON(): FactionSnapshot;
}
interface FactionData {
  id: number;
  name: string;
  rank: number;
  reputation: number;
}
type FactionSnapshot = Readonly<FactionData>;
declare class LiveFaction extends LiveModel<FactionData> implements Faction {
  get id(): number;
  get name(): string;
  get rank(): number;
  get reputation(): number;
  toJSON(): FactionSnapshot;
}
/** An item and its acquisition metadata from AQW's monster-drop response. */
type MonsterDrop = DeepReadonly<MonsterDrop$1>;
interface MonsterSelectorByMapId {
  readonly monsterMapId: number;
  readonly name?: never;
}
interface MonsterSelectorByName {
  readonly monsterMapId?: never;
  readonly name: string;
}
type MonsterSelector = MonsterSelectorByMapId | MonsterSelectorByName;
/**
 * Selects a monster by name, map ID, or selector object.
 *
 * String map IDs require `id` followed by `.`, `:`, `-`, or `'`:
 * `id.123`, `id:123`, `id-123`, or `id'123`.
 */
type MonsterQuery = MonsterSelector | number | string;
declare const parseMonsterMapId: (value: string) => number | undefined;
declare const toMonsterSelector: (query: MonsterQuery) => MonsterSelector;
interface Monster extends Entity {
  readonly aggressive: boolean;
  /** Server-provided drops discovered while sharing a cell with this monster. */
  readonly drops: readonly MonsterDrop[];
  readonly level: number;
  readonly monsterId: number;
  readonly monsterMapId: number;
  readonly name: string;
  readonly race: string;
  matches(selector: MonsterQuery): boolean;
}
declare const orderMonstersByPriority: (monsters: readonly Monster[], priorities: readonly MonsterQuery[]) => readonly Monster[];
type MonsterData = MonsterData$1;
type MonsterSnapshot = DeepReadonly<MonsterSnapshot$1>;
declare class LiveMonster extends LiveEntity<MonsterData> implements Monster {
  #private;
  get aggressive(): boolean;
  get drops(): readonly MonsterDrop[];
  get level(): number;
  get monsterId(): number;
  get monsterMapId(): number;
  get name(): string;
  get race(): string;
  matches(selector: MonsterQuery): boolean;
  /**
   * Replaces the server-provided drop collection without replacing the monster.
   * @internal
   */
  replaceDrops(drops: readonly MonsterDrop[]): void;
  toJSON(): MonsterSnapshot;
}
interface OutfitColors {
  readonly accessory: number | undefined;
  readonly base: number | undefined;
  readonly eye: number | undefined;
  readonly hair: number | undefined;
  readonly skin: number | undefined;
  readonly trim: number | undefined;
}
interface OutfitEquipment {
  readonly armorItemId: number | undefined;
  readonly capeItemId: number | undefined;
  readonly classItemId: number | undefined;
  readonly helmItemId: number | undefined;
  readonly itemId: number | undefined;
  readonly miscItemId: number | undefined;
  readonly petItemId: number | undefined;
  readonly weaponItemId: number | undefined;
}
interface Outfit {
  readonly colors: OutfitColors;
  readonly equipment: OutfitEquipment;
  readonly name: string;
  toJSON(): OutfitSnapshot;
}
interface OutfitData {
  colors: OutfitColors;
  equipment: OutfitEquipment;
  name: string;
}
type OutfitSnapshot = Readonly<OutfitData>;
declare class LiveOutfit extends LiveModel<OutfitData> implements Outfit {
  get colors(): OutfitColors;
  get equipment(): OutfitEquipment;
  get name(): string;
  toJSON(): OutfitSnapshot;
}
interface PlayerSelectorByUsername {
  readonly username: string;
}
type PlayerSelector = PlayerSelectorByUsername;
type PlayerQuery = PlayerSelector | string;
declare const toPlayerSelector: (query: PlayerQuery) => PlayerSelector;
type BoostType = "classPoints" | "exp" | "gold" | "rep";
type Position = Readonly<Point>;
interface Player extends Entity {
  readonly afk: boolean;
  readonly entityId: number;
  readonly entityType: string;
  readonly level: number;
  readonly pad: string;
  readonly position: Position;
  readonly username: string;
}
type PlayerData = PlayerData$1;
type PlayerSnapshot = DeepReadonly<PlayerSnapshot$1>;
declare class LivePlayer extends LiveEntity<PlayerData> implements Player {
  get afk(): boolean;
  get entityId(): number;
  get entityType(): string;
  get level(): number;
  get pad(): string;
  get position(): Position;
  get username(): string;
  toJSON(): PlayerSnapshot;
}
type QuestItem = Readonly<QuestItem$1>;
type QuestReward = Readonly<QuestReward$1>;
type QuestCadence = QuestCadence$1;
interface Quest {
  readonly cadence: QuestCadence;
  readonly id: number;
  readonly name: string;
  readonly once: boolean;
  readonly requirements: readonly QuestItem[];
  readonly rewards: readonly QuestReward[];
  toJSON(): QuestSnapshot;
}
type QuestData = Omit<QuestData$1, "requirements" | "rewards"> & {
  requirements: readonly QuestItem[];
  rewards: readonly QuestReward[];
};
type QuestSnapshot = DeepReadonly<QuestData$1>;
declare class LiveQuest extends LiveModel<QuestData> implements Quest {
  /**
   * How often the quest can be completed.
   */
  get cadence(): QuestCadence;
  get id(): number;
  get name(): string;
  get once(): boolean;
  get requirements(): readonly QuestItem[];
  get rewards(): readonly QuestReward[];
  toJSON(): QuestSnapshot;
}
interface Server {
  readonly chat: number;
  readonly count: number;
  readonly full: boolean;
  readonly language: string;
  readonly max: number;
  readonly memberOnly: boolean;
  readonly name: string;
  readonly online: boolean;
  toJSON(): ServerSnapshot;
}
type ServerData = ServerData$1;
type ServerSnapshot = Readonly<ServerSnapshot$1>;
declare class LiveServer extends LiveModel<ServerData> implements Server {
  get chat(): number;
  get count(): number;
  get full(): boolean;
  get language(): string;
  get max(): number;
  get memberOnly(): boolean;
  get name(): string;
  get online(): boolean;
  toJSON(): ServerSnapshot;
}
interface Shop {
  readonly house: boolean;
  readonly id: number;
  readonly items: readonly Item[];
  readonly limited: boolean;
  readonly merge: boolean;
  readonly name: string;
  toJSON(): ShopSnapshot;
}
type ShopData = Omit<ShopData$1, "items"> & {
  items: readonly Item[];
};
type ShopSnapshot = DeepReadonly<ShopData$1>;
declare class LiveShop extends LiveModel<ShopData> implements Shop {
  get house(): boolean;
  get id(): number;
  get items(): readonly Item[];
  get limited(): boolean;
  get merge(): boolean;
  get name(): string;
  toJSON(): ShopSnapshot;
}
export { ActiveAuraEdit, Aura, AuraApplication, AuraData, AuraDelta, AuraKind, AuraMutation, AuraQueryOptions, AuraSeed, AuraSnapshot, AuraTiming, AuraWithdrawal, BoostType, type DeepReadonly, Enhancement, EnhancementSlot, EnhancementStrategy, EnhancementStrategyResolution, Entity, EntityData, EntitySnapshot, EntityState, Faction, FactionData, FactionSnapshot, InventoryPool, Item, ItemContext, ItemData, ItemPool, ItemQuery, ItemRequirement, ItemSelector, ItemSelectorById, ItemSelectorByName, ItemSnapshot, LiveAura, LiveEntity, LiveFaction, LiveItem, LiveMonster, LiveOutfit, LivePlayer, LiveQuest, LiveServer, LiveShop, Monster, MonsterData, MonsterDrop, MonsterQuery, MonsterSelector, MonsterSelectorByMapId, MonsterSelectorByName, MonsterSnapshot, Outfit, OutfitColors, OutfitData, OutfitEquipment, OutfitSnapshot, PassiveAuraInput, Player, PlayerData, PlayerQuery, PlayerSelector, PlayerSelectorByUsername, PlayerSnapshot, Position, Quest, QuestCadence, QuestData, QuestItem, QuestReward, QuestSnapshot, Server, ServerData, ServerSnapshot, Shop, ShopData, ShopItemQuery, ShopItemSelector, ShopItemSelectorById, ShopSnapshot, formatItemEnhancement, getClassRankFromPoints, getItemEnhancementNames, getItemRarityName, isEntityState, matchesAppliedEnhancement, matchesEnhancementShopItem, normalizeItemQuantity, orderMonstersByPriority, parseMonsterMapId, reduceActiveAura, resolveEnhancementStrategy, toItemSelector, toMonsterSelector, toPlayerSelector };