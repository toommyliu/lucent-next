import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";

import { makeFakeApi, makeFakeHostState } from "../testing/FakeHost";

describe("Player.isReady", () => {
  it.effect("waits for the host to report nothing missing", () =>
    Effect.gen(function* () {
      const state = makeFakeHostState();
      state.missing = ["inventory", "player"];
      const target = {} as Window;
      target.swf = {
        "auth.isLoggedIn": () => true,
        "player.isLoaded": () => true,
        "world.isLoaded": () => true,
      } as unknown as Window["swf"];
      const api = yield* makeFakeApi(target, state);

      expect(yield* api.player.isReady()).toBe(false);
      state.missing = [];
      expect(yield* api.player.isReady()).toBe(true);
    }),
  );
});
