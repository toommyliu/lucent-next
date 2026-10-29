import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";

import { makeScriptSettingsApi } from "../../scripting/api/Settings";
import { makeFakeHost, makeFakeHostState } from "../testing/FakeHost";
import { makeSettings } from "./Settings";

describe("Settings", () => {
  it.effect("maps script patches and snapshots through the host", () =>
    Effect.gen(function* () {
      const state = makeFakeHostState();
      const host = makeFakeHost(state, { swf: {} } as Window);
      const settings = makeSettings(host);
      const api = makeScriptSettingsApi(settings);
      yield* api.update({
        animations: false, antiCounter: false, collisions: false,
        customGuild: "Lucent", customName: "Tommy", deathAds: false,
        enemyMagnet: true, frameRate: 12, infiniteRange: true,
        hidePlayers: true, provokeCell: true, renderingMode: "minimal",
        skipCutscenes: true, walkSpeed: 20,
      });
      expect(yield* api.get()).toEqual({
        animations: false, antiCounter: false, collisions: false,
        customGuild: "Lucent", customName: "Tommy", deathAds: false,
        enemyMagnet: true, frameRate: 12, infiniteRange: true,
        hidePlayers: true, provokeCell: true, renderingMode: "minimal",
        skipCutscenes: true, walkSpeed: 20,
      });
      expect(yield* settings.isAntiCounterEnabled()).toBe(false);
      yield* api.update({ customName: null, customGuild: null });
      yield* api.update({ walkSpeed: 30 });
      const snapshot = yield* api.get();
      expect([snapshot.customName, snapshot.customGuild, snapshot.walkSpeed]).toEqual([null, null, 30]);
      expect(Object.isFrozen(snapshot)).toBe(true);
    }),
  );
});
