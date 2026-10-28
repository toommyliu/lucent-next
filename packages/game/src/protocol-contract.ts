import type * as Protocol from "./protocol";
import type { DeepReadonly } from "./model";
import type { AuraKind, AuraData } from "./aura";
import type { EntityData, EntitySnapshot } from "./entity";
import type {
  Item,
  ItemContext,
  InventoryPool,
  ItemPool,
  Enhancement,
  ItemRequirement,
  ItemData,
  ItemSnapshot,
} from "./item";
import type { MonsterDrop, MonsterData, MonsterSnapshot } from "./monster";
import type { Position, PlayerData, PlayerSnapshot } from "./player";
import type {
  QuestItem,
  QuestReward,
  QuestCadence,
  QuestData,
  QuestSnapshot,
} from "./quest";
import type { ServerData, ServerSnapshot } from "./server";
import type { ShopData, ShopSnapshot } from "./shop";

type Identical<A, B> =
  (<T>() => T extends A ? 1 : 2) extends
  (<T>() => T extends B ? 1 : 2)
    ? true
    : false;

type ReadonlyKeys<T> = {
  [Key in keyof T]-?: Identical<Pick<T, Key>, Readonly<Pick<T, Key>>> extends true
    ? Key
    : never;
}[keyof T];

declare const shape: unique symbol;

type Normalize<T> = 0 extends (1 & T)
  ? { [shape]: "any" }
  : T extends (...args: infer Args) => infer Result
    ? { [shape]: { parameters: Normalize<Args>; result: Normalize<Result> } }
    : T extends object
      ? { [Key in keyof T]: Normalize<T[Key]> } & {
          [shape]: { keys: keyof T; readonly: ReadonlyKeys<T> };
        }
      : T;

export type AssertEqual<A, B> =
  [Normalize<A>] extends [Normalize<B>]
    ? [Normalize<B>] extends [Normalize<A>]
      ? true
      : false
    : false;

type Assert<T extends true> = T;

export type ProtocolContracts = [
  Assert<AssertEqual<AuraKind, Protocol.AuraKind>>,
  Assert<AssertEqual<AuraData, Protocol.AuraData>>,
  Assert<AssertEqual<EntityData, Protocol.EntityData>>,
  Assert<AssertEqual<EntitySnapshot, DeepReadonly<Protocol.EntitySnapshot>>>,
  Assert<AssertEqual<ItemContext, Protocol.ItemContext>>,
  Assert<AssertEqual<InventoryPool, Extract<Protocol.ItemPool, "bag" | "misc">>>,
  Assert<AssertEqual<ItemPool, Protocol.ItemPool>>,
  Assert<AssertEqual<
    Enhancement,
    Readonly<Protocol.Enhancement> & {
      readonly name?: Protocol.EnhancementSnapshot["name"];
      readonly special?: Protocol.EnhancementSnapshot["special"];
    }
  >>,
  Assert<AssertEqual<ItemRequirement, Readonly<Protocol.ItemRequirement>>>,
  Assert<AssertEqual<
    ItemData,
    Omit<Protocol.ItemData, "enhancement" | "requirements" | "wearable" | "worn"> &
      Partial<Pick<Protocol.ItemData, "wearable" | "worn">> & {
        enhancement?: Enhancement;
        requirements?: readonly ItemRequirement[];
      }
  >>,
  Assert<AssertEqual<ItemSnapshot, DeepReadonly<Protocol.ItemSnapshot>>>,
  Assert<AssertEqual<MonsterDrop, DeepReadonly<Protocol.MonsterDrop>>>,
  Assert<AssertEqual<MonsterData, Protocol.MonsterData>>,
  Assert<AssertEqual<MonsterSnapshot, DeepReadonly<Protocol.MonsterSnapshot>>>,
  Assert<AssertEqual<Position, Readonly<Protocol.Point>>>,
  Assert<AssertEqual<
    PlayerData,
    Omit<Protocol.PlayerData, "position"> & { position: Readonly<Protocol.Point> }
  >>,
  Assert<AssertEqual<PlayerSnapshot, DeepReadonly<Protocol.PlayerSnapshot>>>,
  Assert<AssertEqual<QuestItem, Readonly<Protocol.QuestItem>>>,
  Assert<AssertEqual<QuestReward, Readonly<Protocol.QuestReward>>>,
  Assert<AssertEqual<QuestCadence, Protocol.QuestCadence>>,
  Assert<AssertEqual<
    QuestData,
    Omit<Protocol.QuestData, "requirements" | "rewards"> & {
      requirements: readonly QuestItem[];
      rewards: readonly QuestReward[];
    }
  >>,
  Assert<AssertEqual<QuestSnapshot, DeepReadonly<Protocol.QuestData>>>,
  Assert<AssertEqual<ServerData, Protocol.ServerData>>,
  Assert<AssertEqual<ServerSnapshot, Readonly<Protocol.ServerSnapshot>>>,
  Assert<AssertEqual<
    ShopData,
    Omit<Protocol.ShopData, "items"> & { items: readonly Item[] }
  >>,
  Assert<AssertEqual<ShopSnapshot, DeepReadonly<Protocol.ShopData>>>,
];

