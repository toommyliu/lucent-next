import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";

import { makeFakeApi, makeFakeHostState } from "../testing/FakeHost";

const closeShop = (confirmFails: boolean) =>
  Effect.gen(function* () {
    const state = makeFakeHostState();
    state.shop = {
      house: false,
      id: 101,
      items: [],
      limited: false,
      merge: false,
      name: "Armor Shop",
    };
    let open = true;
    let checksAfterClose = 0;
    const target = {} as Window;
    target.swf = {
      "shops.close": () => {
        open = false;
        return true;
      },
      "shops.isOpen": () => {
        if (!open && confirmFails && ++checksAfterClose > 1) {
          throw new Error("isOpen failed");
        }
        return open;
      },
    } as unknown as Window["swf"];
    const api = yield* makeFakeApi(target, state);

    const closed = yield* api.shops.close(101);
    const current = yield* api.shops.getCurrent();
    return { closed, current: current?.id ?? null };
  });

describe("Shops.close", () => {
  it.effect("clears the shop once the host confirms it closed", () =>
    Effect.gen(function* () {
      expect(yield* closeShop(false)).toEqual({ closed: true, current: null });
      expect(yield* closeShop(true)).toEqual({ closed: false, current: 101 });
    }),
  );
});
