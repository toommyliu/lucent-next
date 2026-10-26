import * as Schema from "effect/Schema";
type SettingsCommandCategory = "General" | "Scripts" | "Options" | "Windows";
type SettingsCommandGroup = "Rendering Mode";
type HotkeyDisplayPlatform = "linux" | "mac" | "windows";
declare const SETTINGS_COMMAND_IDS: readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"];
declare const SETTING_COMMAND_CATEGORIES: readonly ["General", "Scripts", "Options", "Windows"];
type SettingsCommandId = (typeof SETTINGS_COMMAND_IDS)[number];
interface SettingsCommandDefinition {
  readonly category: SettingsCommandCategory;
  readonly defaultHotkey: string;
  readonly description?: string;
  readonly group?: SettingsCommandGroup;
  readonly id: SettingsCommandId;
  readonly label: string;
}
interface HotkeyBinding {
  readonly id: SettingsCommandId;
  readonly value: string;
}
interface HotkeyBindingPatch {
  readonly id: SettingsCommandId;
  readonly value: string | null;
}
interface HotkeysSettings {
  readonly bindings: readonly HotkeyBinding[];
}
interface HotkeysPatch {
  readonly bindings?: readonly HotkeyBindingPatch[];
}
declare const SettingsCommandIdSchema: Schema.Literals<readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"]>;
declare const HotkeyBindingSchema: Schema.Struct<{
  readonly id: Schema.Literals<readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"]>;
  readonly value: Schema.String;
}>;
declare const HotkeyBindingPatchSchema: Schema.Struct<{
  readonly id: Schema.Literals<readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"]>;
  readonly value: Schema.NullOr<Schema.String>;
}>;
declare const HotkeysSettingsSchema: Schema.Struct<{
  readonly bindings: Schema.$Array<Schema.Struct<{
    readonly id: Schema.Literals<readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"]>;
    readonly value: Schema.String;
  }>>;
}>;
declare const HotkeysPatchSchema: Schema.Struct<{
  readonly bindings: Schema.optionalKey<Schema.$Array<Schema.Struct<{
    readonly id: Schema.Literals<readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"]>;
    readonly value: Schema.NullOr<Schema.String>;
  }>>>;
}>;
declare const SETTINGS_COMMANDS: readonly SettingsCommandDefinition[];
declare const isSettingsCommandId: (value: unknown) => value is SettingsCommandId;
declare const getSettingsCommandDefinition: (id: SettingsCommandId) => SettingsCommandDefinition;
declare const createDefaultHotkeyBindings: () => readonly HotkeyBinding[];
declare const DEFAULT_HOTKEYS: HotkeysSettings;
declare const normalizeHotkeyBindingValue: (value: unknown) => string | null;
declare const hotkeyBindingMatchKey: (value: string, platform: HotkeyDisplayPlatform) => string | null;
declare const normalizeHotkeySettings: (value: unknown) => HotkeysSettings;
declare const readHotkeyBinding: (bindings: readonly HotkeyBinding[], id: SettingsCommandId) => string;
declare const hotkeyConflictKey: (value: string) => string | null;
declare const findDuplicateHotkeyBinding: (bindings: readonly HotkeyBinding[]) => HotkeyBinding | null;
declare const formatHotkeyDisplayParts: (value: string, platform: HotkeyDisplayPlatform, emptyLabel?: string) => readonly string[];
declare const formatHotkeyDisplay: (value: string, platform: HotkeyDisplayPlatform, emptyLabel?: string) => string;
export { getSettingsCommandDefinition as C, normalizeHotkeyBindingValue as D, isSettingsCommandId as E, normalizeHotkeySettings as O, formatHotkeyDisplayParts as S, hotkeyConflictKey as T, SettingsCommandId as _, HotkeyBindingSchema as a, findDuplicateHotkeyBinding as b, HotkeysPatchSchema as c, SETTINGS_COMMANDS as d, SETTINGS_COMMAND_IDS as f, SettingsCommandGroup as g, SettingsCommandDefinition as h, HotkeyBindingPatchSchema as i, readHotkeyBinding as k, HotkeysSettings as l, SettingsCommandCategory as m, HotkeyBinding as n, HotkeyDisplayPlatform as o, SETTING_COMMAND_CATEGORIES as p, HotkeyBindingPatch as r, HotkeysPatch as s, DEFAULT_HOTKEYS as t, HotkeysSettingsSchema as u, SettingsCommandIdSchema as v, hotkeyBindingMatchKey as w, formatHotkeyDisplay as x, createDefaultHotkeyBindings as y };