Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_scriptInputs = require("./scriptInputs.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/accounts.ts
const ManagedAccountSchema = effect_Schema.Struct({
	label: effect_Schema.String,
	username: effect_Schema.String,
	password: effect_Schema.String
});
const ManagedAccountDraftSchema = effect_Schema.Struct({
	label: effect_Schema.optionalKey(effect_Schema.String),
	username: effect_Schema.String,
	password: effect_Schema.String
});
const ManagedAccountPatchSchema = effect_Schema.Struct({
	label: effect_Schema.optionalKey(effect_Schema.String),
	username: effect_Schema.optionalKey(effect_Schema.String),
	password: effect_Schema.optionalKey(effect_Schema.String)
});
const ManagedAccountGroupsSchema = effect_Schema.Record(effect_Schema.String, effect_Schema.Array(effect_Schema.String));
const ManagedAccountGroupDraftSchema = effect_Schema.Struct({
	name: effect_Schema.String,
	usernames: effect_Schema.Array(effect_Schema.String)
});
const ManagedAccountGroupPatchSchema = effect_Schema.Struct({
	name: effect_Schema.optionalKey(effect_Schema.String),
	usernames: effect_Schema.optionalKey(effect_Schema.Array(effect_Schema.String))
});
const AccountScriptReferenceSchema = require_scriptInputs.ScriptFileReferenceSchema;
const AccountSessionConnectionSchema = effect_Schema.Union([
	effect_Schema.Struct({
		state: effect_Schema.Literal("offline"),
		lastUsername: effect_Schema.optionalKey(effect_Schema.String)
	}),
	effect_Schema.Struct({
		state: effect_Schema.Literal("connecting"),
		lastUsername: effect_Schema.optionalKey(effect_Schema.String)
	}),
	effect_Schema.Struct({
		state: effect_Schema.Literal("online"),
		username: effect_Schema.String
	})
]);
const AccountSessionActiveLoginSchema = (state) => effect_Schema.Struct({
	state: effect_Schema.Literal(state),
	attemptsRemaining: effect_Schema.optionalKey(effect_Schema.Number),
	message: effect_Schema.optionalKey(effect_Schema.String)
});
const AccountSessionLoginSchema = effect_Schema.Union([
	effect_Schema.Struct({ state: effect_Schema.Literal("idle") }),
	effect_Schema.Struct({ state: effect_Schema.Literal("waiting-for-game") }),
	AccountSessionActiveLoginSchema("authenticating"),
	AccountSessionActiveLoginSchema("waiting-for-server"),
	effect_Schema.Struct({
		state: effect_Schema.Literal("selecting-server"),
		attemptsRemaining: effect_Schema.optionalKey(effect_Schema.Number),
		message: effect_Schema.optionalKey(effect_Schema.String),
		server: effect_Schema.optionalKey(effect_Schema.String)
	}),
	AccountSessionActiveLoginSchema("waiting-for-player"),
	effect_Schema.Struct({
		state: effect_Schema.Literal("failed"),
		message: effect_Schema.String
	})
]);
const AccountSessionScriptSchema = effect_Schema.Union([
	effect_Schema.Struct({
		state: effect_Schema.Literal("idle"),
		name: effect_Schema.optionalKey(effect_Schema.String)
	}),
	effect_Schema.Struct({
		state: effect_Schema.Literal("starting"),
		name: effect_Schema.String,
		message: effect_Schema.optionalKey(effect_Schema.String)
	}),
	effect_Schema.Struct({
		state: effect_Schema.Literal("running"),
		name: effect_Schema.String
	}),
	effect_Schema.Struct({
		state: effect_Schema.Literal("stopped"),
		name: effect_Schema.optionalKey(effect_Schema.String),
		message: effect_Schema.optionalKey(effect_Schema.String)
	}),
	effect_Schema.Struct({
		state: effect_Schema.Literal("failed"),
		name: effect_Schema.optionalKey(effect_Schema.String),
		message: effect_Schema.String
	})
]);
/** Separate runtime facts reconciled by the renderer's session reducer. */
const AccountSessionRuntimeSchema = effect_Schema.Struct({
	connection: AccountSessionConnectionSchema,
	login: AccountSessionLoginSchema,
	script: AccountSessionScriptSchema
});
const AccountGameServerSchema = effect_Schema.Struct({
	name: effect_Schema.String,
	language: effect_Schema.String,
	online: effect_Schema.Boolean,
	upgrade: effect_Schema.Boolean,
	playerCount: effect_Schema.Number,
	maxPlayers: effect_Schema.Number
});
const AccountGameServersResultSchema = effect_Schema.Struct({
	servers: effect_Schema.Array(AccountGameServerSchema),
	refreshAvailableAt: effect_Schema.Number
});
const AccountGameServerPingSchema = effect_Schema.Union([effect_Schema.Struct({
	serverName: effect_Schema.String,
	status: effect_Schema.Literal("ok"),
	latencyMs: effect_Schema.Number
}), effect_Schema.Struct({
	serverName: effect_Schema.String,
	status: effect_Schema.Literals([
		"offline",
		"timeout",
		"unreachable"
	])
})]);
const AccountGameServerPingsResultSchema = effect_Schema.Struct({ pings: effect_Schema.Array(AccountGameServerPingSchema) });
const AccountSessionLaunchSchema = effect_Schema.Struct({
	requestedAt: effect_Schema.Number,
	scriptName: effect_Schema.optionalKey(effect_Schema.String),
	server: effect_Schema.optionalKey(effect_Schema.String),
	username: effect_Schema.String
});
const AccountGameSessionSchema = effect_Schema.Struct({
	gameWindowId: effect_Schema.Number,
	gameWindowGroupId: effect_Schema.optionalKey(effect_Schema.Number),
	rendererGeneration: effect_Schema.Int,
	revision: effect_Schema.Int,
	launch: effect_Schema.optionalKey(AccountSessionLaunchSchema),
	connection: AccountSessionConnectionSchema,
	login: AccountSessionLoginSchema,
	script: AccountSessionScriptSchema,
	updatedAt: effect_Schema.Number
});
const RecentlyClosedGameSessionSchema = effect_Schema.Struct({
	id: effect_Schema.Int,
	closedAt: effect_Schema.Number,
	username: effect_Schema.String,
	server: effect_Schema.optionalKey(effect_Schema.String)
});
const AccountManagerStateSchema = effect_Schema.Struct({
	accounts: effect_Schema.Array(ManagedAccountSchema),
	groups: ManagedAccountGroupsSchema,
	sessions: effect_Schema.Array(AccountGameSessionSchema),
	storagePath: effect_Schema.String
});
const AccountLaunchWindowTargetSchema = effect_Schema.Union([effect_Schema.Struct({ kind: effect_Schema.Literal("new") }), effect_Schema.Struct({
	kind: effect_Schema.Literal("same-as-game"),
	gameWindowId: effect_Schema.Number
})]);
const AccountLaunchRequestSchema = effect_Schema.Struct({
	username: effect_Schema.String,
	script: effect_Schema.optionalKey(effect_Schema.NullOr(AccountScriptReferenceSchema)),
	server: effect_Schema.optionalKey(effect_Schema.String),
	gameViewLayout: effect_Schema.optionalKey(effect_Schema.Literal("grid")),
	windowTarget: effect_Schema.optionalKey(AccountLaunchWindowTargetSchema)
});
const AccountLaunchResultSchema = effect_Schema.Struct({ gameWindowId: effect_Schema.Number });
const AccountGameWindowTargetRequestSchema = effect_Schema.Struct({ gameWindowId: effect_Schema.Number });
const AccountGameLaunchPayloadSchema = effect_Schema.Struct({
	account: ManagedAccountSchema,
	script: effect_Schema.optionalKey(AccountScriptReferenceSchema),
	server: effect_Schema.optionalKey(effect_Schema.String),
	gameWindowId: effect_Schema.Number,
	requestedAt: effect_Schema.Number
});
/** A full renderer snapshot ordered within one renderer document generation. */
const AccountSessionReportSchema = effect_Schema.Struct({
	rendererGeneration: effect_Schema.Int,
	revision: effect_Schema.Int,
	runtime: AccountSessionRuntimeSchema
});
const emptyStorage = {
	accounts: [],
	groups: {}
};
const UnknownRecordSchema = effect_Schema.Record(effect_Schema.String, effect_Schema.Unknown);
const UnknownArraySchema = effect_Schema.Array(effect_Schema.Unknown);
const decodeUnknownArray = effect_Schema.decodeUnknownOption(UnknownArraySchema);
const decodeUnknownRecord = effect_Schema.decodeUnknownOption(UnknownRecordSchema);
const isManagedAccount = effect_Schema.is(ManagedAccountSchema);
const decodeArrayOrEmpty = (value) => {
	const decoded = decodeUnknownArray(value);
	return effect_Option.isSome(decoded) ? decoded.value : [];
};
const decodeRecordOrEmpty = (value) => {
	const decoded = decodeUnknownRecord(value);
	return effect_Option.isSome(decoded) ? decoded.value : {};
};
const normalizeStoredAccount = (account) => ({
	label: account.label,
	username: account.username,
	password: account.password
});
const dedupeAccountsByUsername = (accounts) => {
	const seen = /* @__PURE__ */ new Set();
	const nextAccounts = [];
	for (const account of accounts) {
		const key = account.username.toLowerCase();
		if (seen.has(key)) continue;
		seen.add(key);
		nextAccounts.push(account);
	}
	return nextAccounts;
};
const normalizeAccounts = (value) => {
	return dedupeAccountsByUsername(decodeArrayOrEmpty(value).filter(isManagedAccount).map(normalizeStoredAccount));
};
const normalizeStoredGroupMembers = (value, accounts) => {
	const accountUsernames = new Set(accounts.map((account) => account.username));
	const seen = /* @__PURE__ */ new Set();
	const usernames = [];
	for (const member of decodeArrayOrEmpty(value)) {
		if (typeof member !== "string" || !accountUsernames.has(member)) continue;
		const key = member.toLowerCase();
		if (seen.has(key)) continue;
		seen.add(key);
		usernames.push(member);
	}
	return usernames;
};
const normalizeGroups = (value, accounts) => {
	const groups = {};
	const seen = /* @__PURE__ */ new Set();
	for (const [rawName, rawMembers] of Object.entries(decodeRecordOrEmpty(value))) {
		const name = rawName.trim();
		const key = name.toLowerCase();
		if (name === "" || seen.has(key)) continue;
		seen.add(key);
		groups[name] = normalizeStoredGroupMembers(rawMembers, accounts);
	}
	return groups;
};
const normalizeAccountManagerStorage = (value) => {
	const storage = decodeUnknownRecord(value);
	if (effect_Option.isNone(storage)) return emptyStorage;
	const accounts = normalizeAccounts(storage.value["accounts"]);
	return {
		accounts,
		groups: normalizeGroups(storage.value["groups"], accounts)
	};
};
const renameGroupMemberUsername = (groups, currentUsername, nextUsername) => {
	if (currentUsername === nextUsername) return groups;
	const nextGroups = {};
	for (const [name, usernames] of Object.entries(groups)) {
		const seen = /* @__PURE__ */ new Set();
		const nextUsernames = [];
		for (const username of usernames) {
			const next = username === currentUsername ? nextUsername : username;
			const key = next.toLowerCase();
			if (seen.has(key)) continue;
			seen.add(key);
			nextUsernames.push(next);
		}
		nextGroups[name] = nextUsernames;
	}
	return nextGroups;
};
const removeGroupMemberUsername = (groups, accountUsername) => {
	const nextGroups = {};
	for (const [name, usernames] of Object.entries(groups)) nextGroups[name] = usernames.filter((username) => username !== accountUsername);
	return nextGroups;
};
const serializeAccountManagerStorage = (storage) => {
	const accounts = dedupeAccountsByUsername(storage.accounts.map(normalizeStoredAccount));
	return {
		accounts,
		groups: normalizeGroups(storage.groups, accounts)
	};
};
const emptyAccountManagerStorage = () => normalizeAccountManagerStorage(emptyStorage);
exports.AccountGameLaunchPayloadSchema = AccountGameLaunchPayloadSchema;
exports.AccountGameServerPingSchema = AccountGameServerPingSchema;
exports.AccountGameServerPingsResultSchema = AccountGameServerPingsResultSchema;
exports.AccountGameServerSchema = AccountGameServerSchema;
exports.AccountGameServersResultSchema = AccountGameServersResultSchema;
exports.AccountGameSessionSchema = AccountGameSessionSchema;
exports.AccountGameWindowTargetRequestSchema = AccountGameWindowTargetRequestSchema;
exports.AccountLaunchRequestSchema = AccountLaunchRequestSchema;
exports.AccountLaunchResultSchema = AccountLaunchResultSchema;
exports.AccountLaunchWindowTargetSchema = AccountLaunchWindowTargetSchema;
exports.AccountManagerStateSchema = AccountManagerStateSchema;
exports.AccountScriptReferenceSchema = AccountScriptReferenceSchema;
exports.AccountSessionConnectionSchema = AccountSessionConnectionSchema;
exports.AccountSessionLaunchSchema = AccountSessionLaunchSchema;
exports.AccountSessionLoginSchema = AccountSessionLoginSchema;
exports.AccountSessionReportSchema = AccountSessionReportSchema;
exports.AccountSessionRuntimeSchema = AccountSessionRuntimeSchema;
exports.AccountSessionScriptSchema = AccountSessionScriptSchema;
exports.ManagedAccountDraftSchema = ManagedAccountDraftSchema;
exports.ManagedAccountGroupDraftSchema = ManagedAccountGroupDraftSchema;
exports.ManagedAccountGroupPatchSchema = ManagedAccountGroupPatchSchema;
exports.ManagedAccountGroupsSchema = ManagedAccountGroupsSchema;
exports.ManagedAccountPatchSchema = ManagedAccountPatchSchema;
exports.ManagedAccountSchema = ManagedAccountSchema;
exports.RecentlyClosedGameSessionSchema = RecentlyClosedGameSessionSchema;
exports.dedupeAccountsByUsername = dedupeAccountsByUsername;
exports.emptyAccountManagerStorage = emptyAccountManagerStorage;
exports.normalizeAccountManagerStorage = normalizeAccountManagerStorage;
exports.normalizeStoredAccount = normalizeStoredAccount;
exports.removeGroupMemberUsername = removeGroupMemberUsername;
exports.renameGroupMemberUsername = renameGroupMemberUsername;
exports.serializeAccountManagerStorage = serializeAccountManagerStorage;
