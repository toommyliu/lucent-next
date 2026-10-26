Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_baseSchemas = require("./baseSchemas.cjs");
require("./combatProfiles.cjs");
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/follower.ts
const DEFAULT_FOLLOWER_ATTEMPTS = 3;
const DEFAULT_FOLLOWER_COMBAT_ENABLED = true;
const DEFAULT_FOLLOWER_COPY_WALK = false;
const DEFAULT_FOLLOWER_RETRY_ENABLED = true;
const FollowerPhaseSchema = effect_Schema.Literals([
	"idle",
	"starting",
	"following",
	"walking",
	"combat",
	"stopped"
]);
const AttackPriorityInputSchema = effect_Schema.Union([effect_Schema.String, effect_Schema.Array(effect_Schema.Union([effect_Schema.Number, effect_Schema.String]))]);
const LocationFallbackInputSchema = effect_Schema.Union([effect_Schema.String, effect_Schema.Array(effect_Schema.String)]);
const FollowerStartPayloadSchema = effect_Schema.Struct({
	targetName: effect_Schema.String,
	combatEnabled: effect_Schema.optionalKey(effect_Schema.Boolean),
	copyWalk: effect_Schema.optionalKey(effect_Schema.Boolean),
	retryEnabled: effect_Schema.optionalKey(effect_Schema.Boolean),
	maxAttempts: effect_Schema.optionalKey(effect_Schema.Number),
	selectedProfileId: effect_Schema.optionalKey(effect_Schema.String),
	attackPriority: effect_Schema.optionalKey(AttackPriorityInputSchema),
	lockedZoneFallbacks: effect_Schema.optionalKey(LocationFallbackInputSchema),
	lockedZoneRoomOverride: effect_Schema.optionalKey(effect_Schema.String)
});
const FollowerConfigSchema = effect_Schema.Struct({
	targetName: require_baseSchemas.TrimmedString,
	combatEnabled: effect_Schema.Boolean,
	copyWalk: effect_Schema.Boolean,
	retryEnabled: effect_Schema.Boolean,
	maxAttempts: require_baseSchemas.PositiveInt,
	selectedProfileId: require_baseSchemas.TrimmedNonEmptyString,
	attackPriority: effect_Schema.Array(effect_Schema.Union([require_baseSchemas.PositiveInt, require_baseSchemas.TrimmedNonEmptyString])),
	lockedZoneFallbacks: effect_Schema.Array(require_baseSchemas.TrimmedNonEmptyString),
	lockedZoneRoomOverride: require_baseSchemas.TrimmedString
});
const FollowerStateSchema = effect_Schema.Struct({
	enabled: effect_Schema.Boolean,
	running: effect_Schema.Boolean,
	targetName: effect_Schema.String,
	profileId: effect_Schema.optionalKey(effect_Schema.String),
	profileLabel: effect_Schema.optionalKey(effect_Schema.String),
	phase: FollowerPhaseSchema,
	attemptsRemaining: effect_Schema.Int.pipe(effect_Schema.check(effect_Schema.isGreaterThanOrEqualTo(0))),
	lastError: effect_Schema.optionalKey(effect_Schema.String),
	stoppedReason: effect_Schema.optionalKey(effect_Schema.String),
	warnings: effect_Schema.optionalKey(effect_Schema.Array(effect_Schema.String))
});
const monsterMapIdToken = /^id[.:'-]([1-9][0-9]*)$/iu;
const normalizedText = (value) => typeof value === "string" ? value.trim() : "";
const normalizeAttackTarget = (value) => {
	if (typeof value === "number") {
		if (!Number.isFinite(value) || value <= 0) return;
		return Math.min(Number.MAX_SAFE_INTEGER, Math.trunc(value));
	}
	const token = value.trim();
	if (token === "") return;
	const id = monsterMapIdToken.exec(token)?.[1];
	return id === void 0 ? token : Math.min(Number.MAX_SAFE_INTEGER, Number(id));
};
const formatFollowerAttackTarget = (target) => typeof target === "number" ? `id:${target}` : target;
const parseFollowerAttackPriority = (input) => {
	const candidates = typeof input === "string" ? input.split(",") : Array.isArray(input) ? input : [];
	const result = [];
	const identities = /* @__PURE__ */ new Set();
	for (const candidate of candidates) {
		const target = normalizeAttackTarget(candidate);
		if (target === void 0) continue;
		const identity = `${typeof target}:${String(target).toLowerCase()}`;
		if (identities.has(identity)) continue;
		identities.add(identity);
		result.push(target);
	}
	return result;
};
const parseFollowerLocationFallbacks = (input) => {
	const candidates = typeof input === "string" ? input.split(/\r?\n/u) : Array.isArray(input) ? input : [];
	const result = [];
	const identities = /* @__PURE__ */ new Set();
	for (const candidate of candidates) {
		const location = candidate.trim();
		if (location === "") continue;
		const identity = location.toLowerCase();
		if (identities.has(identity)) continue;
		identities.add(identity);
		result.push(location);
	}
	return result;
};
const normalizeFollowerTargetName = (value) => normalizedText(value).toLowerCase();
const normalizeAttemptCount = (value) => {
	if (typeof value !== "number" || !Number.isFinite(value)) return 3;
	return Math.min(Number.MAX_SAFE_INTEGER, Math.max(1, Math.trunc(value)));
};
const normalizeFollowerConfig = (input) => ({
	targetName: normalizeFollowerTargetName(input.targetName),
	combatEnabled: input.combatEnabled ?? true,
	copyWalk: input.copyWalk ?? false,
	retryEnabled: input.retryEnabled ?? true,
	maxAttempts: normalizeAttemptCount(input.maxAttempts),
	selectedProfileId: normalizedText(input.selectedProfileId) || "generic-base",
	attackPriority: parseFollowerAttackPriority(input.attackPriority),
	lockedZoneFallbacks: parseFollowerLocationFallbacks(input.lockedZoneFallbacks),
	lockedZoneRoomOverride: normalizedText(input.lockedZoneRoomOverride)
});
const createIdleFollowerState = () => ({
	enabled: false,
	running: false,
	targetName: "",
	phase: "idle",
	attemptsRemaining: 3
});
const optionalText = (value) => {
	const text = normalizedText(value);
	return text === "" ? void 0 : text;
};
const normalizeFollowerState = (value) => {
	if (typeof value !== "object" || value === null || Array.isArray(value)) return createIdleFollowerState();
	const candidate = value;
	const rawPhase = candidate["phase"];
	const phase = rawPhase === "starting" || rawPhase === "following" || rawPhase === "walking" || rawPhase === "combat" || rawPhase === "stopped" ? rawPhase : "idle";
	const rawAttempts = candidate["attemptsRemaining"];
	const attemptsRemaining = typeof rawAttempts === "number" && Number.isFinite(rawAttempts) ? Math.min(Number.MAX_SAFE_INTEGER, Math.max(0, Math.trunc(rawAttempts))) : 3;
	const profileId = optionalText(candidate["profileId"]);
	const profileLabel = optionalText(candidate["profileLabel"]);
	const lastError = optionalText(candidate["lastError"]);
	const stoppedReason = optionalText(candidate["stoppedReason"]);
	const rawWarnings = candidate["warnings"];
	const warnings = Array.isArray(rawWarnings) ? rawWarnings.flatMap((warning) => optionalText(warning) ?? []) : [];
	return {
		enabled: candidate["enabled"] === true,
		running: candidate["running"] === true,
		targetName: normalizeFollowerTargetName(candidate["targetName"]),
		...profileId === void 0 ? {} : { profileId },
		...profileLabel === void 0 ? {} : { profileLabel },
		phase,
		attemptsRemaining,
		...lastError === void 0 ? {} : { lastError },
		...stoppedReason === void 0 ? {} : { stoppedReason },
		...warnings.length === 0 ? {} : { warnings }
	};
};
exports.DEFAULT_FOLLOWER_ATTEMPTS = DEFAULT_FOLLOWER_ATTEMPTS;
exports.DEFAULT_FOLLOWER_COMBAT_ENABLED = DEFAULT_FOLLOWER_COMBAT_ENABLED;
exports.DEFAULT_FOLLOWER_COPY_WALK = DEFAULT_FOLLOWER_COPY_WALK;
exports.DEFAULT_FOLLOWER_RETRY_ENABLED = DEFAULT_FOLLOWER_RETRY_ENABLED;
exports.FollowerConfigSchema = FollowerConfigSchema;
exports.FollowerPhaseSchema = FollowerPhaseSchema;
exports.FollowerStartPayloadSchema = FollowerStartPayloadSchema;
exports.FollowerStateSchema = FollowerStateSchema;
exports.createIdleFollowerState = createIdleFollowerState;
exports.formatFollowerAttackTarget = formatFollowerAttackTarget;
exports.normalizeFollowerConfig = normalizeFollowerConfig;
exports.normalizeFollowerState = normalizeFollowerState;
exports.normalizeFollowerTargetName = normalizeFollowerTargetName;
exports.parseFollowerAttackPriority = parseFollowerAttackPriority;
exports.parseFollowerLocationFallbacks = parseFollowerLocationFallbacks;
