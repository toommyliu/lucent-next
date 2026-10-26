import { PositiveInt, TrimmedNonEmptyString, TrimmedString } from "./baseSchemas.mjs";
import "./combatProfiles.mjs";
import * as Schema from "effect/Schema";
// src/follower.ts
const DEFAULT_FOLLOWER_ATTEMPTS = 3;
const DEFAULT_FOLLOWER_COMBAT_ENABLED = true;
const DEFAULT_FOLLOWER_COPY_WALK = false;
const DEFAULT_FOLLOWER_RETRY_ENABLED = true;
const FollowerPhaseSchema = Schema.Literals([
	"idle",
	"starting",
	"following",
	"walking",
	"combat",
	"stopped"
]);
const AttackPriorityInputSchema = Schema.Union([Schema.String, Schema.Array(Schema.Union([Schema.Number, Schema.String]))]);
const LocationFallbackInputSchema = Schema.Union([Schema.String, Schema.Array(Schema.String)]);
const FollowerStartPayloadSchema = Schema.Struct({
	targetName: Schema.String,
	combatEnabled: Schema.optionalKey(Schema.Boolean),
	copyWalk: Schema.optionalKey(Schema.Boolean),
	retryEnabled: Schema.optionalKey(Schema.Boolean),
	maxAttempts: Schema.optionalKey(Schema.Number),
	selectedProfileId: Schema.optionalKey(Schema.String),
	attackPriority: Schema.optionalKey(AttackPriorityInputSchema),
	lockedZoneFallbacks: Schema.optionalKey(LocationFallbackInputSchema),
	lockedZoneRoomOverride: Schema.optionalKey(Schema.String)
});
const FollowerConfigSchema = Schema.Struct({
	targetName: TrimmedString,
	combatEnabled: Schema.Boolean,
	copyWalk: Schema.Boolean,
	retryEnabled: Schema.Boolean,
	maxAttempts: PositiveInt,
	selectedProfileId: TrimmedNonEmptyString,
	attackPriority: Schema.Array(Schema.Union([PositiveInt, TrimmedNonEmptyString])),
	lockedZoneFallbacks: Schema.Array(TrimmedNonEmptyString),
	lockedZoneRoomOverride: TrimmedString
});
const FollowerStateSchema = Schema.Struct({
	enabled: Schema.Boolean,
	running: Schema.Boolean,
	targetName: Schema.String,
	profileId: Schema.optionalKey(Schema.String),
	profileLabel: Schema.optionalKey(Schema.String),
	phase: FollowerPhaseSchema,
	attemptsRemaining: Schema.Int.pipe(Schema.check(Schema.isGreaterThanOrEqualTo(0))),
	lastError: Schema.optionalKey(Schema.String),
	stoppedReason: Schema.optionalKey(Schema.String),
	warnings: Schema.optionalKey(Schema.Array(Schema.String))
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
export { DEFAULT_FOLLOWER_ATTEMPTS, DEFAULT_FOLLOWER_COMBAT_ENABLED, DEFAULT_FOLLOWER_COPY_WALK, DEFAULT_FOLLOWER_RETRY_ENABLED, FollowerConfigSchema, FollowerPhaseSchema, FollowerStartPayloadSchema, FollowerStateSchema, createIdleFollowerState, formatFollowerAttackTarget, normalizeFollowerConfig, normalizeFollowerState, normalizeFollowerTargetName, parseFollowerAttackPriority, parseFollowerLocationFallbacks };
