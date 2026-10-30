import type { GameSettings } from "@lucent/game/protocol";

export type SettingsState = GameSettings;

export const makeSettingsState = (): SettingsState => ({
  animationsEnabled: true,
  antiCounterEnabled: true,
  collisionsEnabled: true,
  customGuild: "",
  customGuildConfigured: false,
  customName: "",
  customNameConfigured: false,
  deathAdsVisible: true,
  enemyMagnetEnabled: false,
  frameRate: 24,
  infiniteRangeEnabled: false,
  otherPlayersVisible: true,
  provokeCellEnabled: false,
  renderingMode: "full",
  skipCutscenesEnabled: false,
  walkSpeed: 8,
});
