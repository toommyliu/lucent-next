Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_baseSchemas = require("./baseSchemas.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/combatProfiles.ts
const COMBAT_PROFILE_LIBRARY_VERSION = 1;
const DEFAULT_COMBAT_PROFILE_ID = "generic-base";
const DEFAULT_COMBAT_PROFILE_DELAY_MS = 150;
const MAX_DELAY_MS = 6e4;
const MAX_WAIT_MS = 6e4;
const MAX_ID_LENGTH = 80;
const MAX_LABEL_LENGTH = 80;
const MAX_CLASS_NAME_LENGTH = 80;
const MAX_CONSUMABLE_NAME_LENGTH = 80;
const MAX_AURA_NAME_LENGTH = 80;
const MAX_MESSAGE_TRIGGER_TEXT_LENGTH = 160;
const structuredCloneValue = globalThis.structuredClone;
const SkillSlotSchema = effect_Schema.Literals([
	0,
	1,
	2,
	3,
	4,
	5
]);
const CombatProfileCooldownModeSchema = effect_Schema.Literals(["use-if-ready", "wait-for-cooldown"]);
const CombatProfileThresholdUnitSchema = effect_Schema.Literals(["percent", "value"]);
const CombatProfileComparisonSchema = effect_Schema.Literals(["<=", ">="]);
const CombatProfileStatConditionSchema = effect_Schema.Struct({
	type: effect_Schema.Literals([
		"self-hp",
		"self-mp",
		"ally-hp"
	]),
	op: CombatProfileComparisonSchema,
	value: require_baseSchemas.boundedInt(0, 999999),
	unit: CombatProfileThresholdUnitSchema
});
const CombatProfileAuraConditionSchema = effect_Schema.Struct({
	type: effect_Schema.Literals(["self-aura", "target-aura"]),
	auraName: require_baseSchemas.TrimmedNonEmptyString,
	op: CombatProfileComparisonSchema,
	value: require_baseSchemas.boundedInt(0, 999)
});
const CombatProfileConditionSchema = effect_Schema.Union([CombatProfileStatConditionSchema, CombatProfileAuraConditionSchema]);
const CombatProfileStepSchema = effect_Schema.Struct({
	skill: SkillSlotSchema,
	conditions: effect_Schema.Array(CombatProfileConditionSchema),
	priority: effect_Schema.optionalKey(effect_Schema.Boolean),
	cooldownMode: effect_Schema.optionalKey(CombatProfileCooldownModeSchema),
	waitMs: effect_Schema.optionalKey(require_baseSchemas.boundedInt(0, MAX_WAIT_MS))
});
const CombatProfileMessageTriggerSourceSchema = effect_Schema.Literals([
	"any",
	"animation",
	"aura"
]);
const CombatProfileMessageTriggerSchema = effect_Schema.Struct({
	messageIncludes: require_baseSchemas.TrimmedNonEmptyString,
	skill: SkillSlotSchema,
	source: CombatProfileMessageTriggerSourceSchema,
	cooldownMs: effect_Schema.optionalKey(require_baseSchemas.boundedInt(0, MAX_WAIT_MS))
});
const CombatProfileSchema = effect_Schema.Struct({
	id: require_baseSchemas.TrimmedNonEmptyString,
	label: require_baseSchemas.TrimmedNonEmptyString,
	classNames: effect_Schema.optionalKey(effect_Schema.Array(require_baseSchemas.TrimmedNonEmptyString)),
	consumable: effect_Schema.optionalKey(require_baseSchemas.TrimmedNonEmptyString),
	delayMs: require_baseSchemas.boundedInt(0, MAX_DELAY_MS),
	cooldownMode: CombatProfileCooldownModeSchema,
	resetSkillIndexOnTargetDeath: effect_Schema.optionalKey(effect_Schema.Boolean),
	steps: effect_Schema.Array(CombatProfileStepSchema),
	messageTriggers: effect_Schema.optionalKey(effect_Schema.Array(CombatProfileMessageTriggerSchema))
});
const CombatProfileLibrarySchema = effect_Schema.Struct({
	version: effect_Schema.Literal(1),
	profiles: effect_Schema.Array(CombatProfileSchema)
});
var CombatProfileNormalizationError = class extends Error {
	_tag = "CombatProfileNormalizationError";
	constructor(message) {
		super(message);
		this.name = "CombatProfileNormalizationError";
	}
};
const UnknownRecordSchema = effect_Schema.Record(effect_Schema.String, effect_Schema.Unknown);
const CombatProfileDefinitionInputSchema = effect_Schema.Struct({ steps: effect_Schema.Array(effect_Schema.Unknown) });
const decodeArray = effect_Schema.decodeUnknownOption(effect_Schema.Array(effect_Schema.Unknown));
const decodeBoolean = effect_Schema.decodeUnknownOption(effect_Schema.Boolean);
const decodeCombatProfileComparison = effect_Schema.decodeUnknownOption(CombatProfileComparisonSchema);
const decodeCombatProfileCooldownMode = effect_Schema.decodeUnknownOption(CombatProfileCooldownModeSchema);
const decodeCombatProfileDefinitionInput = effect_Schema.decodeUnknownOption(CombatProfileDefinitionInputSchema);
const decodeCombatProfileMessageTriggerSource = effect_Schema.decodeUnknownOption(CombatProfileMessageTriggerSourceSchema);
const decodeCombatProfileThresholdUnit = effect_Schema.decodeUnknownOption(CombatProfileThresholdUnitSchema);
const decodeFinite = effect_Schema.decodeUnknownOption(effect_Schema.Finite);
const decodeRecord = effect_Schema.decodeUnknownOption(UnknownRecordSchema);
const decodeString = effect_Schema.decodeUnknownOption(effect_Schema.String);
const decodeSkillSlot = effect_Schema.decodeUnknownOption(SkillSlotSchema);
const fromOption = (decode, value) => {
	const decoded = decode(value);
	return effect_Option.isSome(decoded) ? decoded.value : void 0;
};
const asRecord = (value) => fromOption(decodeRecord, value);
const asArray = (value) => fromOption(decodeArray, value) ?? [];
const normalizeArray = (value, normalize) => asArray(value).map(normalize).filter((value) => value !== void 0);
const clampInt = (value, fallback, min, max) => {
	const parsed = fromOption(decodeFinite, value);
	return parsed === void 0 ? fallback : Math.min(max, Math.max(min, Math.trunc(parsed)));
};
const truncateUtf16 = (value, maxLength) => {
	const truncated = value.slice(0, maxLength);
	const last = truncated.charCodeAt(truncated.length - 1);
	return last >= 55296 && last <= 56319 ? truncated.slice(0, -1) : truncated;
};
const trimString = (value, maxLength) => {
	const decoded = fromOption(decodeString, value);
	if (decoded === void 0) return;
	const trimmed = decoded.trim();
	const truncated = truncateUtf16(trimmed, maxLength);
	return truncated === "" ? void 0 : truncated;
};
const appendIdSuffix = (id, suffix) => {
	const suffixText = `-${suffix}`;
	return `${truncateUtf16(id, MAX_ID_LENGTH - suffixText.length)}${suffixText}`;
};
const ensureUniqueIds = (values) => {
	const reservedIds = new Set(values.map((value) => value.id));
	const usedIds = /* @__PURE__ */ new Set();
	const nextSuffixes = /* @__PURE__ */ new Map();
	return values.map((value) => {
		if (!usedIds.has(value.id)) {
			usedIds.add(value.id);
			return value;
		}
		let suffix = nextSuffixes.get(value.id) ?? 2;
		let id = appendIdSuffix(value.id, suffix);
		while (reservedIds.has(id) || usedIds.has(id)) {
			suffix += 1;
			id = appendIdSuffix(value.id, suffix);
		}
		nextSuffixes.set(value.id, suffix + 1);
		usedIds.add(id);
		return {
			...value,
			id
		};
	});
};
const genericProfile = () => ({
	id: DEFAULT_COMBAT_PROFILE_ID,
	label: "Generic",
	delayMs: 150,
	cooldownMode: "use-if-ready",
	steps: [
		1,
		2,
		3,
		4
	].map((skill) => ({
		skill,
		conditions: []
	}))
});
const normalizeCondition = (value) => {
	const record = asRecord(value);
	if (record === void 0) return;
	const type = record["type"];
	if (type === "self-hp" || type === "self-mp" || type === "ally-hp") {
		const unit = fromOption(decodeCombatProfileThresholdUnit, record["unit"]) ?? "percent";
		return {
			type,
			op: fromOption(decodeCombatProfileComparison, record["op"]) ?? "<=",
			value: clampInt(record["value"], 0, 0, unit === "percent" ? 100 : 999999),
			unit
		};
	}
	if (type !== "self-aura" && type !== "target-aura") return;
	const auraName = trimString(record["auraName"], MAX_AURA_NAME_LENGTH);
	return auraName === void 0 ? void 0 : {
		type,
		auraName,
		op: fromOption(decodeCombatProfileComparison, record["op"]) ?? "<=",
		value: clampInt(record["value"], 0, 0, 999)
	};
};
const normalizeStep = (value) => {
	const record = asRecord(value);
	if (record === void 0) return;
	const skill = clampInt(record["skill"], NaN, 0, 5);
	const skillSlot = fromOption(decodeSkillSlot, skill);
	if (skillSlot === void 0) return;
	const cooldownMode = fromOption(decodeCombatProfileCooldownMode, record["cooldownMode"]);
	const waitMs = clampInt(record["waitMs"], 0, 0, MAX_WAIT_MS);
	return {
		skill: skillSlot,
		conditions: normalizeArray(record["conditions"], normalizeCondition),
		...fromOption(decodeBoolean, record["priority"]) ? { priority: true } : {},
		...cooldownMode === void 0 ? {} : { cooldownMode },
		...waitMs > 0 ? { waitMs } : {}
	};
};
const normalizeMessageTrigger = (value) => {
	const record = asRecord(value);
	if (record === void 0) return;
	const messageIncludes = trimString(record["messageIncludes"], MAX_MESSAGE_TRIGGER_TEXT_LENGTH);
	const skill = clampInt(record["skill"], NaN, 0, 5);
	const skillSlot = fromOption(decodeSkillSlot, skill);
	if (messageIncludes === void 0 || skillSlot === void 0) return;
	const cooldownMs = clampInt(record["cooldownMs"], 0, 0, MAX_WAIT_MS);
	return {
		messageIncludes,
		skill: skillSlot,
		source: fromOption(decodeCombatProfileMessageTriggerSource, record["source"]) ?? "any",
		...cooldownMs > 0 ? { cooldownMs } : {}
	};
};
const normalizeProfile = (value, fallbackId) => {
	const record = asRecord(value);
	if (record === void 0) return;
	const id = trimString(record["id"], MAX_ID_LENGTH) ?? fallbackId;
	if (id === void 0) return;
	const label = trimString(record["label"], MAX_LABEL_LENGTH) ?? "Profile";
	const classNames = normalizeArray(record["classNames"], (className) => trimString(className, MAX_CLASS_NAME_LENGTH));
	const consumable = trimString(record["consumable"], MAX_CONSUMABLE_NAME_LENGTH);
	const messageTriggers = normalizeArray(record["messageTriggers"], normalizeMessageTrigger);
	const steps = normalizeArray(record["steps"], normalizeStep);
	return {
		id,
		label,
		...classNames.length === 0 ? {} : { classNames },
		...consumable === void 0 ? {} : { consumable },
		delayMs: clampInt(record["delayMs"], 150, 0, MAX_DELAY_MS),
		cooldownMode: fromOption(decodeCombatProfileCooldownMode, record["cooldownMode"]) ?? "use-if-ready",
		...fromOption(decodeBoolean, record["resetSkillIndexOnTargetDeath"]) ? { resetSkillIndexOnTargetDeath: true } : {},
		steps,
		...messageTriggers.length === 0 ? {} : { messageTriggers }
	};
};
const normalizeProfiles = (value) => {
	const profiles = [...normalizeArray(value, (profile) => normalizeProfile(profile))];
	const genericIndex = profiles.findIndex((profile) => profile.id === DEFAULT_COMBAT_PROFILE_ID);
	if (genericIndex === -1) profiles.unshift(genericProfile());
	else if (genericIndex > 0) {
		const [generic] = profiles.splice(genericIndex, 1);
		profiles.unshift(generic);
	}
	return ensureUniqueIds(profiles);
};
const assertSupportedLibraryVersion = (version) => {
	const parsed = fromOption(decodeFinite, version);
	if (parsed !== void 0 && Number.isInteger(parsed) && parsed > 1) throw new CombatProfileNormalizationError(`Unsupported combat profile library version ${parsed}`);
};
const DEFAULT_COMBAT_PROFILE_LIBRARY = {
	version: 1,
	profiles: [genericProfile()]
};
const cloneCombatProfileLibrary = (library) => structuredCloneValue(library);
const normalizeCombatProfile = (value) => normalizeProfile(value, "generic-base") ?? genericProfile();
const isCombatProfileDefinition = (value) => effect_Option.isSome(decodeCombatProfileDefinitionInput(value));
const normalizeCombatProfileLibrary = (value) => {
	const source = asRecord(value);
	assertSupportedLibraryVersion(source?.["version"]);
	return {
		version: 1,
		profiles: normalizeProfiles(source?.["profiles"])
	};
};
const findCombatProfileById = (library, profileId) => library.profiles.find((profile) => profile.id === profileId);
const getDuplicateCombatProfileLabel = (label, profiles) => {
	const labels = new Set(profiles.map((profile) => profile.label));
	const copyLabel = `${label} Copy`;
	if (!labels.has(copyLabel)) return copyLabel;
	let index = 2;
	while (labels.has(`${copyLabel} ${index}`)) index += 1;
	return `${copyLabel} ${index}`;
};
const duplicateCombatProfile = (profile, profiles, createId) => ({
	...profile,
	id: createId(),
	label: getDuplicateCombatProfileLabel(profile.label, profiles),
	steps: profile.steps.map((step) => ({
		...step,
		conditions: step.conditions.map((condition) => ({ ...condition }))
	})),
	...profile.messageTriggers === void 0 ? {} : { messageTriggers: profile.messageTriggers.map((trigger) => ({ ...trigger })) }
});
const getCombatProfileById = (library, profileId) => findCombatProfileById(library, profileId) ?? findCombatProfileById(library, "generic-base") ?? genericProfile();
const serializeCombatProfileLibrary = (library) => normalizeCombatProfileLibrary(library);
exports.COMBAT_PROFILE_LIBRARY_VERSION = COMBAT_PROFILE_LIBRARY_VERSION;
exports.CombatProfileAuraConditionSchema = CombatProfileAuraConditionSchema;
exports.CombatProfileComparisonSchema = CombatProfileComparisonSchema;
exports.CombatProfileConditionSchema = CombatProfileConditionSchema;
exports.CombatProfileCooldownModeSchema = CombatProfileCooldownModeSchema;
exports.CombatProfileLibrarySchema = CombatProfileLibrarySchema;
exports.CombatProfileMessageTriggerSchema = CombatProfileMessageTriggerSchema;
exports.CombatProfileMessageTriggerSourceSchema = CombatProfileMessageTriggerSourceSchema;
exports.CombatProfileNormalizationError = CombatProfileNormalizationError;
exports.CombatProfileSchema = CombatProfileSchema;
exports.CombatProfileStatConditionSchema = CombatProfileStatConditionSchema;
exports.CombatProfileStepSchema = CombatProfileStepSchema;
exports.CombatProfileThresholdUnitSchema = CombatProfileThresholdUnitSchema;
exports.DEFAULT_COMBAT_PROFILE_DELAY_MS = DEFAULT_COMBAT_PROFILE_DELAY_MS;
exports.DEFAULT_COMBAT_PROFILE_ID = DEFAULT_COMBAT_PROFILE_ID;
exports.DEFAULT_COMBAT_PROFILE_LIBRARY = DEFAULT_COMBAT_PROFILE_LIBRARY;
exports.SkillSlotSchema = SkillSlotSchema;
exports.cloneCombatProfileLibrary = cloneCombatProfileLibrary;
exports.duplicateCombatProfile = duplicateCombatProfile;
exports.findCombatProfileById = findCombatProfileById;
exports.getCombatProfileById = getCombatProfileById;
exports.isCombatProfileDefinition = isCombatProfileDefinition;
exports.normalizeCombatProfile = normalizeCombatProfile;
exports.normalizeCombatProfileLibrary = normalizeCombatProfileLibrary;
exports.serializeCombatProfileLibrary = serializeCombatProfileLibrary;
