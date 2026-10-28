import type { Item } from "./item";
import { LiveModel } from "./model";
import type { DeepReadonly } from "./model";
import type * as Protocol from "./protocol";

export interface Shop {
  readonly house: boolean;
  readonly id: number;
  readonly items: readonly Item[];
  readonly limited: boolean;
  readonly merge: boolean;
  readonly name: string;
  toJSON(): ShopSnapshot;
}

export type ShopData = Omit<Protocol.ShopData, "items"> & {
  items: readonly Item[];
};

export type ShopSnapshot = DeepReadonly<Protocol.ShopData>;

export class LiveShop extends LiveModel<ShopData> implements Shop {
  get house(): boolean {
    return this.modelData.house;
  }
  get id(): number {
    return this.modelData.id;
  }
  get items(): readonly Item[] {
    return this.modelData.items;
  }
  get limited(): boolean {
    return this.modelData.limited;
  }
  get merge(): boolean {
    return this.modelData.merge;
  }
  get name(): string {
    return this.modelData.name;
  }
  toJSON(): ShopSnapshot {
    return {
      ...this.modelData,
      items: this.items.map((item) => item.toJSON()),
    };
  }
}
