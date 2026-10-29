import type { SettingsPatch } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";

import type { HostStateService } from "../state/HostState";

export const makeSettings = (host: HostStateService) => {
  const get = () => host.read({ kind: "settings" });
  return {
    apply: (patch: SettingsPatch) =>
      host.command({ kind: "updateSettings", patch }).pipe(Effect.asVoid),
    get,
    isAntiCounterEnabled: () =>
      get().pipe(Effect.map((state) => state.antiCounterEnabled)),
    reapply: () =>
      host
        .command({ kind: "reapplySettings", actionsOnly: false })
        .pipe(Effect.asVoid),
    reapplyActions: () =>
      host
        .command({ kind: "reapplySettings", actionsOnly: true })
        .pipe(Effect.asVoid),
  };
};

export type Settings = ReturnType<typeof makeSettings>;
