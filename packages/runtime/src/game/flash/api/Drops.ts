import type { ItemQuery } from "@lucent/game";
import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
import * as Semaphore from "effect/Semaphore";

import type { BridgeService } from "../bridge/Bridge";
import { PositiveWireInt, WireBoolean, WireInt } from "../contract/Coercion";
import { packetData } from "../contract/Packet";
import {
  isSelectable,
  itemQuantity,
  readItem,
  readItems,
  type HostStateService,
} from "../state/HostState";
import type { Auth } from "./Auth";
import type { Wait } from "./Wait";

const AcceptResponse = Schema.Struct({
  ItemID: PositiveWireInt,
  bBank: Schema.optionalKey(WireBoolean),
  bHouse: Schema.optionalKey(WireBoolean),
  bSuccess: WireBoolean,
  iQty: Schema.optionalKey(WireInt),
  iQtyNow: Schema.optionalKey(WireInt),
});
const decodeAcceptResponse = Schema.decodeUnknownOption(AcceptResponse);

export const makeDrops = Effect.fnUntraced(function* (
  bridge: BridgeService,
  host: HostStateService,
  auth: Auth,
  wait: Wait,
) {
  const accepts = yield* Semaphore.make(1);

  const get = (selector: ItemQuery) => readItem(host, "drop", selector);

  const accept = (selector: ItemQuery) => {
    return accepts.withPermits(1)(
      Effect.gen(function* () {
        if (!(yield* auth.isLoggedIn())) return false;
        const drop = yield* get(selector);
        if (drop === null) return false;

        const before = yield* Effect.all({
          bank: itemQuantity(host, "bank", drop.itemId),
          house: itemQuantity(host, "house", drop.itemId),
          inventory: itemQuantity(host, "inventory", drop.itemId),
        });
        let response: typeof AcceptResponse.Type | undefined;
        const packet = yield* wait.forPacket(
          {
            command: "getDrop",
            direction: "extension",
            predicate: (candidate) => {
              const decoded = decodeAcceptResponse(packetData(candidate));
              if (
                Option.isNone(decoded) ||
                decoded.value.ItemID !== drop.itemId
              ) {
                return false;
              }
              response = decoded.value;
              return true;
            },
            encoding: "json",
          },
          {
            timeout: "10 seconds",
            trigger: bridge
              .invoke("drops.accept", [drop.itemId], Schema.Boolean)
              .pipe(Effect.map(Option.getOrElse(() => false))),
          },
        );
        if (packet === null || response?.bSuccess !== true) return false;

        const container =
          response.bBank && drop.category !== "Class"
            ? "bank"
            : response.bHouse || drop.houseItem
              ? "house"
              : "inventory";
        const expected =
          response.iQtyNow ??
          before[container] + (response.iQty ?? drop.quantity);
        const owned = yield* readItem(host, container, drop.itemId);
        return owned !== null && owned.quantity >= expected;
      }),
    );
  };

  const contains = (selector: ItemQuery) =>
    get(selector).pipe(Effect.map((drop) => drop !== null));

  const getAll = () => readItems(host, "drop");

  const isCustomUiEnabled = () =>
    bridge
      .invoke("drops.isUsingCustomDrops", undefined, Schema.Boolean)
      .pipe(Effect.map(Option.getOrElse(() => false)));

  const reject = (selector: ItemQuery) =>
    Effect.gen(function* () {
      if (!(yield* auth.isLoggedIn())) return false;
      if (!isSelectable(selector)) return false;
      const rejected = yield* host.command({ kind: "rejectDrop", selector });
      return Option.getOrElse(rejected, () => false);
    });

  const toggleUi = () =>
    bridge.invoke("drops.toggleUi", undefined, Schema.Void).pipe(Effect.asVoid);

  return {
    accept,
    contains,
    get,
    getAll,
    isCustomUiEnabled,
    reject,
    toggleUi,
  };
});

export type Drops = Effect.Success<ReturnType<typeof makeDrops>>;
