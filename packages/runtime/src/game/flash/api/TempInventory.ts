import { normalizeItemQuantity } from "@lucent/game";
import type { ItemQuery } from "@lucent/game";
import * as Effect from "effect/Effect";

import {
  itemQuantity,
  readItem,
  readItems,
  type HostStateService,
} from "../state/HostState";

export const makeTempInventory = (host: HostStateService) => {
  const contains = (selector: ItemQuery, quantity?: number) =>
    itemQuantity(host, "temporary", selector).pipe(
      Effect.map((owned) => owned >= normalizeItemQuantity(quantity)),
    );

  const get = (selector: ItemQuery) => readItem(host, "temporary", selector);

  const getAll = () => readItems(host, "temporary");

  return {
    contains,
    get,
    getAll,
  };
};

export type TempInventory = ReturnType<typeof makeTempInventory>;
