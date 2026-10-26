Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/accountSettings.ts
const ACCOUNT_SETTINGS_VERSION = 1;
const MINIMUM_ROOM_NUMBER = 1;
const MINIMUM_PRIVATE_ROOM = 1001;
const MINIMUM_RANDOM_PRIVATE_ROOM = 1e4;
const MAXIMUM_ROOM_NUMBER = 99999;
const RoomNumberSchema = effect_Schema.Int.check(effect_Schema.isBetween({
	minimum: 1,
	maximum: MAXIMUM_ROOM_NUMBER
}));
const RoomPolicySchema = effect_Schema.Union([
	effect_Schema.Struct({ kind: effect_Schema.Literal("public") }),
	effect_Schema.Struct({ kind: effect_Schema.Literal("random-private") }),
	effect_Schema.Struct({
		kind: effect_Schema.Literal("specific"),
		roomNumber: RoomNumberSchema
	})
]);
const AccountScriptSettingsSchema = effect_Schema.Struct({
	restartAfterReconnect: effect_Schema.Boolean,
	roomPolicy: RoomPolicySchema,
	safeStartStop: effect_Schema.Boolean
});
const AccountSettingsSchema = effect_Schema.Struct({
	version: effect_Schema.Literal(ACCOUNT_SETTINGS_VERSION),
	scripts: AccountScriptSettingsSchema
});
const AccountScriptSettingsPatchSchema = effect_Schema.Struct({
	restartAfterReconnect: effect_Schema.optionalKey(effect_Schema.Boolean),
	roomPolicy: effect_Schema.optionalKey(RoomPolicySchema),
	safeStartStop: effect_Schema.optionalKey(effect_Schema.Boolean)
});
const AccountSettingsPatchSchema = effect_Schema.Struct({ scripts: effect_Schema.optionalKey(AccountScriptSettingsPatchSchema) });
const PUBLIC_ROOM_POLICY = { kind: "public" };
const RANDOM_PRIVATE_ROOM_POLICY = { kind: "random-private" };
const DEFAULT_ACCOUNT_SETTINGS = {
	version: ACCOUNT_SETTINGS_VERSION,
	scripts: {
		restartAfterReconnect: false,
		roomPolicy: RANDOM_PRIVATE_ROOM_POLICY,
		safeStartStop: true
	}
};
const UnknownRecordSchema = effect_Schema.Record(effect_Schema.String, effect_Schema.Unknown);
const decodeRecord = effect_Schema.decodeUnknownOption(UnknownRecordSchema);
const decodeBoolean = effect_Schema.decodeUnknownOption(effect_Schema.Boolean);
const decodeRoomPolicy = effect_Schema.decodeUnknownOption(RoomPolicySchema);
const recordOrEmpty = (value) => {
	const decoded = decodeRecord(value);
	return effect_Option.isSome(decoded) ? decoded.value : {};
};
const booleanOr = (value, fallback) => {
	const decoded = decodeBoolean(value);
	return effect_Option.isSome(decoded) ? decoded.value : fallback;
};
const roomPolicyOr = (value, fallback) => {
	const decoded = decodeRoomPolicy(value);
	return effect_Option.isSome(decoded) ? decoded.value : fallback;
};
const normalizeAccountSettings = (value) => {
	const document = recordOrEmpty(value);
	const scripts = recordOrEmpty(document["scripts"]);
	return {
		version: ACCOUNT_SETTINGS_VERSION,
		scripts: {
			restartAfterReconnect: booleanOr(scripts["restartAfterReconnect"], DEFAULT_ACCOUNT_SETTINGS.scripts.restartAfterReconnect),
			roomPolicy: roomPolicyOr(scripts["roomPolicy"], DEFAULT_ACCOUNT_SETTINGS.scripts.roomPolicy),
			safeStartStop: booleanOr(scripts["safeStartStop"], DEFAULT_ACCOUNT_SETTINGS.scripts.safeStartStop)
		}
	};
};
const applyAccountSettingsPatch = (current, patch) => normalizeAccountSettings({
	...current,
	scripts: {
		...current.scripts,
		...patch.scripts
	}
});
const serializeAccountSettings = (settings) => normalizeAccountSettings(settings);
exports.AccountScriptSettingsPatchSchema = AccountScriptSettingsPatchSchema;
exports.AccountScriptSettingsSchema = AccountScriptSettingsSchema;
exports.AccountSettingsPatchSchema = AccountSettingsPatchSchema;
exports.AccountSettingsSchema = AccountSettingsSchema;
exports.DEFAULT_ACCOUNT_SETTINGS = DEFAULT_ACCOUNT_SETTINGS;
exports.MAXIMUM_ROOM_NUMBER = MAXIMUM_ROOM_NUMBER;
exports.MINIMUM_PRIVATE_ROOM = MINIMUM_PRIVATE_ROOM;
exports.MINIMUM_RANDOM_PRIVATE_ROOM = MINIMUM_RANDOM_PRIVATE_ROOM;
exports.MINIMUM_ROOM_NUMBER = MINIMUM_ROOM_NUMBER;
exports.PUBLIC_ROOM_POLICY = PUBLIC_ROOM_POLICY;
exports.RANDOM_PRIVATE_ROOM_POLICY = RANDOM_PRIVATE_ROOM_POLICY;
exports.RoomNumberSchema = RoomNumberSchema;
exports.RoomPolicySchema = RoomPolicySchema;
exports.applyAccountSettingsPatch = applyAccountSettingsPatch;
exports.normalizeAccountSettings = normalizeAccountSettings;
exports.serializeAccountSettings = serializeAccountSettings;