export type EqualityProofs = [
  // @ts-expect-error regrouped union readonly differs
  Assert<AssertEqual<{ readonly kind: "a" | "b" }, { kind: "a" } | { readonly kind: "b" }>>,
  // @ts-expect-error intersection optionality differs
  Assert<AssertEqual<{ readonly value?: string } & { count: number }, { readonly value: string; count: number }>>,
  // @ts-expect-error optional missing field differs
  Assert<AssertEqual<{ value: string; extra?: number }, { value: string }>>,
  // @ts-expect-error nested optional missing field differs
  Assert<AssertEqual<{ child: { value: string; extra?: number } }, { child: { value: string } }>>,
  Assert<AssertEqual<{ readonly kind: "a" | "b" }, { readonly kind: "a" } | { readonly kind: "b" }>>,
  // @ts-expect-error callable optional parameter differs
  Assert<AssertEqual<{ call: () => void }, { call: (extra?: string) => void }>>,
  // @ts-expect-error any differs
  Assert<AssertEqual<{ value: any }, { value: string }>>,
  // @ts-expect-error unknown differs
  Assert<AssertEqual<{ value: unknown }, { value: {} }>>,
  // @ts-expect-error optional extra field differs
  Assert<AssertEqual<{ value: string }, { value: string; extra?: number }>>,
  Assert<AssertEqual<{ readonly value?: string } & { count: number }, { readonly value?: string; count: number }>>,
  Assert<AssertEqual<{ child: { readonly value?: string } & { count: number } }, { child: { readonly value?: string; count: number } }>>,
  // @ts-expect-error nested protocol enhancement field differs
  Assert<AssertEqual<ItemSnapshot, DeepReadonly<Omit<Protocol.ItemSnapshot, "enhancement"> & { enhancement?: Protocol.EnhancementSnapshot & { extra?: string } }>>>,
  // @ts-expect-error readonly differs
  Assert<AssertEqual<{ readonly value: string }, { value: string }>>,
  // @ts-expect-error optionality differs
  Assert<AssertEqual<{ value?: string }, { value: string }>>,
  // @ts-expect-error explicit undefined differs
  Assert<AssertEqual<{ value?: string }, { value?: string | undefined }>>,
  // @ts-expect-error missing field differs
  Assert<AssertEqual<{ value: string; extra: number }, { value: string }>>,
  // @ts-expect-error extra field differs
  Assert<AssertEqual<{ value: string }, { value: string; extra: number }>>,
  // @ts-expect-error field type differs
  Assert<AssertEqual<{ value: string }, { value: number }>>,
  // @ts-expect-error index signature differs
  Assert<AssertEqual<{ value: string }, { value: string; [key: string]: string }>>,
  // @ts-expect-error nullability differs
  Assert<AssertEqual<{ value: string }, { value: string | null }>>,
  // @ts-expect-error nested readonly differs
  Assert<AssertEqual<{ child: { readonly value: string } }, { child: { value: string } }>>,
  // @ts-expect-error nested optionality differs
  Assert<AssertEqual<{ child: { value?: string } }, { child: { value: string } }>>,
  // @ts-expect-error nested explicit undefined differs
  Assert<AssertEqual<{ child: { value?: string } }, { child: { value?: string | undefined } }>>,
  // @ts-expect-error nested missing field differs
  Assert<AssertEqual<{ child: { value: string; extra: number } }, { child: { value: string } }>>,
  // @ts-expect-error nested extra field differs
  Assert<AssertEqual<{ child: { value: string } }, { child: { value: string; extra: number } }>>,
  // @ts-expect-error nested field type differs
  Assert<AssertEqual<{ child: { value: string } }, { child: { value: number } }>>,
  // @ts-expect-error nested index signature differs
  Assert<AssertEqual<{ child: { value: string } }, { child: { value: string; [key: string]: string } }>>,
  // @ts-expect-error nested nullability differs
  Assert<AssertEqual<{ child: { value: string } }, { child: { value: string | null } }>>,
];
