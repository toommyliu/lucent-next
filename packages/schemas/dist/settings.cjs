Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_hotkeys = require("./hotkeys.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/settings.ts
const APP_SETTINGS_VERSION = 1;
const AppLaunchModeSchema = effect_Schema.Literals(["game", "account-manager"]);
const ThemeModeSchema = effect_Schema.Literals([
	"system",
	"light",
	"dark"
]);
const MotionModeSchema = effect_Schema.Literals([
	"system",
	"on",
	"off"
]);
const ThemeVariantSchema = effect_Schema.Literals(["light", "dark"]);
const THEME_TOKEN_NAMES = [
	"background",
	"foreground",
	"card",
	"cardForeground",
	"popover",
	"popoverForeground",
	"primary",
	"primaryForeground",
	"secondary",
	"secondaryForeground",
	"muted",
	"mutedForeground",
	"accent",
	"accentForeground",
	"destructive",
	"destructiveForeground",
	"success",
	"successForeground",
	"warning",
	"warningForeground",
	"info",
	"infoForeground",
	"border",
	"input",
	"ring"
];
const ThemeTokenNameSchema = effect_Schema.Literals(THEME_TOKEN_NAMES);
const ThemeRgbSchema = effect_Schema.Tuple([
	effect_Schema.Int.check(effect_Schema.isBetween({
		minimum: 0,
		maximum: 255
	})),
	effect_Schema.Int.check(effect_Schema.isBetween({
		minimum: 0,
		maximum: 255
	})),
	effect_Schema.Int.check(effect_Schema.isBetween({
		minimum: 0,
		maximum: 255
	}))
]);
const UnknownRecordSchema = effect_Schema.Record(effect_Schema.String, effect_Schema.Unknown);
const THEME_FONT_MIN_LENGTH = 1;
const THEME_FONT_MAX_LENGTH = 256;
const THEME_FONT_SIZE_MIN = 10;
const THEME_FONT_SIZE_MAX = 24;
const THEME_ROUNDING_MIN = 0;
const THEME_ROUNDING_MAX = 2;
const ThemeFontSchema = effect_Schema.String.check(effect_Schema.isBetweenLength(1, 256));
const ThemeFontSizeSchema = effect_Schema.Int.check(effect_Schema.isBetween({
	minimum: 10,
	maximum: 24
}));
const ThemeRoundingSchema = effect_Schema.Finite.check(effect_Schema.isBetween({
	minimum: 0,
	maximum: 2
}));
const ThemeTokenValuesSchema = effect_Schema.Record(ThemeTokenNameSchema, ThemeRgbSchema);
const OptionalThemeRgbSchema = effect_Schema.optionalKey(effect_Schema.NullOr(ThemeRgbSchema));
const ThemeTokenPatchSchema = effect_Schema.Struct({
	background: OptionalThemeRgbSchema,
	foreground: OptionalThemeRgbSchema,
	card: OptionalThemeRgbSchema,
	cardForeground: OptionalThemeRgbSchema,
	popover: OptionalThemeRgbSchema,
	popoverForeground: OptionalThemeRgbSchema,
	primary: OptionalThemeRgbSchema,
	primaryForeground: OptionalThemeRgbSchema,
	secondary: OptionalThemeRgbSchema,
	secondaryForeground: OptionalThemeRgbSchema,
	muted: OptionalThemeRgbSchema,
	mutedForeground: OptionalThemeRgbSchema,
	accent: OptionalThemeRgbSchema,
	accentForeground: OptionalThemeRgbSchema,
	destructive: OptionalThemeRgbSchema,
	destructiveForeground: OptionalThemeRgbSchema,
	success: OptionalThemeRgbSchema,
	successForeground: OptionalThemeRgbSchema,
	warning: OptionalThemeRgbSchema,
	warningForeground: OptionalThemeRgbSchema,
	info: OptionalThemeRgbSchema,
	infoForeground: OptionalThemeRgbSchema,
	border: OptionalThemeRgbSchema,
	input: OptionalThemeRgbSchema,
	ring: OptionalThemeRgbSchema
});
const ThemeProfileSchema = effect_Schema.Struct({
	tokens: ThemeTokenValuesSchema,
	sansFont: ThemeFontSchema,
	monoFont: ThemeFontSchema,
	sansFontSize: ThemeFontSizeSchema,
	monoFontSize: ThemeFontSizeSchema,
	rounding: ThemeRoundingSchema
});
const ThemeProfilePatchSchema = effect_Schema.Struct({
	tokens: effect_Schema.optionalKey(ThemeTokenPatchSchema),
	sansFont: effect_Schema.optionalKey(ThemeFontSchema),
	monoFont: effect_Schema.optionalKey(ThemeFontSchema),
	sansFontSize: effect_Schema.optionalKey(ThemeFontSizeSchema),
	monoFontSize: effect_Schema.optionalKey(ThemeFontSizeSchema),
	rounding: effect_Schema.optionalKey(ThemeRoundingSchema)
});
const PreferencesPatchSchema = effect_Schema.Struct({
	checkForUpdates: effect_Schema.optionalKey(effect_Schema.Boolean),
	launchMode: effect_Schema.optionalKey(AppLaunchModeSchema),
	showGameUsernameInWindowTitle: effect_Schema.optionalKey(effect_Schema.Boolean),
	useGameTabs: effect_Schema.optionalKey(effect_Schema.Boolean)
});
const AppearancePatchSchema = effect_Schema.Struct({
	themeMode: effect_Schema.optionalKey(ThemeModeSchema),
	reduceMotion: effect_Schema.optionalKey(MotionModeSchema),
	useCursorPointers: effect_Schema.optionalKey(effect_Schema.Boolean),
	themes: effect_Schema.optionalKey(effect_Schema.Struct({
		light: effect_Schema.optionalKey(ThemeProfilePatchSchema),
		dark: effect_Schema.optionalKey(ThemeProfilePatchSchema)
	}))
});
const AppSettingsSchema = effect_Schema.Struct({
	version: effect_Schema.Literal(APP_SETTINGS_VERSION),
	preferences: effect_Schema.Struct({
		checkForUpdates: effect_Schema.Boolean,
		launchMode: AppLaunchModeSchema,
		showGameUsernameInWindowTitle: effect_Schema.Boolean,
		useGameTabs: effect_Schema.Boolean
	}),
	appearance: effect_Schema.Struct({
		themeMode: ThemeModeSchema,
		reduceMotion: MotionModeSchema,
		useCursorPointers: effect_Schema.Boolean,
		themes: effect_Schema.Struct({
			light: ThemeProfileSchema,
			dark: ThemeProfileSchema
		})
	}),
	hotkeys: require_hotkeys.HotkeysSettingsSchema
});
const DEFAULT_SANS_FONT = "-apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif";
const DEFAULT_MONO_FONT = "ui-monospace, \"SFMono-Regular\", \"SF Mono\", Menlo, Consolas, \"Liberation Mono\", monospace";
const DEFAULT_SANS_FONT_SIZE = 14;
const DEFAULT_MONO_FONT_SIZE = 12;
const DEFAULT_THEME_TOKENS = {
	light: {
		background: [
			255,
			255,
			255
		],
		foreground: [
			38,
			38,
			38
		],
		card: [
			255,
			255,
			255
		],
		cardForeground: [
			38,
			38,
			38
		],
		popover: [
			255,
			255,
			255
		],
		popoverForeground: [
			38,
			38,
			38
		],
		primary: [
			38,
			38,
			38
		],
		primaryForeground: [
			250,
			250,
			250
		],
		secondary: [
			245,
			245,
			245
		],
		secondaryForeground: [
			38,
			38,
			38
		],
		muted: [
			245,
			245,
			245
		],
		mutedForeground: [
			92,
			92,
			92
		],
		accent: [
			245,
			245,
			245
		],
		accentForeground: [
			38,
			38,
			38
		],
		destructive: [
			239,
			68,
			68
		],
		destructiveForeground: [
			185,
			28,
			28
		],
		success: [
			16,
			185,
			129
		],
		successForeground: [
			4,
			120,
			87
		],
		warning: [
			245,
			158,
			11
		],
		warningForeground: [
			180,
			83,
			9
		],
		info: [
			59,
			130,
			246
		],
		infoForeground: [
			29,
			78,
			216
		],
		border: [
			235,
			235,
			235
		],
		input: [
			229,
			229,
			229
		],
		ring: [
			163,
			163,
			163
		]
	},
	dark: {
		background: [
			14,
			14,
			15
		],
		foreground: [
			245,
			245,
			245
		],
		card: [
			18,
			18,
			20
		],
		cardForeground: [
			245,
			245,
			245
		],
		popover: [
			22,
			22,
			24
		],
		popoverForeground: [
			245,
			245,
			245
		],
		primary: [
			245,
			245,
			245
		],
		primaryForeground: [
			38,
			38,
			38
		],
		secondary: [
			32,
			32,
			34
		],
		secondaryForeground: [
			245,
			245,
			245
		],
		muted: [
			32,
			32,
			34
		],
		mutedForeground: [
			166,
			166,
			166
		],
		accent: [
			32,
			32,
			34
		],
		accentForeground: [
			245,
			245,
			245
		],
		destructive: [
			248,
			113,
			113
		],
		destructiveForeground: [
			248,
			113,
			113
		],
		success: [
			52,
			211,
			153
		],
		successForeground: [
			52,
			211,
			153
		],
		warning: [
			251,
			191,
			36
		],
		warningForeground: [
			251,
			191,
			36
		],
		info: [
			96,
			165,
			250
		],
		infoForeground: [
			96,
			165,
			250
		],
		border: [
			38,
			38,
			40
		],
		input: [
			46,
			46,
			49
		],
		ring: [
			115,
			115,
			115
		]
	}
};
const DEFAULT_APP_SETTINGS = {
	version: APP_SETTINGS_VERSION,
	preferences: {
		checkForUpdates: false,
		launchMode: "game",
		showGameUsernameInWindowTitle: false,
		useGameTabs: false
	},
	appearance: {
		themeMode: "dark",
		reduceMotion: "system",
		useCursorPointers: false,
		themes: {
			light: {
				tokens: DEFAULT_THEME_TOKENS.light,
				sansFont: DEFAULT_SANS_FONT,
				monoFont: DEFAULT_MONO_FONT,
				sansFontSize: DEFAULT_SANS_FONT_SIZE,
				monoFontSize: DEFAULT_MONO_FONT_SIZE,
				rounding: 1
			},
			dark: {
				tokens: DEFAULT_THEME_TOKENS.dark,
				sansFont: DEFAULT_SANS_FONT,
				monoFont: DEFAULT_MONO_FONT,
				sansFontSize: DEFAULT_SANS_FONT_SIZE,
				monoFontSize: DEFAULT_MONO_FONT_SIZE,
				rounding: 1
			}
		}
	},
	hotkeys: require_hotkeys.DEFAULT_HOTKEYS
};
const decodeAppLaunchMode = effect_Schema.decodeUnknownOption(AppLaunchModeSchema);
const decodeThemeMode = effect_Schema.decodeUnknownOption(ThemeModeSchema);
const decodeMotionMode = effect_Schema.decodeUnknownOption(MotionModeSchema);
const decodeThemeTokenName = effect_Schema.decodeUnknownOption(ThemeTokenNameSchema);
const decodeThemeRgb = effect_Schema.decodeUnknownOption(ThemeRgbSchema);
const decodeRecord = effect_Schema.decodeUnknownOption(UnknownRecordSchema);
const decodeBoolean = effect_Schema.decodeUnknownOption(effect_Schema.Boolean);
const decodeFinite = effect_Schema.decodeUnknownOption(effect_Schema.Finite);
const decodeString = effect_Schema.decodeUnknownOption(effect_Schema.String);
const decodeOrElse = (decode, value, fallback) => {
	const decoded = decode(value);
	return effect_Option.isSome(decoded) ? decoded.value : fallback;
};
const decodeRecordOrEmpty = (value) => {
	const decoded = decodeRecord(value);
	return effect_Option.isSome(decoded) ? decoded.value : {};
};
const isAppLaunchMode = (value) => effect_Option.isSome(decodeAppLaunchMode(value));
const isThemeTokenName = (value) => effect_Option.isSome(decodeThemeTokenName(value));
const normalizeRgb = (value) => {
	const stringValue = decodeString(value);
	if (effect_Option.isSome(stringValue)) {
		const match = /^#?([0-9a-f]{6})$/i.exec(stringValue.value.trim());
		if (!match) return;
		const hex = match[1];
		if (hex === void 0) return;
		return [
			Number.parseInt(hex.slice(0, 2), 16),
			Number.parseInt(hex.slice(2, 4), 16),
			Number.parseInt(hex.slice(4, 6), 16)
		];
	}
	const tupleValue = decodeThemeRgb(value);
	return effect_Option.isSome(tupleValue) ? tupleValue.value : void 0;
};
const rgbToHex = (rgb) => `#${rgb.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
const normalizeFont = (value, fallback) => {
	const decoded = decodeString(value);
	if (effect_Option.isNone(decoded)) return fallback;
	const font = decoded.value.trim();
	return font.length >= 1 && font.length <= 256 ? font : fallback;
};
const normalizeFontSize = (value, fallback) => {
	const decoded = decodeFinite(value);
	if (effect_Option.isNone(decoded)) return fallback;
	return Math.min(24, Math.max(10, Math.round(decoded.value)));
};
const normalizeRounding = (value, fallback) => {
	const decoded = decodeFinite(value);
	if (effect_Option.isNone(decoded)) return fallback;
	return Math.min(2, Math.max(0, decoded.value));
};
const normalizeThemeTokens = (value, fallback) => {
	const tokenRecord = decodeRecord(value);
	if (effect_Option.isNone(tokenRecord)) return fallback;
	const tokens = {};
	for (const [key, rawToken] of Object.entries(tokenRecord.value)) {
		if (!isThemeTokenName(key)) continue;
		const token = normalizeRgb(rawToken);
		if (token !== void 0) tokens[key] = token;
	}
	return {
		...fallback,
		...tokens
	};
};
const normalizeThemeProfile = (value, fallback) => {
	const profile = decodeRecord(value);
	if (effect_Option.isNone(profile)) return fallback;
	return {
		tokens: normalizeThemeTokens(profile.value["tokens"], fallback.tokens),
		sansFont: normalizeFont(profile.value["sansFont"], fallback.sansFont),
		monoFont: normalizeFont(profile.value["monoFont"], fallback.monoFont),
		sansFontSize: normalizeFontSize(profile.value["sansFontSize"], fallback.sansFontSize),
		monoFontSize: normalizeFontSize(profile.value["monoFontSize"], fallback.monoFontSize),
		rounding: normalizeRounding(profile.value["rounding"], fallback.rounding)
	};
};
const normalizeAppSettings = (value) => {
	const settings = decodeRecordOrEmpty(value);
	const preferences = decodeRecordOrEmpty(settings["preferences"]);
	const appearance = decodeRecordOrEmpty(settings["appearance"]);
	const themes = decodeRecordOrEmpty(appearance["themes"]);
	return {
		version: APP_SETTINGS_VERSION,
		preferences: {
			checkForUpdates: decodeOrElse(decodeBoolean, preferences["checkForUpdates"], DEFAULT_APP_SETTINGS.preferences.checkForUpdates),
			launchMode: decodeOrElse(decodeAppLaunchMode, preferences["launchMode"], DEFAULT_APP_SETTINGS.preferences.launchMode),
			showGameUsernameInWindowTitle: decodeOrElse(decodeBoolean, preferences["showGameUsernameInWindowTitle"], DEFAULT_APP_SETTINGS.preferences.showGameUsernameInWindowTitle),
			useGameTabs: decodeOrElse(decodeBoolean, preferences["useGameTabs"], DEFAULT_APP_SETTINGS.preferences.useGameTabs)
		},
		appearance: {
			themeMode: decodeOrElse(decodeThemeMode, appearance["themeMode"], DEFAULT_APP_SETTINGS.appearance.themeMode),
			reduceMotion: decodeOrElse(decodeMotionMode, appearance["reduceMotion"], DEFAULT_APP_SETTINGS.appearance.reduceMotion),
			useCursorPointers: decodeOrElse(decodeBoolean, appearance["useCursorPointers"], DEFAULT_APP_SETTINGS.appearance.useCursorPointers),
			themes: {
				light: normalizeThemeProfile(themes["light"], DEFAULT_APP_SETTINGS.appearance.themes.light),
				dark: normalizeThemeProfile(themes["dark"], DEFAULT_APP_SETTINGS.appearance.themes.dark)
			}
		},
		hotkeys: require_hotkeys.normalizeHotkeySettings(settings["hotkeys"])
	};
};
const serializeTokens = (tokens) => {
	const serialized = {};
	for (const [name, value] of Object.entries(tokens)) {
		const tokenName = name;
		const rgb = normalizeRgb(value);
		if (rgb !== void 0) serialized[tokenName] = rgbToHex(rgb);
	}
	return serialized;
};
const serializeThemeProfile = (profile) => ({
	tokens: serializeTokens(profile.tokens),
	sansFont: profile.sansFont,
	monoFont: profile.monoFont,
	sansFontSize: profile.sansFontSize,
	monoFontSize: profile.monoFontSize,
	rounding: profile.rounding
});
const serializeAppSettings = (settings) => {
	const normalized = normalizeAppSettings(settings);
	return {
		version: normalized.version,
		preferences: normalized.preferences,
		appearance: {
			themeMode: normalized.appearance.themeMode,
			reduceMotion: normalized.appearance.reduceMotion,
			useCursorPointers: normalized.appearance.useCursorPointers,
			themes: {
				light: serializeThemeProfile(normalized.appearance.themes.light),
				dark: serializeThemeProfile(normalized.appearance.themes.dark)
			}
		},
		hotkeys: normalized.hotkeys
	};
};
exports.AppLaunchModeSchema = AppLaunchModeSchema;
exports.AppSettingsSchema = AppSettingsSchema;
exports.AppearancePatchSchema = AppearancePatchSchema;
exports.DEFAULT_APP_SETTINGS = DEFAULT_APP_SETTINGS;
exports.MotionModeSchema = MotionModeSchema;
exports.PreferencesPatchSchema = PreferencesPatchSchema;
exports.THEME_FONT_MAX_LENGTH = THEME_FONT_MAX_LENGTH;
exports.THEME_FONT_MIN_LENGTH = THEME_FONT_MIN_LENGTH;
exports.THEME_FONT_SIZE_MAX = THEME_FONT_SIZE_MAX;
exports.THEME_FONT_SIZE_MIN = THEME_FONT_SIZE_MIN;
exports.THEME_ROUNDING_MAX = THEME_ROUNDING_MAX;
exports.THEME_ROUNDING_MIN = THEME_ROUNDING_MIN;
exports.THEME_TOKEN_NAMES = THEME_TOKEN_NAMES;
exports.ThemeFontSchema = ThemeFontSchema;
exports.ThemeFontSizeSchema = ThemeFontSizeSchema;
exports.ThemeModeSchema = ThemeModeSchema;
exports.ThemeProfilePatchSchema = ThemeProfilePatchSchema;
exports.ThemeProfileSchema = ThemeProfileSchema;
exports.ThemeRgbSchema = ThemeRgbSchema;
exports.ThemeRoundingSchema = ThemeRoundingSchema;
exports.ThemeTokenNameSchema = ThemeTokenNameSchema;
exports.ThemeTokenValuesSchema = ThemeTokenValuesSchema;
exports.ThemeVariantSchema = ThemeVariantSchema;
exports.UnknownRecordSchema = UnknownRecordSchema;
exports.isAppLaunchMode = isAppLaunchMode;
exports.normalizeAppSettings = normalizeAppSettings;
exports.serializeAppSettings = serializeAppSettings;
