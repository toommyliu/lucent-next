import * as Effect from "effect/Effect";
import * as SubscriptionRef from "effect/SubscriptionRef";

import { makeSettingsState, type SettingsState } from "./Settings";

export const makeStore = Effect.gen(function* () {
  const settingsRef = yield* SubscriptionRef.make(makeSettingsState());

  const settings = {
    changes: SubscriptionRef.changes(settingsRef),
    get: SubscriptionRef.get(settingsRef),
    patch: (patch: Partial<SettingsState>) =>
      SubscriptionRef.updateAndGet(settingsRef, (state) => ({
        ...state,
        ...patch,
      })),
  };

  return { settings };
});

export type Store = Effect.Success<typeof makeStore>;
