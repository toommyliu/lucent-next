import { l as HotkeysSettings } from "./hotkeys-BdLGhI6F.cjs";
import * as Schema from "effect/Schema";
declare const APP_SETTINGS_VERSION = 1;
declare const AppLaunchModeSchema: Schema.Literals<readonly ["game", "account-manager"]>;
declare const ThemeModeSchema: Schema.Literals<readonly ["system", "light", "dark"]>;
declare const MotionModeSchema: Schema.Literals<readonly ["system", "on", "off"]>;
declare const ThemeVariantSchema: Schema.Literals<readonly ["light", "dark"]>;
type AppSettingsVersion = typeof APP_SETTINGS_VERSION;
type AppLaunchMode = typeof AppLaunchModeSchema.Type;
type ThemeMode = typeof ThemeModeSchema.Type;
type MotionMode = typeof MotionModeSchema.Type;
type ThemeVariant = typeof ThemeVariantSchema.Type;
type ThemeRgb = readonly [number, number, number];
declare const THEME_TOKEN_NAMES: readonly ["background", "foreground", "card", "cardForeground", "popover", "popoverForeground", "primary", "primaryForeground", "secondary", "secondaryForeground", "muted", "mutedForeground", "accent", "accentForeground", "destructive", "destructiveForeground", "success", "successForeground", "warning", "warningForeground", "info", "infoForeground", "border", "input", "ring"];
declare const ThemeTokenNameSchema: Schema.Literals<readonly ["background", "foreground", "card", "cardForeground", "popover", "popoverForeground", "primary", "primaryForeground", "secondary", "secondaryForeground", "muted", "mutedForeground", "accent", "accentForeground", "destructive", "destructiveForeground", "success", "successForeground", "warning", "warningForeground", "info", "infoForeground", "border", "input", "ring"]>;
declare const ThemeRgbSchema: Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>;
declare const UnknownRecordSchema: Schema.$Record<Schema.String, Schema.Unknown>;
declare const THEME_FONT_MIN_LENGTH = 1;
declare const THEME_FONT_MAX_LENGTH = 256;
declare const THEME_FONT_SIZE_MIN = 10;
declare const THEME_FONT_SIZE_MAX = 24;
declare const THEME_ROUNDING_MIN = 0;
declare const THEME_ROUNDING_MAX = 2;
declare const ThemeFontSchema: Schema.String;
declare const ThemeFontSizeSchema: Schema.Int;
declare const ThemeRoundingSchema: Schema.Finite;
declare const ThemeTokenValuesSchema: Schema.$Record<Schema.Literals<readonly ["background", "foreground", "card", "cardForeground", "popover", "popoverForeground", "primary", "primaryForeground", "secondary", "secondaryForeground", "muted", "mutedForeground", "accent", "accentForeground", "destructive", "destructiveForeground", "success", "successForeground", "warning", "warningForeground", "info", "infoForeground", "border", "input", "ring"]>, Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>;
declare const ThemeProfileSchema: Schema.Struct<{
  readonly tokens: Schema.$Record<Schema.Literals<readonly ["background", "foreground", "card", "cardForeground", "popover", "popoverForeground", "primary", "primaryForeground", "secondary", "secondaryForeground", "muted", "mutedForeground", "accent", "accentForeground", "destructive", "destructiveForeground", "success", "successForeground", "warning", "warningForeground", "info", "infoForeground", "border", "input", "ring"]>, Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>;
  readonly sansFont: Schema.String;
  readonly monoFont: Schema.String;
  readonly sansFontSize: Schema.Int;
  readonly monoFontSize: Schema.Int;
  readonly rounding: Schema.Finite;
}>;
declare const ThemeProfilePatchSchema: Schema.Struct<{
  readonly tokens: Schema.optionalKey<Schema.Struct<{
    readonly background: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly foreground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly card: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly cardForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly popover: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly popoverForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly primary: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly primaryForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly secondary: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly secondaryForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly muted: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly mutedForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly accent: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly accentForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly destructive: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly destructiveForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly success: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly successForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly warning: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly warningForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly info: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly infoForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly border: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly input: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
    readonly ring: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
  }>>;
  readonly sansFont: Schema.optionalKey<Schema.String>;
  readonly monoFont: Schema.optionalKey<Schema.String>;
  readonly sansFontSize: Schema.optionalKey<Schema.Int>;
  readonly monoFontSize: Schema.optionalKey<Schema.Int>;
  readonly rounding: Schema.optionalKey<Schema.Finite>;
}>;
declare const PreferencesPatchSchema: Schema.Struct<{
  readonly checkForUpdates: Schema.optionalKey<Schema.Boolean>;
  readonly launchMode: Schema.optionalKey<Schema.Literals<readonly ["game", "account-manager"]>>;
  readonly showGameUsernameInWindowTitle: Schema.optionalKey<Schema.Boolean>;
  readonly useGameTabs: Schema.optionalKey<Schema.Boolean>;
}>;
declare const AppearancePatchSchema: Schema.Struct<{
  readonly themeMode: Schema.optionalKey<Schema.Literals<readonly ["system", "light", "dark"]>>;
  readonly reduceMotion: Schema.optionalKey<Schema.Literals<readonly ["system", "on", "off"]>>;
  readonly useCursorPointers: Schema.optionalKey<Schema.Boolean>;
  readonly themes: Schema.optionalKey<Schema.Struct<{
    readonly light: Schema.optionalKey<Schema.Struct<{
      readonly tokens: Schema.optionalKey<Schema.Struct<{
        readonly background: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly foreground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly card: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly cardForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly popover: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly popoverForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly primary: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly primaryForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly secondary: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly secondaryForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly muted: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly mutedForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly accent: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly accentForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly destructive: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly destructiveForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly success: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly successForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly warning: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly warningForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly info: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly infoForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly border: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly input: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly ring: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
      }>>;
      readonly sansFont: Schema.optionalKey<Schema.String>;
      readonly monoFont: Schema.optionalKey<Schema.String>;
      readonly sansFontSize: Schema.optionalKey<Schema.Int>;
      readonly monoFontSize: Schema.optionalKey<Schema.Int>;
      readonly rounding: Schema.optionalKey<Schema.Finite>;
    }>>;
    readonly dark: Schema.optionalKey<Schema.Struct<{
      readonly tokens: Schema.optionalKey<Schema.Struct<{
        readonly background: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly foreground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly card: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly cardForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly popover: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly popoverForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly primary: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly primaryForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly secondary: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly secondaryForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly muted: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly mutedForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly accent: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly accentForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly destructive: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly destructiveForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly success: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly successForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly warning: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly warningForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly info: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly infoForeground: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly border: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly input: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
        readonly ring: Schema.optionalKey<Schema.NullOr<Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>>;
      }>>;
      readonly sansFont: Schema.optionalKey<Schema.String>;
      readonly monoFont: Schema.optionalKey<Schema.String>;
      readonly sansFontSize: Schema.optionalKey<Schema.Int>;
      readonly monoFontSize: Schema.optionalKey<Schema.Int>;
      readonly rounding: Schema.optionalKey<Schema.Finite>;
    }>>;
  }>>;
}>;
type ThemeTokenName = (typeof THEME_TOKEN_NAMES)[number];
type ThemeTokenValues = Record<ThemeTokenName, ThemeRgb>;
interface PreferencesPatch {
  readonly checkForUpdates?: boolean;
  readonly launchMode?: AppLaunchMode;
  readonly showGameUsernameInWindowTitle?: boolean;
  readonly useGameTabs?: boolean;
}
interface ThemeProfilePatch {
  readonly monoFont?: string;
  readonly monoFontSize?: number;
  readonly rounding?: number;
  readonly sansFont?: string;
  readonly sansFontSize?: number;
  readonly tokens?: Partial<Record<ThemeTokenName, ThemeRgb | null>>;
}
interface AppearancePatch {
  readonly reduceMotion?: MotionMode;
  readonly themeMode?: ThemeMode;
  readonly themes?: Partial<Record<ThemeVariant, ThemeProfilePatch>>;
  readonly useCursorPointers?: boolean;
}
interface ThemeProfile {
  readonly tokens: ThemeTokenValues;
  readonly sansFont: string;
  readonly monoFont: string;
  readonly sansFontSize: number;
  readonly monoFontSize: number;
  readonly rounding: number;
}
interface AppSettings {
  readonly version: AppSettingsVersion;
  readonly preferences: {
    readonly checkForUpdates: boolean;
    readonly launchMode: AppLaunchMode;
    readonly showGameUsernameInWindowTitle: boolean;
    readonly useGameTabs: boolean;
  };
  readonly appearance: {
    readonly themeMode: ThemeMode;
    readonly reduceMotion: MotionMode;
    readonly useCursorPointers: boolean;
    readonly themes: Record<ThemeVariant, ThemeProfile>;
  };
  readonly hotkeys: HotkeysSettings;
}
declare const AppSettingsSchema: Schema.Struct<{
  readonly version: Schema.Literal<1>;
  readonly preferences: Schema.Struct<{
    readonly checkForUpdates: Schema.Boolean;
    readonly launchMode: Schema.Literals<readonly ["game", "account-manager"]>;
    readonly showGameUsernameInWindowTitle: Schema.Boolean;
    readonly useGameTabs: Schema.Boolean;
  }>;
  readonly appearance: Schema.Struct<{
    readonly themeMode: Schema.Literals<readonly ["system", "light", "dark"]>;
    readonly reduceMotion: Schema.Literals<readonly ["system", "on", "off"]>;
    readonly useCursorPointers: Schema.Boolean;
    readonly themes: Schema.Struct<{
      readonly light: Schema.Struct<{
        readonly tokens: Schema.$Record<Schema.Literals<readonly ["background", "foreground", "card", "cardForeground", "popover", "popoverForeground", "primary", "primaryForeground", "secondary", "secondaryForeground", "muted", "mutedForeground", "accent", "accentForeground", "destructive", "destructiveForeground", "success", "successForeground", "warning", "warningForeground", "info", "infoForeground", "border", "input", "ring"]>, Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>;
        readonly sansFont: Schema.String;
        readonly monoFont: Schema.String;
        readonly sansFontSize: Schema.Int;
        readonly monoFontSize: Schema.Int;
        readonly rounding: Schema.Finite;
      }>;
      readonly dark: Schema.Struct<{
        readonly tokens: Schema.$Record<Schema.Literals<readonly ["background", "foreground", "card", "cardForeground", "popover", "popoverForeground", "primary", "primaryForeground", "secondary", "secondaryForeground", "muted", "mutedForeground", "accent", "accentForeground", "destructive", "destructiveForeground", "success", "successForeground", "warning", "warningForeground", "info", "infoForeground", "border", "input", "ring"]>, Schema.Tuple<readonly [Schema.Int, Schema.Int, Schema.Int]>>;
        readonly sansFont: Schema.String;
        readonly monoFont: Schema.String;
        readonly sansFontSize: Schema.Int;
        readonly monoFontSize: Schema.Int;
        readonly rounding: Schema.Finite;
      }>;
    }>;
  }>;
  readonly hotkeys: Schema.Struct<{
    readonly bindings: Schema.$Array<Schema.Struct<{
      readonly id: Schema.Literals<readonly ["toggleTopBar", "loadScript", "toggleScript", "toggleScriptsDialog", "toggleOptionsMenu", "openEnvironment", "openLoaderGrabber", "openFollower", "openPackets", "toggleAutoattack", "toggleFollower", "toggleBank", "toggleInfiniteRange", "toggleProvokeCell", "toggleEnemyMagnet", "toggleInterfaceOnlyRendering", "toggleMinimalRendering", "toggleHidePlayers", "toggleSkipCutscenes", "toggleAntiCounter", "toggleAnimations", "toggleCollisions", "toggleDeathAds"]>;
      readonly value: Schema.String;
    }>>;
  }>;
}>;
declare const DEFAULT_APP_SETTINGS: AppSettings;
declare const isAppLaunchMode: (value: unknown) => value is AppLaunchMode;
declare const normalizeAppSettings: (value: unknown) => AppSettings;
declare const serializeAppSettings: (settings: AppSettings) => unknown;
export { AppLaunchMode, AppLaunchModeSchema, AppSettings, AppSettingsSchema, AppSettingsVersion, AppearancePatch, AppearancePatchSchema, DEFAULT_APP_SETTINGS, MotionMode, MotionModeSchema, PreferencesPatch, PreferencesPatchSchema, THEME_FONT_MAX_LENGTH, THEME_FONT_MIN_LENGTH, THEME_FONT_SIZE_MAX, THEME_FONT_SIZE_MIN, THEME_ROUNDING_MAX, THEME_ROUNDING_MIN, THEME_TOKEN_NAMES, ThemeFontSchema, ThemeFontSizeSchema, ThemeMode, ThemeModeSchema, ThemeProfile, ThemeProfilePatch, ThemeProfilePatchSchema, ThemeProfileSchema, ThemeRgb, ThemeRgbSchema, ThemeRoundingSchema, ThemeTokenName, ThemeTokenNameSchema, ThemeTokenValues, ThemeTokenValuesSchema, ThemeVariant, ThemeVariantSchema, UnknownRecordSchema, isAppLaunchMode, normalizeAppSettings, serializeAppSettings };