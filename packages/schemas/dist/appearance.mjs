import { DEFAULT_APP_SETTINGS, MotionModeSchema, ThemeFontSchema, ThemeFontSizeSchema, ThemeRoundingSchema, ThemeTokenValuesSchema, ThemeVariantSchema, UnknownRecordSchema, normalizeAppSettings } from "./settings.mjs";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
// src/appearance.ts
const AppearanceSnapshotSchema = Schema.Struct({
	backgroundColor: Schema.String,
	monoFont: ThemeFontSchema,
	monoFontSize: ThemeFontSizeSchema,
	reduceMotion: MotionModeSchema,
	rounding: ThemeRoundingSchema,
	sansFont: ThemeFontSchema,
	sansFontSize: ThemeFontSizeSchema,
	tokens: ThemeTokenValuesSchema,
	useCursorPointers: Schema.Boolean,
	variant: ThemeVariantSchema
});
const APPEARANCE_SNAPSHOT_ARGUMENT = "--lucent__appearance";
const SETTINGS_SNAPSHOT_ARGUMENT = "--lucent__settings";
const decodeAppearanceSnapshot = Schema.decodeUnknownOption(AppearanceSnapshotSchema);
const decodeUnknownRecord = Schema.decodeUnknownOption(UnknownRecordSchema);
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
	...DEFAULT_APP_SETTINGS.appearance.themes[variant].tokens,
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
	return Option.isSome(decoded) ? decoded.value : null;
};
const serializeSettingsSnapshotArgument = (settings) => `${SETTINGS_SNAPSHOT_ARGUMENT}=${encodeURIComponent(JSON.stringify(settings))}`;
const readSettingsSnapshotArgument = (argv) => {
	const value = readArgumentValue(argv, SETTINGS_SNAPSHOT_ARGUMENT);
	if (value === null) return null;
	const parsed = parseSerializedJson(value);
	if (parsed === null) return null;
	const decoded = decodeUnknownRecord(parsed);
	return Option.isSome(decoded) ? normalizeAppSettings(decoded.value) : null;
};
export { APPEARANCE_SNAPSHOT_ARGUMENT, AppearanceSnapshotSchema, SETTINGS_SNAPSHOT_ARGUMENT, createAppearanceSnapshot, getTextSizeTokens, hexToRgb, readAppearanceSnapshotArgument, readSettingsSnapshotArgument, resolveThemeTokens, resolveThemeVariant, rgbEquals, rgbToCssValue, rgbToHex, serializeAppearanceSnapshotArgument, serializeSettingsSnapshotArgument };
