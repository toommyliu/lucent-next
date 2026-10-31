import type { AutoZoneMap } from "@lucent/game/protocol";

export type AutoZoneSupportedMap = AutoZoneMap;

export interface AutoZoneState {
  readonly enabled: boolean;
  readonly map: AutoZoneSupportedMap | undefined;
}
