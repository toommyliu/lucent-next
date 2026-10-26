import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
// src/hotkeys.ts
const SETTINGS_COMMAND_IDS = [
	"toggleTopBar",
	"loadScript",
	"toggleScript",
	"toggleScriptsDialog",
	"toggleOptionsMenu",
	"openEnvironment",
	"openLoaderGrabber",
	"openFollower",
	"openPackets",
	"toggleAutoattack",
	"toggleFollower",
	"toggleBank",
	"toggleInfiniteRange",
	"toggleProvokeCell",
	"toggleEnemyMagnet",
	"toggleInterfaceOnlyRendering",
	"toggleMinimalRendering",
	"toggleHidePlayers",
	"toggleSkipCutscenes",
	"toggleAntiCounter",
	"toggleAnimations",
	"toggleCollisions",
	"toggleDeathAds"
];
const SETTING_COMMAND_CATEGORIES = [
	"General",
	"Scripts",
	"Options",
	"Windows"
];
const SettingsCommandIdSchema = Schema.Literals(SETTINGS_COMMAND_IDS);
const HotkeyBindingSchema = Schema.Struct({
	id: SettingsCommandIdSchema,
	value: Schema.String
});
const HotkeyBindingPatchSchema = Schema.Struct({
	id: SettingsCommandIdSchema,
	value: Schema.NullOr(Schema.String)
});
const HotkeysSettingsSchema = Schema.Struct({ bindings: Schema.Array(HotkeyBindingSchema) });
const HotkeysPatchSchema = Schema.Struct({ bindings: Schema.optionalKey(Schema.Array(HotkeyBindingPatchSchema)) });
const PersistedHotkeySettingsSchema = Schema.Struct({ bindings: Schema.Array(Schema.Struct({
	id: Schema.String,
	value: Schema.String
})) });
const decodePersistedHotkeySettings = Schema.decodeUnknownOption(PersistedHotkeySettingsSchema);
const SETTINGS_COMMANDS = [
	{
		id: "toggleTopBar",
		category: "General",
		label: "Toggle Top Bar",
		defaultHotkey: "Mod+Shift+T"
	},
	{
		id: "loadScript",
		category: "Scripts",
		label: "Load Script",
		defaultHotkey: "Mod+O"
	},
	{
		id: "toggleScript",
		category: "Scripts",
		label: "Toggle Script",
		defaultHotkey: "Mod+Shift+X"
	},
	{
		id: "toggleScriptsDialog",
		category: "Scripts",
		label: "Toggle Scripts Dialog",
		defaultHotkey: "Mod+Shift+S"
	},
	{
		id: "toggleOptionsMenu",
		category: "Options",
		label: "Toggle Options Menu",
		defaultHotkey: "Mod+Shift+,"
	},
	{
		id: "openEnvironment",
		category: "Windows",
		label: "Open Environment",
		defaultHotkey: "Alt+E"
	},
	{
		id: "openLoaderGrabber",
		category: "Windows",
		label: "Open Loader/Grabber",
		defaultHotkey: ""
	},
	{
		id: "openFollower",
		category: "Windows",
		label: "Open Follower",
		defaultHotkey: "Alt+F"
	},
	{
		id: "openPackets",
		category: "Windows",
		label: "Open Packets",
		defaultHotkey: ""
	},
	{
		id: "toggleAutoattack",
		category: "General",
		label: "Toggle Auto Attack",
		defaultHotkey: "Alt+A"
	},
	{
		id: "toggleFollower",
		category: "General",
		label: "Toggle Follower",
		defaultHotkey: "Alt+Shift+F"
	},
	{
		id: "toggleBank",
		category: "General",
		label: "Toggle Bank",
		defaultHotkey: "Mod+B"
	},
	{
		id: "toggleInfiniteRange",
		category: "Options",
		label: "Toggle Infinite Range",
		defaultHotkey: "Alt+I"
	},
	{
		id: "toggleProvokeCell",
		category: "Options",
		label: "Toggle Provoke Cell",
		defaultHotkey: ""
	},
	{
		id: "toggleEnemyMagnet",
		category: "Options",
		label: "Toggle Enemy Magnet",
		defaultHotkey: ""
	},
	{
		id: "toggleInterfaceOnlyRendering",
		category: "Options",
		group: "Rendering Mode",
		label: "Toggle Interface-Only Rendering",
		defaultHotkey: "Alt+L"
	},
	{
		id: "toggleMinimalRendering",
		category: "Options",
		group: "Rendering Mode",
		label: "Toggle Minimal Rendering",
		defaultHotkey: "Alt+Shift+L"
	},
	{
		id: "toggleHidePlayers",
		category: "Options",
		label: "Toggle Hide Players",
		defaultHotkey: ""
	},
	{
		id: "toggleSkipCutscenes",
		category: "Options",
		label: "Toggle Skip Cutscenes",
		defaultHotkey: ""
	},
	{
		id: "toggleAntiCounter",
		category: "Options",
		label: "Toggle Anti-Counter",
		defaultHotkey: ""
	},
	{
		id: "toggleAnimations",
		category: "Options",
		label: "Toggle Animations",
		defaultHotkey: ""
	},
	{
		id: "toggleCollisions",
		category: "Options",
		label: "Toggle Collisions",
		defaultHotkey: ""
	},
	{
		id: "toggleDeathAds",
		category: "Options",
		label: "Toggle Death Ads",
		defaultHotkey: ""
	}
];
const commandIds = new Set(SETTINGS_COMMAND_IDS);
const commandDefinitions = new Map(SETTINGS_COMMANDS.map((command) => [command.id, command]));
const modifierAliases = /* @__PURE__ */ new Map([
	["alt", "Alt"],
	["option", "Alt"],
	["control", "Control"],
	["ctrl", "Control"],
	["command", "Meta"],
	["cmd", "Meta"],
	["meta", "Meta"],
	["mod", "Mod"],
	["shift", "Shift"]
]);
const modifierOrder = /* @__PURE__ */ new Map([
	["Mod", 0],
	["Control", 1],
	["Alt", 2],
	["Shift", 3],
	["Meta", 4]
]);
const normalizeKey = (value) => {
	const trimmed = value.trim();
	if (trimmed.length === 0) return null;
	const alias = modifierAliases.get(trimmed.toLowerCase());
	if (alias !== void 0) return alias;
	return /^[a-z]$/i.test(trimmed) ? trimmed.toUpperCase() : trimmed;
};
const isSettingsCommandId = (value) => typeof value === "string" && commandIds.has(value);
const getSettingsCommandDefinition = (id) => {
	const definition = commandDefinitions.get(id);
	if (definition === void 0) throw new Error(`Unknown settings command: ${id}`);
	return definition;
};
const createDefaultHotkeyBindings = () => SETTINGS_COMMANDS.map((command) => ({
	id: command.id,
	value: command.defaultHotkey
}));
const DEFAULT_HOTKEYS = { bindings: createDefaultHotkeyBindings() };
const normalizeHotkeyBindingValue = (value) => {
	if (typeof value !== "string") return null;
	const trimmed = value.trim();
	if (trimmed.length === 0) return "";
	const parts = (trimmed.includes("+") ? trimmed.split("+") : trimmed.split(/\s+/)).map(normalizeKey).filter((part) => part !== null);
	if (parts.length === 0) return null;
	if (new Set(parts.map((part) => part.toLowerCase())).size !== parts.length) return null;
	const nonModifiers = parts.filter((part) => !modifierOrder.has(part));
	if (nonModifiers.length !== 1) return null;
	return [...[...modifierOrder.keys()].filter((modifier) => parts.includes(modifier)), nonModifiers[0]].join("+");
};
const resolveHotkeyModifier = (part, platform) => {
	if (part !== "Mod") return part;
	return platform === "mac" ? "Meta" : "Control";
};
const hotkeyBindingMatchKey = (value, platform) => {
	const normalized = normalizeHotkeyBindingValue(value);
	if (normalized === null || normalized.length === 0) return null;
	const resolved = normalized.split("+").map((part) => resolveHotkeyModifier(part, platform)).join("+");
	const normalizedResolved = normalizeHotkeyBindingValue(resolved);
	if (normalizedResolved === null || normalizedResolved.length === 0) return null;
	return normalizedResolved.toLowerCase();
};
const normalizeHotkeySettings = (value) => {
	const defaultValues = new Map(DEFAULT_HOTKEYS.bindings.map((binding) => [binding.id, binding.value]));
	const decoded = decodePersistedHotkeySettings(value);
	if (Option.isSome(decoded)) for (const binding of decoded.value.bindings) {
		if (!isSettingsCommandId(binding.id)) continue;
		const normalizedValue = normalizeHotkeyBindingValue(binding.value);
		if (normalizedValue !== null) defaultValues.set(binding.id, normalizedValue);
	}
	return { bindings: SETTINGS_COMMANDS.map((command) => ({
		id: command.id,
		value: defaultValues.get(command.id) ?? command.defaultHotkey
	})) };
};
const readHotkeyBinding = (bindings, id) => bindings.find((binding) => binding.id === id)?.value ?? getSettingsCommandDefinition(id).defaultHotkey;
const hotkeyConflictKey = (value) => {
	const normalized = normalizeHotkeyBindingValue(value);
	return normalized === null || normalized.length === 0 ? null : normalized.toLowerCase();
};
const findDuplicateHotkeyBinding = (bindings) => {
	const seen = /* @__PURE__ */ new Set();
	for (const binding of bindings) {
		const key = hotkeyConflictKey(binding.value);
		if (key === null) continue;
		if (seen.has(key)) return binding;
		seen.add(key);
	}
	return null;
};
const macDisplayAliases = {
	alt: "⌥",
	cmd: "⌘",
	command: "⌘",
	control: "⌃",
	ctrl: "⌃",
	meta: "⌘",
	mod: "⌘",
	option: "⌥",
	shift: "⇧",
	"⌃": "⌃",
	"⌘": "⌘",
	"⌥": "⌥",
	"⇧": "⇧"
};
const nonMacDisplayAliases = {
	alt: "Alt",
	control: "Ctrl",
	ctrl: "Ctrl",
	meta: "Win",
	mod: "Ctrl",
	option: "Alt",
	shift: "Shift"
};
const splitHotkeyParts = (value) => {
	const trimmed = value.trim();
	return trimmed.includes("+") ? trimmed.split("+") : trimmed.split(/\s+/);
};
const formatHotkeyPart = (part, platform) => {
	const trimmed = part.trim();
	const alias = (platform === "mac" ? macDisplayAliases : nonMacDisplayAliases)[trimmed.toLowerCase()];
	if (alias !== void 0) return alias;
	return /^[a-z]$/i.test(trimmed) ? trimmed.toUpperCase() : trimmed;
};
const formatHotkeyDisplayParts = (value, platform, emptyLabel = "Unbound") => {
	if (value.length === 0) return [emptyLabel];
	return splitHotkeyParts(value).map((part) => part.trim()).filter((part) => part.length > 0).map((part) => formatHotkeyPart(part, platform));
};
const formatHotkeyDisplay = (value, platform, emptyLabel = "Unbound") => {
	if (value.length === 0) return emptyLabel;
	const separator = platform === "mac" ? " " : "+";
	return formatHotkeyDisplayParts(value, platform, emptyLabel).join(separator);
};
export { DEFAULT_HOTKEYS, HotkeyBindingPatchSchema, HotkeyBindingSchema, HotkeysPatchSchema, HotkeysSettingsSchema, SETTINGS_COMMANDS, SETTINGS_COMMAND_IDS, SETTING_COMMAND_CATEGORIES, SettingsCommandIdSchema, createDefaultHotkeyBindings, findDuplicateHotkeyBinding, formatHotkeyDisplay, formatHotkeyDisplayParts, getSettingsCommandDefinition, hotkeyBindingMatchKey, hotkeyConflictKey, isSettingsCommandId, normalizeHotkeyBindingValue, normalizeHotkeySettings, readHotkeyBinding };
