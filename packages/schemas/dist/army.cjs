Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_accountSettings = require("./accountSettings.cjs");
const require_baseSchemas = require("./baseSchemas.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/army.ts
const ArmyConfigRawSchema = effect_Schema.Record(effect_Schema.String, effect_Schema.Unknown);
const ArmyEquipSetSchema = effect_Schema.Struct({
	armor: effect_Schema.optionalKey(effect_Schema.String),
	cape: effect_Schema.optionalKey(effect_Schema.String),
	class: effect_Schema.optionalKey(effect_Schema.String),
	helm: effect_Schema.optionalKey(effect_Schema.String),
	pet: effect_Schema.optionalKey(effect_Schema.String),
	pots: effect_Schema.optionalKey(effect_Schema.Array(effect_Schema.String)),
	safeClass: effect_Schema.optionalKey(effect_Schema.String),
	safePot: effect_Schema.optionalKey(effect_Schema.String),
	scroll: effect_Schema.optionalKey(effect_Schema.String),
	weapon: effect_Schema.optionalKey(effect_Schema.String)
});
const ArmySetConfigSchema = effect_Schema.Struct({
	default: effect_Schema.optionalKey(ArmyEquipSetSchema),
	players: effect_Schema.Record(effect_Schema.String, ArmyEquipSetSchema)
});
const ArmyConfigPayloadFields = {
	configName: effect_Schema.String,
	items: effect_Schema.Record(effect_Schema.String, effect_Schema.String),
	players: effect_Schema.Array(effect_Schema.String),
	raw: ArmyConfigRawSchema,
	room: require_accountSettings.RoomNumberSchema,
	sets: effect_Schema.Record(effect_Schema.String, ArmySetConfigSchema)
};
const ArmySessionPayloadSchema = effect_Schema.Struct({
	...ArmyConfigPayloadFields,
	playerName: effect_Schema.String,
	playerNumber: effect_Schema.Int,
	role: effect_Schema.Literals(["leader", "member"]),
	sessionId: effect_Schema.String
});
const ArmySessionEndedPayloadSchema = effect_Schema.Struct({
	reason: effect_Schema.String,
	sessionId: effect_Schema.String
});
const ArmyLoopTauntStrategySchema = effect_Schema.Union([effect_Schema.Struct({ type: effect_Schema.Literal("focus") }), effect_Schema.Struct({
	message: effect_Schema.String,
	type: effect_Schema.Literal("message")
})]);
const ArmyLoopTauntResolvedTargetSchema = effect_Schema.Struct({
	focusActive: effect_Schema.Boolean,
	lifeRevision: require_baseSchemas.NonNegativeInt,
	monsterMapId: effect_Schema.Int,
	state: effect_Schema.Literals(["alive", "dead"])
});
const ArmyLoopTauntRegistrationAssignmentSchema = effect_Schema.Struct({
	assignmentId: require_baseSchemas.NonNegativeInt,
	players: effect_Schema.Array(effect_Schema.Int),
	strategy: ArmyLoopTauntStrategySchema,
	target: ArmyLoopTauntResolvedTargetSchema
});
const ArmyLoopTauntRegistrationPriorityGroupSchema = effect_Schema.Struct({ assignments: effect_Schema.Array(ArmyLoopTauntRegistrationAssignmentSchema) });
const ArmyLoopTauntMapIdentitySchema = effect_Schema.Struct({
	id: effect_Schema.Int,
	name: effect_Schema.String,
	roomNumber: effect_Schema.Int
});
const NonNegativeFiniteNumber = effect_Schema.Finite.check(effect_Schema.isGreaterThanOrEqualTo(0));
const ArmyLoopTauntRegisterPayloadSchema = effect_Schema.Struct({
	map: ArmyLoopTauntMapIdentitySchema,
	priorityGroups: effect_Schema.Array(ArmyLoopTauntRegistrationPriorityGroupSchema),
	sessionId: effect_Schema.String
});
const ArmyLoopTauntRegisterResultSchema = effect_Schema.Struct({ runId: effect_Schema.String });
const ArmyLoopTauntRunPayloadSchema = effect_Schema.Struct({
	runId: effect_Schema.String,
	sessionId: effect_Schema.String
});
const ArmyLoopTauntLeavePayloadSchema = effect_Schema.Struct({
	reason: effect_Schema.optionalKey(effect_Schema.String),
	runId: effect_Schema.String,
	sessionId: effect_Schema.String
});
const ArmyLoopTauntTerminalResultSchema = effect_Schema.Union([effect_Schema.Struct({ status: effect_Schema.Literal("completed") }), effect_Schema.Struct({
	reason: effect_Schema.String,
	status: effect_Schema.Literal("failed")
})]);
const ArmyLoopTauntReportSchema = effect_Schema.Union([
	effect_Schema.Struct({
		alive: effect_Schema.Boolean,
		cooldownMs: effect_Schema.NullOr(NonNegativeFiniteNumber),
		type: effect_Schema.Literal("participant-state"),
		usable: effect_Schema.Boolean
	}),
	effect_Schema.Struct({
		assignmentId: require_baseSchemas.NonNegativeInt,
		focusActive: effect_Schema.optionalKey(effect_Schema.Boolean),
		lifeRevision: require_baseSchemas.NonNegativeInt,
		monsterMapId: effect_Schema.Int,
		state: effect_Schema.Literals(["alive", "dead"]),
		type: effect_Schema.Literal("target-state")
	}),
	effect_Schema.Struct({
		active: effect_Schema.Boolean,
		assignmentId: require_baseSchemas.NonNegativeInt,
		lifeRevision: require_baseSchemas.NonNegativeInt,
		monsterMapId: effect_Schema.Int,
		type: effect_Schema.Literal("focus-state")
	}),
	effect_Schema.Struct({
		assignmentId: require_baseSchemas.NonNegativeInt,
		lifeRevision: require_baseSchemas.NonNegativeInt,
		message: effect_Schema.String,
		monsterMapId: effect_Schema.Int,
		type: effect_Schema.Literal("message")
	}),
	effect_Schema.Struct({
		commandId: effect_Schema.Int,
		cooldownMs: effect_Schema.optionalKey(effect_Schema.NullOr(NonNegativeFiniteNumber)),
		outcome: effect_Schema.Literals([
			"confirmed",
			"target-unavailable",
			"cast-failed",
			"not-ready",
			"skipped"
		]),
		type: effect_Schema.Literal("command-result")
	}),
	effect_Schema.Struct({
		reason: effect_Schema.String,
		type: effect_Schema.Literal("failed")
	})
]);
const ArmyLoopTauntReportPayloadSchema = effect_Schema.Struct({
	report: ArmyLoopTauntReportSchema,
	runId: effect_Schema.String,
	sessionId: effect_Schema.String
});
const ArmyLoopTauntCommandSchema = effect_Schema.Struct({
	assignmentId: require_baseSchemas.NonNegativeInt,
	expiresAt: NonNegativeFiniteNumber,
	lifeRevision: require_baseSchemas.NonNegativeInt,
	monsterMapId: effect_Schema.Int,
	type: effect_Schema.Literal("taunt")
});
const ArmyLoopTauntCommandPayloadSchema = effect_Schema.Struct({
	command: ArmyLoopTauntCommandSchema,
	commandId: effect_Schema.Int,
	runId: effect_Schema.String,
	sessionId: effect_Schema.String
});
const EquipFields = /* @__PURE__ */ new Set([
	"armor",
	"cape",
	"class",
	"helm",
	"pet",
	"pots",
	"safeClass",
	"safePot",
	"scroll",
	"weapon"
]);
const decodeRecord = effect_Schema.decodeUnknownOption(ArmyConfigRawSchema);
const decodeArray = effect_Schema.decodeUnknownOption(effect_Schema.Array(effect_Schema.Unknown));
const decodeStringLike = effect_Schema.decodeUnknownOption(effect_Schema.Union([effect_Schema.String, effect_Schema.Number]));
const recordAt = (value, message) => {
	const decoded = decodeRecord(value);
	if (effect_Option.isNone(decoded)) throw new Error(message);
	return decoded.value;
};
const stringAt = (value, message) => {
	const decoded = decodeStringLike(value);
	if (effect_Option.isNone(decoded)) throw new Error(message);
	const normalized = String(decoded.value).trim();
	if (normalized === "") throw new Error(message);
	return normalized;
};
const decodeRoomNumber = effect_Schema.decodeUnknownOption(require_accountSettings.RoomNumberSchema);
const roomAt = (value) => {
	const text = stringAt(value, "Army config must define room");
	const room = /^\d+$/u.test(text) ? decodeRoomNumber(Number(text)) : effect_Option.none();
	if (effect_Option.isNone(room)) throw new Error(`Army config room must be a whole number from 1 to ${require_accountSettings.MAXIMUM_ROOM_NUMBER}`);
	return room.value;
};
const optionalStringAt = (value, message) => value === void 0 ? void 0 : stringAt(value, message);
const arrayAt = (value, message) => {
	const decoded = decodeArray(value);
	if (effect_Option.isNone(decoded)) throw new Error(message);
	return decoded.value;
};
const normalizeArmyConfigName = (fileName) => {
	const trimmed = fileName.trim();
	return trimmed.toLowerCase().endsWith(".yaml") ? trimmed.slice(0, -5).trim() : trimmed;
};
const isValidArmyConfigName = (configName) => /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(configName);
const assertValidArmyConfigName = (fileName) => {
	const configName = normalizeArmyConfigName(fileName);
	if (configName === "") throw new Error("Army config name is required");
	if (!isValidArmyConfigName(configName)) throw new Error("Army config name may only contain letters, numbers, dots, dashes, and underscores");
	return configName;
};
const normalizeArmyPlayerKey = (playerName) => playerName.trim().toLowerCase();
const assertArmyConfigRaw = (value) => recordAt(value, "Army config must be a YAML object");
const parsePlayers = (value) => {
	const rawPlayers = arrayAt(value, "Army config players must be a non-empty array");
	if (rawPlayers.length === 0) throw new Error("Army config players must be a non-empty array");
	const players = rawPlayers.map((player) => stringAt(player, "Army config players must only contain non-empty strings"));
	const seen = /* @__PURE__ */ new Set();
	for (const player of players) {
		const key = normalizeArmyPlayerKey(player);
		if (seen.has(key)) throw new Error(`Duplicate army player: ${player}`);
		seen.add(key);
	}
	return players;
};
const parseItems = (value) => {
	if (value === void 0) return {};
	const raw = recordAt(value, "Army config items must be an object");
	return Object.fromEntries(Object.entries(raw).map(([alias, item]) => {
		const normalizedAlias = alias.trim();
		if (normalizedAlias === "") throw new Error("Army config items must map non-empty aliases to items");
		return [normalizedAlias, stringAt(item, "Army config items must map non-empty aliases to items")];
	}));
};
const parseEquipSet = (value, path) => {
	const raw = recordAt(value, `${path} must be an object`);
	const parsed = {};
	for (const [field, fieldValue] of Object.entries(raw)) {
		if (!EquipFields.has(field)) throw new Error(`Unknown army equip set key: ${path}.${field}`);
		if (field === "pots") parsed[field] = arrayAt(fieldValue, `${path}.pots must be an array of non-empty strings`).map((pot) => stringAt(pot, `${path}.pots must be an array of non-empty strings`));
		else {
			const item = optionalStringAt(fieldValue, `${path}.${field} must be a non-empty string`);
			if (item !== void 0) parsed[field] = item;
		}
	}
	return parsed;
};
const parseSets = (value, playerCount) => {
	if (value === void 0) return {};
	const rawSets = recordAt(value, "Army config sets must be an object");
	const sets = {};
	for (const [rawName, value] of Object.entries(rawSets)) {
		const setName = rawName.trim();
		if (setName === "") throw new Error("Army config set names must be non-empty");
		const rawSet = recordAt(value, `Army config set ${setName} must be an object`);
		const players = {};
		let defaultSet;
		for (const [slot, slotValue] of Object.entries(rawSet)) {
			if (slot === "default") {
				defaultSet = parseEquipSet(slotValue, `sets.${setName}.default`);
				continue;
			}
			const match = /^player([1-9]\d*)$/.exec(slot);
			if (match?.[1] === void 0) throw new Error(`Unknown army set slot: sets.${setName}.${slot}`);
			if (Number.parseInt(match[1], 10) > playerCount) throw new Error(`Army set slot ${slot} is outside the configured player roster`);
			players[slot] = parseEquipSet(slotValue, `sets.${setName}.${slot}`);
		}
		sets[setName] = {
			...defaultSet === void 0 ? {} : { default: defaultSet },
			players
		};
	}
	return sets;
};
const parseArmyConfigCore = (value) => {
	const raw = assertArmyConfigRaw(value);
	const players = parsePlayers(raw["players"]);
	return {
		items: parseItems(raw["items"]),
		players,
		room: roomAt(raw["room"]),
		sets: parseSets(raw["sets"], players.length)
	};
};
const normalizeArmyConfig = (configName, value) => {
	const raw = assertArmyConfigRaw(value);
	return {
		configName: assertValidArmyConfigName(configName),
		raw,
		...parseArmyConfigCore(raw)
	};
};
const resolveArmyEquipSet = (session, setName) => {
	const set = session.sets[setName];
	if (set === void 0) return void 0;
	return {
		...set.default,
		...set.players[`player${session.playerNumber}`]
	};
};
const resolveArmyItemAlias = (config, item) => {
	if (item === void 0) return void 0;
	const normalized = item.trim();
	return normalized === "" ? void 0 : config.items[normalized] ?? normalized;
};
exports.ArmyConfigRawSchema = ArmyConfigRawSchema;
exports.ArmyEquipSetSchema = ArmyEquipSetSchema;
exports.ArmyLoopTauntCommandPayloadSchema = ArmyLoopTauntCommandPayloadSchema;
exports.ArmyLoopTauntCommandSchema = ArmyLoopTauntCommandSchema;
exports.ArmyLoopTauntLeavePayloadSchema = ArmyLoopTauntLeavePayloadSchema;
exports.ArmyLoopTauntMapIdentitySchema = ArmyLoopTauntMapIdentitySchema;
exports.ArmyLoopTauntRegisterPayloadSchema = ArmyLoopTauntRegisterPayloadSchema;
exports.ArmyLoopTauntRegisterResultSchema = ArmyLoopTauntRegisterResultSchema;
exports.ArmyLoopTauntRegistrationAssignmentSchema = ArmyLoopTauntRegistrationAssignmentSchema;
exports.ArmyLoopTauntRegistrationPriorityGroupSchema = ArmyLoopTauntRegistrationPriorityGroupSchema;
exports.ArmyLoopTauntReportPayloadSchema = ArmyLoopTauntReportPayloadSchema;
exports.ArmyLoopTauntReportSchema = ArmyLoopTauntReportSchema;
exports.ArmyLoopTauntResolvedTargetSchema = ArmyLoopTauntResolvedTargetSchema;
exports.ArmyLoopTauntRunPayloadSchema = ArmyLoopTauntRunPayloadSchema;
exports.ArmyLoopTauntStrategySchema = ArmyLoopTauntStrategySchema;
exports.ArmyLoopTauntTerminalResultSchema = ArmyLoopTauntTerminalResultSchema;
exports.ArmySessionEndedPayloadSchema = ArmySessionEndedPayloadSchema;
exports.ArmySessionPayloadSchema = ArmySessionPayloadSchema;
exports.ArmySetConfigSchema = ArmySetConfigSchema;
exports.assertArmyConfigRaw = assertArmyConfigRaw;
exports.assertValidArmyConfigName = assertValidArmyConfigName;
exports.isValidArmyConfigName = isValidArmyConfigName;
exports.normalizeArmyConfig = normalizeArmyConfig;
exports.normalizeArmyConfigName = normalizeArmyConfigName;
exports.normalizeArmyPlayerKey = normalizeArmyPlayerKey;
exports.parseArmyConfigCore = parseArmyConfigCore;
exports.resolveArmyEquipSet = resolveArmyEquipSet;
exports.resolveArmyItemAlias = resolveArmyItemAlias;
