import { ScriptFileReferenceSchema } from "./scriptInputs.mjs";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
// src/accounts.ts
const ManagedAccountSchema = Schema.Struct({
	label: Schema.String,
	username: Schema.String,
	password: Schema.String
});
const ManagedAccountDraftSchema = Schema.Struct({
	label: Schema.optionalKey(Schema.String),
	username: Schema.String,
	password: Schema.String
});
const ManagedAccountPatchSchema = Schema.Struct({
	label: Schema.optionalKey(Schema.String),
	username: Schema.optionalKey(Schema.String),
	password: Schema.optionalKey(Schema.String)
});
const ManagedAccountGroupsSchema = Schema.Record(Schema.String, Schema.Array(Schema.String));
const ManagedAccountGroupDraftSchema = Schema.Struct({
	name: Schema.String,
	usernames: Schema.Array(Schema.String)
});
const ManagedAccountGroupPatchSchema = Schema.Struct({
	name: Schema.optionalKey(Schema.String),
	usernames: Schema.optionalKey(Schema.Array(Schema.String))
});
const AccountScriptReferenceSchema = ScriptFileReferenceSchema;
const AccountSessionConnectionSchema = Schema.Union([
	Schema.Struct({
		state: Schema.Literal("offline"),
		lastUsername: Schema.optionalKey(Schema.String)
	}),
	Schema.Struct({
		state: Schema.Literal("connecting"),
		lastUsername: Schema.optionalKey(Schema.String)
	}),
	Schema.Struct({
		state: Schema.Literal("online"),
		username: Schema.String
	})
]);
const AccountSessionActiveLoginSchema = (state) => Schema.Struct({
	state: Schema.Literal(state),
	attemptsRemaining: Schema.optionalKey(Schema.Number),
	message: Schema.optionalKey(Schema.String)
});
const AccountSessionLoginSchema = Schema.Union([
	Schema.Struct({ state: Schema.Literal("idle") }),
	Schema.Struct({ state: Schema.Literal("waiting-for-game") }),
	AccountSessionActiveLoginSchema("authenticating"),
	AccountSessionActiveLoginSchema("waiting-for-server"),
	Schema.Struct({
		state: Schema.Literal("selecting-server"),
		attemptsRemaining: Schema.optionalKey(Schema.Number),
		message: Schema.optionalKey(Schema.String),
		server: Schema.optionalKey(Schema.String)
	}),
	AccountSessionActiveLoginSchema("waiting-for-player"),
	Schema.Struct({
		state: Schema.Literal("failed"),
		message: Schema.String
	})
]);
const AccountSessionScriptSchema = Schema.Union([
	Schema.Struct({
		state: Schema.Literal("idle"),
		name: Schema.optionalKey(Schema.String)
	}),
	Schema.Struct({
		state: Schema.Literal("starting"),
		name: Schema.String,
		message: Schema.optionalKey(Schema.String)
	}),
	Schema.Struct({
		state: Schema.Literal("running"),
		name: Schema.String
	}),
	Schema.Struct({
		state: Schema.Literal("stopped"),
		name: Schema.optionalKey(Schema.String),
		message: Schema.optionalKey(Schema.String)
	}),
	Schema.Struct({
		state: Schema.Literal("failed"),
		name: Schema.optionalKey(Schema.String),
		message: Schema.String
	})
]);
/** Separate runtime facts reconciled by the renderer's session reducer. */
const AccountSessionRuntimeSchema = Schema.Struct({
	connection: AccountSessionConnectionSchema,
	login: AccountSessionLoginSchema,
	script: AccountSessionScriptSchema
});
const AccountGameServerSchema = Schema.Struct({
	name: Schema.String,
	language: Schema.String,
	online: Schema.Boolean,
	upgrade: Schema.Boolean,
	playerCount: Schema.Number,
	maxPlayers: Schema.Number
});
const AccountGameServersResultSchema = Schema.Struct({
	servers: Schema.Array(AccountGameServerSchema),
	refreshAvailableAt: Schema.Number
});
const AccountGameServerPingSchema = Schema.Union([Schema.Struct({
	serverName: Schema.String,
	status: Schema.Literal("ok"),
	latencyMs: Schema.Number
}), Schema.Struct({
	serverName: Schema.String,
	status: Schema.Literals([
		"offline",
		"timeout",
		"unreachable"
	])
})]);
const AccountGameServerPingsResultSchema = Schema.Struct({ pings: Schema.Array(AccountGameServerPingSchema) });
const AccountSessionLaunchSchema = Schema.Struct({
	requestedAt: Schema.Number,
	scriptName: Schema.optionalKey(Schema.String),
	server: Schema.optionalKey(Schema.String),
	username: Schema.String
});
const AccountGameSessionSchema = Schema.Struct({
	gameWindowId: Schema.Number,
	gameWindowGroupId: Schema.optionalKey(Schema.Number),
	rendererGeneration: Schema.Int,
	revision: Schema.Int,
	launch: Schema.optionalKey(AccountSessionLaunchSchema),
	connection: AccountSessionConnectionSchema,
	login: AccountSessionLoginSchema,
	script: AccountSessionScriptSchema,
	updatedAt: Schema.Number
});
const RecentlyClosedGameSessionSchema = Schema.Struct({
	id: Schema.Int,
	closedAt: Schema.Number,
	username: Schema.String,
	server: Schema.optionalKey(Schema.String)
});
const AccountManagerStateSchema = Schema.Struct({
	accounts: Schema.Array(ManagedAccountSchema),
	groups: ManagedAccountGroupsSchema,
	sessions: Schema.Array(AccountGameSessionSchema),
	storagePath: Schema.String
});
const AccountLaunchWindowTargetSchema = Schema.Union([Schema.Struct({ kind: Schema.Literal("new") }), Schema.Struct({
	kind: Schema.Literal("same-as-game"),
	gameWindowId: Schema.Number
})]);
const AccountLaunchRequestSchema = Schema.Struct({
	username: Schema.String,
	script: Schema.optionalKey(Schema.NullOr(AccountScriptReferenceSchema)),
	server: Schema.optionalKey(Schema.String),
	gameViewLayout: Schema.optionalKey(Schema.Literal("grid")),
	windowTarget: Schema.optionalKey(AccountLaunchWindowTargetSchema)
});
const AccountLaunchResultSchema = Schema.Struct({ gameWindowId: Schema.Number });
const AccountGameWindowTargetRequestSchema = Schema.Struct({ gameWindowId: Schema.Number });
const AccountGameLaunchPayloadSchema = Schema.Struct({
	account: ManagedAccountSchema,
	script: Schema.optionalKey(AccountScriptReferenceSchema),
	server: Schema.optionalKey(Schema.String),
	gameWindowId: Schema.Number,
	requestedAt: Schema.Number
});
/** A full renderer snapshot ordered within one renderer document generation. */
const AccountSessionReportSchema = Schema.Struct({
	rendererGeneration: Schema.Int,
	revision: Schema.Int,
	runtime: AccountSessionRuntimeSchema
});
const emptyStorage = {
	accounts: [],
	groups: {}
};
const UnknownRecordSchema = Schema.Record(Schema.String, Schema.Unknown);
const UnknownArraySchema = Schema.Array(Schema.Unknown);
const decodeUnknownArray = Schema.decodeUnknownOption(UnknownArraySchema);
const decodeUnknownRecord = Schema.decodeUnknownOption(UnknownRecordSchema);
const isManagedAccount = Schema.is(ManagedAccountSchema);
const decodeArrayOrEmpty = (value) => {
	const decoded = decodeUnknownArray(value);
	return Option.isSome(decoded) ? decoded.value : [];
};
const decodeRecordOrEmpty = (value) => {
	const decoded = decodeUnknownRecord(value);
	return Option.isSome(decoded) ? decoded.value : {};
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
	if (Option.isNone(storage)) return emptyStorage;
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
export { AccountGameLaunchPayloadSchema, AccountGameServerPingSchema, AccountGameServerPingsResultSchema, AccountGameServerSchema, AccountGameServersResultSchema, AccountGameSessionSchema, AccountGameWindowTargetRequestSchema, AccountLaunchRequestSchema, AccountLaunchResultSchema, AccountLaunchWindowTargetSchema, AccountManagerStateSchema, AccountScriptReferenceSchema, AccountSessionConnectionSchema, AccountSessionLaunchSchema, AccountSessionLoginSchema, AccountSessionReportSchema, AccountSessionRuntimeSchema, AccountSessionScriptSchema, ManagedAccountDraftSchema, ManagedAccountGroupDraftSchema, ManagedAccountGroupPatchSchema, ManagedAccountGroupsSchema, ManagedAccountPatchSchema, ManagedAccountSchema, RecentlyClosedGameSessionSchema, dedupeAccountsByUsername, emptyAccountManagerStorage, normalizeAccountManagerStorage, normalizeStoredAccount, removeGroupMemberUsername, renameGroupMemberUsername, serializeAccountManagerStorage };
