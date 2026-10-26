import * as Schema from "effect/Schema";
declare const MINIMUM_ROOM_NUMBER = 1;
declare const MINIMUM_PRIVATE_ROOM = 1001;
declare const MINIMUM_RANDOM_PRIVATE_ROOM = 10000;
declare const MAXIMUM_ROOM_NUMBER = 99999;
declare const RoomNumberSchema: Schema.Int;
declare const RoomPolicySchema: Schema.Union<readonly [Schema.Struct<{
  readonly kind: Schema.Literal<"public">;
}>, Schema.Struct<{
  readonly kind: Schema.Literal<"random-private">;
}>, Schema.Struct<{
  readonly kind: Schema.Literal<"specific">;
  readonly roomNumber: Schema.Int;
}>]>;
type RoomPolicy = typeof RoomPolicySchema.Type;
declare const AccountScriptSettingsSchema: Schema.Struct<{
  readonly restartAfterReconnect: Schema.Boolean;
  readonly roomPolicy: Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"public">;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"random-private">;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"specific">;
    readonly roomNumber: Schema.Int;
  }>]>;
  readonly safeStartStop: Schema.Boolean;
}>;
type AccountScriptSettings = typeof AccountScriptSettingsSchema.Type;
declare const AccountSettingsSchema: Schema.Struct<{
  readonly version: Schema.Literal<1>;
  readonly scripts: Schema.Struct<{
    readonly restartAfterReconnect: Schema.Boolean;
    readonly roomPolicy: Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"public">;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"random-private">;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"specific">;
      readonly roomNumber: Schema.Int;
    }>]>;
    readonly safeStartStop: Schema.Boolean;
  }>;
}>;
type AccountSettings = typeof AccountSettingsSchema.Type;
declare const AccountScriptSettingsPatchSchema: Schema.Struct<{
  readonly restartAfterReconnect: Schema.optionalKey<Schema.Boolean>;
  readonly roomPolicy: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"public">;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"random-private">;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"specific">;
    readonly roomNumber: Schema.Int;
  }>]>>;
  readonly safeStartStop: Schema.optionalKey<Schema.Boolean>;
}>;
type AccountScriptSettingsPatch = typeof AccountScriptSettingsPatchSchema.Type;
declare const AccountSettingsPatchSchema: Schema.Struct<{
  readonly scripts: Schema.optionalKey<Schema.Struct<{
    readonly restartAfterReconnect: Schema.optionalKey<Schema.Boolean>;
    readonly roomPolicy: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"public">;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"random-private">;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"specific">;
      readonly roomNumber: Schema.Int;
    }>]>>;
    readonly safeStartStop: Schema.optionalKey<Schema.Boolean>;
  }>>;
}>;
type AccountSettingsPatch = typeof AccountSettingsPatchSchema.Type;
declare const PUBLIC_ROOM_POLICY: RoomPolicy;
declare const RANDOM_PRIVATE_ROOM_POLICY: RoomPolicy;
declare const DEFAULT_ACCOUNT_SETTINGS: AccountSettings;
declare const normalizeAccountSettings: (value: unknown) => AccountSettings;
declare const applyAccountSettingsPatch: (current: AccountSettings, patch: AccountSettingsPatch) => AccountSettings;
declare const serializeAccountSettings: (settings: AccountSettings) => AccountSettings;
export { AccountScriptSettings, AccountScriptSettingsPatch, AccountScriptSettingsPatchSchema, AccountScriptSettingsSchema, AccountSettings, AccountSettingsPatch, AccountSettingsPatchSchema, AccountSettingsSchema, DEFAULT_ACCOUNT_SETTINGS, MAXIMUM_ROOM_NUMBER, MINIMUM_PRIVATE_ROOM, MINIMUM_RANDOM_PRIVATE_ROOM, MINIMUM_ROOM_NUMBER, PUBLIC_ROOM_POLICY, RANDOM_PRIVATE_ROOM_POLICY, RoomNumberSchema, RoomPolicy, RoomPolicySchema, applyAccountSettingsPatch, normalizeAccountSettings, serializeAccountSettings };