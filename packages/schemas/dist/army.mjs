import { MAXIMUM_ROOM_NUMBER, RoomNumberSchema } from "./accountSettings.mjs";
import { NonNegativeInt } from "./baseSchemas.mjs";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
// src/army.ts
const ArmyConfigRawSchema = Schema.Record(Schema.String, Schema.Unknown);
const ArmyEquipSetSchema = Schema.Struct({
	armor: Schema.optionalKey(Schema.String),
	cape: Schema.optionalKey(Schema.String),
	class: Schema.optionalKey(Schema.String),
	helm: Schema.optionalKey(Schema.String),
	pet: Schema.optionalKey(Schema.String),
	pots: Schema.optionalKey(Schema.Array(Schema.String)),
	safeClass: Schema.optionalKey(Schema.String),
	safePot: Schema.optionalKey(Schema.String),
	scroll: Schema.optionalKey(Schema.String),
	weapon: Schema.optionalKey(Schema.String)
});
const ArmySetConfigSchema = Schema.Struct({
	default: Schema.optionalKey(ArmyEquipSetSchema),
	players: Schema.Record(Schema.String, ArmyEquipSetSchema)
});
const ArmyConfigPayloadFields = {
	configName: Schema.String,
	items: Schema.Record(Schema.String, Schema.String),
	players: Schema.Array(Schema.String),
	raw: ArmyConfigRawSchema,
	room: RoomNumberSchema,
	sets: Schema.Record(Schema.String, ArmySetConfigSchema)
};
const ArmySessionPayloadSchema = Schema.Struct({
	...ArmyConfigPayloadFields,
	playerName: Schema.String,
	playerNumber: Schema.Int,
	role: Schema.Literals(["leader", "member"]),
	sessionId: Schema.String
});
const ArmySessionEndedPayloadSchema = Schema.Struct({
	reason: Schema.String,
	sessionId: Schema.String
});
const ArmyLoopTauntStrategySchema = Schema.Union([Schema.Struct({ type: Schema.Literal("focus") }), Schema.Struct({
	message: Schema.String,
	type: Schema.Literal("message")
})]);
const ArmyLoopTauntResolvedTargetSchema = Schema.Struct({
	focusActive: Schema.Boolean,
	lifeRevision: NonNegativeInt,
	monsterMapId: Schema.Int,
	state: Schema.Literals(["alive", "dead"])
});
const ArmyLoopTauntRegistrationAssignmentSchema = Schema.Struct({
	assignmentId: NonNegativeInt,
	players: Schema.Array(Schema.Int),
	strategy: ArmyLoopTauntStrategySchema,
	target: ArmyLoopTauntResolvedTargetSchema
});
const ArmyLoopTauntRegistrationPriorityGroupSchema = Schema.Struct({ assignments: Schema.Array(ArmyLoopTauntRegistrationAssignmentSchema) });
const ArmyLoopTauntMapIdentitySchema = Schema.Struct({
	id: Schema.Int,
	name: Schema.String,
	roomNumber: Schema.Int
});
const NonNegativeFiniteNumber = Schema.Finite.check(Schema.isGreaterThanOrEqualTo(0));
const ArmyLoopTauntRegisterPayloadSchema = Schema.Struct({
	map: ArmyLoopTauntMapIdentitySchema,
	priorityGroups: Schema.Array(ArmyLoopTauntRegistrationPriorityGroupSchema),
	sessionId: Schema.String
});
const ArmyLoopTauntRegisterResultSchema = Schema.Struct({ runId: Schema.String });
const ArmyLoopTauntRunPayloadSchema = Schema.Struct({
	runId: Schema.String,
	sessionId: Schema.String
});
const ArmyLoopTauntLeavePayloadSchema = Schema.Struct({
	reason: Schema.optionalKey(Schema.String),
	runId: Schema.String,
	sessionId: Schema.String
});
const ArmyLoopTauntTerminalResultSchema = Schema.Union([Schema.Struct({ status: Schema.Literal("completed") }), Schema.Struct({
	reason: Schema.String,
	status: Schema.Literal("failed")
})]);
const ArmyLoopTauntReportSchema = Schema.Union([
	Schema.Struct({
		alive: Schema.Boolean,
		cooldownMs: Schema.NullOr(NonNegativeFiniteNumber),
		type: Schema.Literal("participant-state"),
		usable: Schema.Boolean
	}),
	Schema.Struct({
		assignmentId: NonNegativeInt,
		focusActive: Schema.optionalKey(Schema.Boolean),
		lifeRevision: NonNegativeInt,
		monsterMapId: Schema.Int,
		state: Schema.Literals(["alive", "dead"]),
		type: Schema.Literal("target-state")
	}),
	Schema.Struct({
		active: Schema.Boolean,
		assignmentId: NonNegativeInt,
		lifeRevision: NonNegativeInt,
		monsterMapId: Schema.Int,
		type: Schema.Literal("focus-state")
	}),
	Schema.Struct({
		assignmentId: NonNegativeInt,
		lifeRevision: NonNegativeInt,
		message: Schema.String,
		monsterMapId: Schema.Int,
		type: Schema.Literal("message")
	}),
	Schema.Struct({
		commandId: Schema.Int,
		cooldownMs: Schema.optionalKey(Schema.NullOr(NonNegativeFiniteNumber)),
		outcome: Schema.Literals([
			"confirmed",
			"target-unavailable",
			"cast-failed",
			"not-ready",
			"skipped"
		]),
		type: Schema.Literal("command-result")
	}),
	Schema.Struct({
		reason: Schema.String,
		type: Schema.Literal("failed")
	})
]);
const ArmyLoopTauntReportPayloadSchema = Schema.Struct({
	report: ArmyLoopTauntReportSchema,
	runId: Schema.String,
	sessionId: Schema.String
});
const ArmyLoopTauntCommandSchema = Schema.Struct({
	assignmentId: NonNegativeInt,
	expiresAt: NonNegativeFiniteNumber,
	lifeRevision: NonNegativeInt,
	monsterMapId: Schema.Int,
	type: Schema.Literal("taunt")
});
const ArmyLoopTauntCommandPayloadSchema = Schema.Struct({
	command: ArmyLoopTauntCommandSchema,
	commandId: Schema.Int,
	runId: Schema.String,
	sessionId: Schema.String
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
const decodeRecord = Schema.decodeUnknownOption(ArmyConfigRawSchema);
const decodeArray = Schema.decodeUnknownOption(Schema.Array(Schema.Unknown));
const decodeStringLike = Schema.decodeUnknownOption(Schema.Union([Schema.String, Schema.Number]));
const recordAt = (value, message) => {
	const decoded = decodeRecord(value);
	if (Option.isNone(decoded)) throw new Error(message);
	return decoded.value;
};
const stringAt = (value, message) => {
	const decoded = decodeStringLike(value);
	if (Option.isNone(decoded)) throw new Error(message);
	const normalized = String(decoded.value).trim();
	if (normalized === "") throw new Error(message);
	return normalized;
};
const decodeRoomNumber = Schema.decodeUnknownOption(RoomNumberSchema);
const roomAt = (value) => {
	const text = stringAt(value, "Army config must define room");
	const room = /^\d+$/u.test(text) ? decodeRoomNumber(Number(text)) : Option.none();
	if (Option.isNone(room)) throw new Error(`Army config room must be a whole number from 1 to ${MAXIMUM_ROOM_NUMBER}`);
	return room.value;
};
const optionalStringAt = (value, message) => value === void 0 ? void 0 : stringAt(value, message);
const arrayAt = (value, message) => {
	const decoded = decodeArray(value);
	if (Option.isNone(decoded)) throw new Error(message);
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
export { ArmyConfigRawSchema, ArmyEquipSetSchema, ArmyLoopTauntCommandPayloadSchema, ArmyLoopTauntCommandSchema, ArmyLoopTauntLeavePayloadSchema, ArmyLoopTauntMapIdentitySchema, ArmyLoopTauntRegisterPayloadSchema, ArmyLoopTauntRegisterResultSchema, ArmyLoopTauntRegistrationAssignmentSchema, ArmyLoopTauntRegistrationPriorityGroupSchema, ArmyLoopTauntReportPayloadSchema, ArmyLoopTauntReportSchema, ArmyLoopTauntResolvedTargetSchema, ArmyLoopTauntRunPayloadSchema, ArmyLoopTauntStrategySchema, ArmyLoopTauntTerminalResultSchema, ArmySessionEndedPayloadSchema, ArmySessionPayloadSchema, ArmySetConfigSchema, assertArmyConfigRaw, assertValidArmyConfigName, isValidArmyConfigName, normalizeArmyConfig, normalizeArmyConfigName, normalizeArmyPlayerKey, parseArmyConfigCore, resolveArmyEquipSet, resolveArmyItemAlias };
