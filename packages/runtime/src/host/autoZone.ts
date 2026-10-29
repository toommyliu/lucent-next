import type { AutoZoneConfig, AutoZonePatch } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";

import type { AutoZoneState, AutoZoneSupportedMap } from "../game/automation/AutoZone";

export const makeAutoZoneService = (
  host: Pick<typeof __host, "read" | "command">,
) => {
  const state = (config: AutoZoneConfig): AutoZoneState => ({
    enabled: config.enabled,
    map: config.map ?? undefined,
  });
  const getState = () => Effect.sync(() => state(
    JSON.parse(host.read(JSON.stringify({ kind: "autoZone" }))) as AutoZoneConfig,
  ));
  const patch = (patch: AutoZonePatch) => Effect.sync(() => state(
    JSON.parse(host.command(JSON.stringify({ kind: "patchAutoZone", patch }))) as AutoZoneConfig,
  ));

  return {
    getState,
    getMap: () => getState().pipe(Effect.map((state) => state.map)),
    isEnabled: () => getState().pipe(Effect.map((state) => state.enabled)),
    setEnabled: (enabled: boolean) => patch({ enabled }),
    setMap: (map: AutoZoneSupportedMap | undefined) => patch({ map: map ?? null }),
  };
};
