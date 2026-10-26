import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
// src/accountSettings.ts
const ACCOUNT_SETTINGS_VERSION = 1;
const MINIMUM_ROOM_NUMBER = 1;
const MINIMUM_PRIVATE_ROOM = 1001;
const MINIMUM_RANDOM_PRIVATE_ROOM = 1e4;
const MAXIMUM_ROOM_NUMBER = 99999;
const RoomNumberSchema = Schema.Int.check(Schema.isBetween({
	minimum: 1,
	maximum: MAXIMUM_ROOM_NUMBER
}));
const RoomPolicySchema = Schema.Union([
	Schema.Struct({ kind: Schema.Literal("public") }),
	Schema.Struct({ kind: Schema.Literal("random-private") }),
	Schema.Struct({
		kind: Schema.Literal("specific"),
		roomNumber: RoomNumberSchema
	})
]);
const AccountScriptSettingsSchema = Schema.Struct({
	restartAfterReconnect: Schema.Boolean,
	roomPolicy: RoomPolicySchema,
	safeStartStop: Schema.Boolean
});
const AccountSettingsSchema = Schema.Struct({
	version: Schema.Literal(ACCOUNT_SETTINGS_VERSION),
	scripts: AccountScriptSettingsSchema
});
const AccountScriptSettingsPatchSchema = Schema.Struct({
	restartAfterReconnect: Schema.optionalKey(Schema.Boolean),
	roomPolicy: Schema.optionalKey(RoomPolicySchema),
	safeStartStop: Schema.optionalKey(Schema.Boolean)
});
const AccountSettingsPatchSchema = Schema.Struct({ scripts: Schema.optionalKey(AccountScriptSettingsPatchSchema) });
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
const UnknownRecordSchema = Schema.Record(Schema.String, Schema.Unknown);
const decodeRecord = Schema.decodeUnknownOption(UnknownRecordSchema);
const decodeBoolean = Schema.decodeUnknownOption(Schema.Boolean);
const decodeRoomPolicy = Schema.decodeUnknownOption(RoomPolicySchema);
const recordOrEmpty = (value) => {
	const decoded = decodeRecord(value);
	return Option.isSome(decoded) ? decoded.value : {};
};
const booleanOr = (value, fallback) => {
	const decoded = decodeBoolean(value);
	return Option.isSome(decoded) ? decoded.value : fallback;
};
const roomPolicyOr = (value, fallback) => {
	const decoded = decodeRoomPolicy(value);
	return Option.isSome(decoded) ? decoded.value : fallback;
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
export { AccountScriptSettingsPatchSchema, AccountScriptSettingsSchema, AccountSettingsPatchSchema, AccountSettingsSchema, DEFAULT_ACCOUNT_SETTINGS, MAXIMUM_ROOM_NUMBER, MINIMUM_PRIVATE_ROOM, MINIMUM_RANDOM_PRIVATE_ROOM, MINIMUM_ROOM_NUMBER, PUBLIC_ROOM_POLICY, RANDOM_PRIVATE_ROOM_POLICY, RoomNumberSchema, RoomPolicySchema, applyAccountSettingsPatch, normalizeAccountSettings, serializeAccountSettings };
