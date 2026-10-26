Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_settings = require("./settings.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/appearance.ts
const AppearanceSnapshotSchema = effect_Schema.Struct({
	backgroundColor: effect_Schema.String,
	monoFont: require_settings.ThemeFontSchema,
	monoFontSize: require_settings.ThemeFontSizeSchema,
	reduceMotion: require_settings.MotionModeSchema,
	rounding: require_settings.ThemeRoundingSchema,
	sansFont: require_settings.ThemeFontSchema,
	sansFontSize: require_settings.ThemeFontSizeSchema,
	tokens: require_settings.ThemeTokenValuesSchema,
	useCursorPointers: effect_Schema.Boolean,
	variant: require_settings.ThemeVariantSchema
});
const APPEARANCE_SNAPSHOT_ARGUMENT = "--lucent__appearance";
const SETTINGS_SNAPSHOT_ARGUMENT = "--lucent__settings";
const decodeAppearanceSnapshot = effect_Schema.decodeUnknownOption(AppearanceSnapshotSchema);
const decodeUnknownRecord = effect_Schema.decodeUnknownOption(require_settings.UnknownRecordSchema);
const textSizeRatios = {
	"--text-2xs": 10 / 14,
	"--text-xs": 11 / 14,
	"--text-sm": 12 / 14,
	"--text-base": 1,
	"--text-md": 1,
	"--text-lg": 15 / 14,
	"--text-xl": 16 / 14,
	"--text-2xl": 18 / 14,
	"--text-3xl": 20 / 14,
	"--text-4xl": 24 / 14,
	"--text-5xl": 2
};
const HEX_RGB_PATTERN = /^#?([0-9a-f]{6})$/i;
const toHexPair = (value) => Math.max(0, Math.min(255, value)).toString(16).padStart(2, "0");
const rgbEquals = (left, right) => left[0] === right[0] && left[1] === right[1] && left[2] === right[2];
const rgbToCssValue = (rgb) => rgb.join(", ");
const rgbToHex = (rgb) => `#${toHexPair(rgb[0])}${toHexPair(rgb[1])}${toHexPair(rgb[2])}`;
const hexToRgb = (hex) => {
	const value = HEX_RGB_PATTERN.exec(hex.trim())?.[1];
	if (value === void 0) return null;
	return [
		Number.parseInt(value.slice(0, 2), 16),
		Number.parseInt(value.slice(2, 4), 16),
		Number.parseInt(value.slice(4, 6), 16)
	];
};
const getTextSizeTokens = (baseSize) => {
	const tokens = {};
	for (const [name, ratio] of Object.entries(textSizeRatios)) tokens[name] = `${Number((baseSize * ratio).toFixed(4))}px`;
	return tokens;
};
const resolveThemeVariant = (settings, systemPrefersDark) => {
	const mode = settings.appearance.themeMode;
	if (mode === "light" || mode === "dark") return mode;
	return systemPrefersDark ? "dark" : "light";
};
const resolveThemeTokens = (settings, variant) => ({
	...require_settings.DEFAULT_APP_SETTINGS.appearance.themes[variant].tokens,
	...settings.appearance.themes[variant].tokens
});
const createAppearanceSnapshot = (settings, systemPrefersDark) => {
	const variant = resolveThemeVariant(settings, systemPrefersDark);
	const profile = settings.appearance.themes[variant];
	const tokens = resolveThemeTokens(settings, variant);
	return {
		backgroundColor: rgbToHex(tokens.background),
		monoFont: profile.monoFont,
		monoFontSize: profile.monoFontSize,
		reduceMotion: settings.appearance.reduceMotion,
		rounding: profile.rounding,
		sansFont: profile.sansFont,
		sansFontSize: profile.sansFontSize,
		tokens,
		useCursorPointers: settings.appearance.useCursorPointers,
		variant
	};
};
const parseSerializedJson = (value) => {
	try {
		return JSON.parse(decodeURIComponent(value));
	} catch {
		return null;
	}
};
const readArgumentValue = (argv, name) => {
	const prefix = `${name}=`;
	const value = argv.find((argument) => argument.startsWith(prefix));
	return value === void 0 ? null : value.slice(prefix.length);
};
const serializeAppearanceSnapshotArgument = (snapshot) => `${APPEARANCE_SNAPSHOT_ARGUMENT}=${encodeURIComponent(JSON.stringify(snapshot))}`;
const readAppearanceSnapshotArgument = (argv) => {
	const value = readArgumentValue(argv, APPEARANCE_SNAPSHOT_ARGUMENT);
	if (value === null) return null;
	const parsed = parseSerializedJson(value);
	if (parsed === null) return null;
	const decoded = decodeAppearanceSnapshot(parsed);
	return effect_Option.isSome(decoded) ? decoded.value : null;
};
const serializeSettingsSnapshotArgument = (settings) => `${SETTINGS_SNAPSHOT_ARGUMENT}=${encodeURIComponent(JSON.stringify(settings))}`;
const readSettingsSnapshotArgument = (argv) => {
	const value = readArgumentValue(argv, SETTINGS_SNAPSHOT_ARGUMENT);
	if (value === null) return null;
	const parsed = parseSerializedJson(value);
	if (parsed === null) return null;
	const decoded = decodeUnknownRecord(parsed);
	return effect_Option.isSome(decoded) ? require_settings.normalizeAppSettings(decoded.value) : null;
};
exports.APPEARANCE_SNAPSHOT_ARGUMENT = APPEARANCE_SNAPSHOT_ARGUMENT;
exports.AppearanceSnapshotSchema = AppearanceSnapshotSchema;
exports.SETTINGS_SNAPSHOT_ARGUMENT = SETTINGS_SNAPSHOT_ARGUMENT;
exports.createAppearanceSnapshot = createAppearanceSnapshot;
exports.getTextSizeTokens = getTextSizeTokens;
exports.hexToRgb = hexToRgb;
exports.readAppearanceSnapshotArgument = readAppearanceSnapshotArgument;
exports.readSettingsSnapshotArgument = readSettingsSnapshotArgument;
exports.resolveThemeTokens = resolveThemeTokens;
exports.resolveThemeVariant = resolveThemeVariant;
exports.rgbEquals = rgbEquals;
exports.rgbToCssValue = rgbToCssValue;
exports.rgbToHex = rgbToHex;
exports.serializeAppearanceSnapshotArgument = serializeAppearanceSnapshotArgument;
exports.serializeSettingsSnapshotArgument = serializeSettingsSnapshotArgument;
