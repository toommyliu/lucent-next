import * as Schema from "effect/Schema";
declare const COMBAT_PROFILE_LIBRARY_VERSION: 1;
declare const DEFAULT_COMBAT_PROFILE_ID = "generic-base";
declare const DEFAULT_COMBAT_PROFILE_DELAY_MS = 150;
type SkillSlot = 0 | 1 | 2 | 3 | 4 | 5;
declare const SkillSlotSchema: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
declare const CombatProfileCooldownModeSchema: Schema.Literals<readonly ["use-if-ready", "wait-for-cooldown"]>;
type CombatProfileCooldownMode = typeof CombatProfileCooldownModeSchema.Type;
declare const CombatProfileThresholdUnitSchema: Schema.Literals<readonly ["percent", "value"]>;
type CombatProfileThresholdUnit = typeof CombatProfileThresholdUnitSchema.Type;
declare const CombatProfileComparisonSchema: Schema.Literals<readonly ["<=", ">="]>;
type CombatProfileComparison = typeof CombatProfileComparisonSchema.Type;
declare const CombatProfileStatConditionSchema: Schema.Struct<{
  readonly type: Schema.Literals<readonly ["self-hp", "self-mp", "ally-hp"]>;
  readonly op: Schema.Literals<readonly ["<=", ">="]>;
  readonly value: Schema.Int;
  readonly unit: Schema.Literals<readonly ["percent", "value"]>;
}>;
type CombatProfileStatCondition = typeof CombatProfileStatConditionSchema.Type;
declare const CombatProfileAuraConditionSchema: Schema.Struct<{
  readonly type: Schema.Literals<readonly ["self-aura", "target-aura"]>;
  readonly auraName: Schema.decodeTo<Schema.String, Schema.String, never, never>;
  readonly op: Schema.Literals<readonly ["<=", ">="]>;
  readonly value: Schema.Int;
}>;
type CombatProfileAuraCondition = typeof CombatProfileAuraConditionSchema.Type;
declare const CombatProfileConditionSchema: Schema.Union<readonly [Schema.Struct<{
  readonly type: Schema.Literals<readonly ["self-hp", "self-mp", "ally-hp"]>;
  readonly op: Schema.Literals<readonly ["<=", ">="]>;
  readonly value: Schema.Int;
  readonly unit: Schema.Literals<readonly ["percent", "value"]>;
}>, Schema.Struct<{
  readonly type: Schema.Literals<readonly ["self-aura", "target-aura"]>;
  readonly auraName: Schema.decodeTo<Schema.String, Schema.String, never, never>;
  readonly op: Schema.Literals<readonly ["<=", ">="]>;
  readonly value: Schema.Int;
}>]>;
type CombatProfileCondition = typeof CombatProfileConditionSchema.Type;
declare const CombatProfileStepSchema: Schema.Struct<{
  readonly skill: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
  readonly conditions: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
    readonly type: Schema.Literals<readonly ["self-hp", "self-mp", "ally-hp"]>;
    readonly op: Schema.Literals<readonly ["<=", ">="]>;
    readonly value: Schema.Int;
    readonly unit: Schema.Literals<readonly ["percent", "value"]>;
  }>, Schema.Struct<{
    readonly type: Schema.Literals<readonly ["self-aura", "target-aura"]>;
    readonly auraName: Schema.decodeTo<Schema.String, Schema.String, never, never>;
    readonly op: Schema.Literals<readonly ["<=", ">="]>;
    readonly value: Schema.Int;
  }>]>>;
  readonly priority: Schema.optionalKey<Schema.Boolean>;
  readonly cooldownMode: Schema.optionalKey<Schema.Literals<readonly ["use-if-ready", "wait-for-cooldown"]>>;
  readonly waitMs: Schema.optionalKey<Schema.Int>;
}>;
type CombatProfileStep = typeof CombatProfileStepSchema.Type;
declare const CombatProfileMessageTriggerSourceSchema: Schema.Literals<readonly ["any", "animation", "aura"]>;
type CombatProfileMessageTriggerSource = typeof CombatProfileMessageTriggerSourceSchema.Type;
declare const CombatProfileMessageTriggerSchema: Schema.Struct<{
  readonly messageIncludes: Schema.decodeTo<Schema.String, Schema.String, never, never>;
  readonly skill: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
  readonly source: Schema.Literals<readonly ["any", "animation", "aura"]>;
  readonly cooldownMs: Schema.optionalKey<Schema.Int>;
}>;
type CombatProfileMessageTrigger = typeof CombatProfileMessageTriggerSchema.Type;
declare const CombatProfileSchema: Schema.Struct<{
  readonly id: Schema.decodeTo<Schema.String, Schema.String, never, never>;
  readonly label: Schema.decodeTo<Schema.String, Schema.String, never, never>;
  readonly classNames: Schema.optionalKey<Schema.$Array<Schema.decodeTo<Schema.String, Schema.String, never, never>>>;
  readonly consumable: Schema.optionalKey<Schema.decodeTo<Schema.String, Schema.String, never, never>>;
  readonly delayMs: Schema.Int;
  readonly cooldownMode: Schema.Literals<readonly ["use-if-ready", "wait-for-cooldown"]>;
  readonly resetSkillIndexOnTargetDeath: Schema.optionalKey<Schema.Boolean>;
  readonly steps: Schema.$Array<Schema.Struct<{
    readonly skill: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
    readonly conditions: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
      readonly type: Schema.Literals<readonly ["self-hp", "self-mp", "ally-hp"]>;
      readonly op: Schema.Literals<readonly ["<=", ">="]>;
      readonly value: Schema.Int;
      readonly unit: Schema.Literals<readonly ["percent", "value"]>;
    }>, Schema.Struct<{
      readonly type: Schema.Literals<readonly ["self-aura", "target-aura"]>;
      readonly auraName: Schema.decodeTo<Schema.String, Schema.String, never, never>;
      readonly op: Schema.Literals<readonly ["<=", ">="]>;
      readonly value: Schema.Int;
    }>]>>;
    readonly priority: Schema.optionalKey<Schema.Boolean>;
    readonly cooldownMode: Schema.optionalKey<Schema.Literals<readonly ["use-if-ready", "wait-for-cooldown"]>>;
    readonly waitMs: Schema.optionalKey<Schema.Int>;
  }>>;
  readonly messageTriggers: Schema.optionalKey<Schema.$Array<Schema.Struct<{
    readonly messageIncludes: Schema.decodeTo<Schema.String, Schema.String, never, never>;
    readonly skill: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
    readonly source: Schema.Literals<readonly ["any", "animation", "aura"]>;
    readonly cooldownMs: Schema.optionalKey<Schema.Int>;
  }>>>;
}>;
type CombatProfile = typeof CombatProfileSchema.Type;
declare const CombatProfileLibrarySchema: Schema.Struct<{
  readonly version: Schema.Literal<1>;
  readonly profiles: Schema.$Array<Schema.Struct<{
    readonly id: Schema.decodeTo<Schema.String, Schema.String, never, never>;
    readonly label: Schema.decodeTo<Schema.String, Schema.String, never, never>;
    readonly classNames: Schema.optionalKey<Schema.$Array<Schema.decodeTo<Schema.String, Schema.String, never, never>>>;
    readonly consumable: Schema.optionalKey<Schema.decodeTo<Schema.String, Schema.String, never, never>>;
    readonly delayMs: Schema.Int;
    readonly cooldownMode: Schema.Literals<readonly ["use-if-ready", "wait-for-cooldown"]>;
    readonly resetSkillIndexOnTargetDeath: Schema.optionalKey<Schema.Boolean>;
    readonly steps: Schema.$Array<Schema.Struct<{
      readonly skill: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
      readonly conditions: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
        readonly type: Schema.Literals<readonly ["self-hp", "self-mp", "ally-hp"]>;
        readonly op: Schema.Literals<readonly ["<=", ">="]>;
        readonly value: Schema.Int;
        readonly unit: Schema.Literals<readonly ["percent", "value"]>;
      }>, Schema.Struct<{
        readonly type: Schema.Literals<readonly ["self-aura", "target-aura"]>;
        readonly auraName: Schema.decodeTo<Schema.String, Schema.String, never, never>;
        readonly op: Schema.Literals<readonly ["<=", ">="]>;
        readonly value: Schema.Int;
      }>]>>;
      readonly priority: Schema.optionalKey<Schema.Boolean>;
      readonly cooldownMode: Schema.optionalKey<Schema.Literals<readonly ["use-if-ready", "wait-for-cooldown"]>>;
      readonly waitMs: Schema.optionalKey<Schema.Int>;
    }>>;
    readonly messageTriggers: Schema.optionalKey<Schema.$Array<Schema.Struct<{
      readonly messageIncludes: Schema.decodeTo<Schema.String, Schema.String, never, never>;
      readonly skill: Schema.Literals<readonly [0, 1, 2, 3, 4, 5]>;
      readonly source: Schema.Literals<readonly ["any", "animation", "aura"]>;
      readonly cooldownMs: Schema.optionalKey<Schema.Int>;
    }>>>;
  }>>;
}>;
type CombatProfileLibrary = typeof CombatProfileLibrarySchema.Type;
type CombatProfileStepDefinition = Partial<CombatProfileStep> & {
  readonly skill: number;
};
type CombatProfileMessageTriggerDefinition = Partial<CombatProfileMessageTrigger> & {
  readonly messageIncludes: string;
  readonly skill: number;
};
interface CombatProfileDefinition extends Partial<Omit<CombatProfile, "steps" | "messageTriggers">> {
  readonly steps: readonly CombatProfileStepDefinition[];
  readonly messageTriggers?: readonly CombatProfileMessageTriggerDefinition[];
}
declare class CombatProfileNormalizationError extends Error {
  readonly _tag = "CombatProfileNormalizationError";
  constructor(message: string);
}
declare const DEFAULT_COMBAT_PROFILE_LIBRARY: CombatProfileLibrary;
declare const cloneCombatProfileLibrary: (library: CombatProfileLibrary) => CombatProfileLibrary;
declare const normalizeCombatProfile: (value: unknown) => CombatProfile;
declare const isCombatProfileDefinition: (value: unknown) => value is CombatProfileDefinition;
declare const normalizeCombatProfileLibrary: (value: unknown) => CombatProfileLibrary;
declare const findCombatProfileById: (library: CombatProfileLibrary, profileId: string) => CombatProfile | undefined;
declare const duplicateCombatProfile: (profile: CombatProfile, profiles: readonly CombatProfile[], createId: () => string) => CombatProfile;
declare const getCombatProfileById: (library: CombatProfileLibrary, profileId: string) => CombatProfile;
declare const serializeCombatProfileLibrary: (library: CombatProfileLibrary) => CombatProfileLibrary;
export { COMBAT_PROFILE_LIBRARY_VERSION, CombatProfile, CombatProfileAuraCondition, CombatProfileAuraConditionSchema, CombatProfileComparison, CombatProfileComparisonSchema, CombatProfileCondition, CombatProfileConditionSchema, CombatProfileCooldownMode, CombatProfileCooldownModeSchema, CombatProfileDefinition, CombatProfileLibrary, CombatProfileLibrarySchema, CombatProfileMessageTrigger, CombatProfileMessageTriggerDefinition, CombatProfileMessageTriggerSchema, CombatProfileMessageTriggerSource, CombatProfileMessageTriggerSourceSchema, CombatProfileNormalizationError, CombatProfileSchema, CombatProfileStatCondition, CombatProfileStatConditionSchema, CombatProfileStep, CombatProfileStepDefinition, CombatProfileStepSchema, CombatProfileThresholdUnit, CombatProfileThresholdUnitSchema, DEFAULT_COMBAT_PROFILE_DELAY_MS, DEFAULT_COMBAT_PROFILE_ID, DEFAULT_COMBAT_PROFILE_LIBRARY, SkillSlot, SkillSlotSchema, cloneCombatProfileLibrary, duplicateCombatProfile, findCombatProfileById, getCombatProfileById, isCombatProfileDefinition, normalizeCombatProfile, normalizeCombatProfileLibrary, serializeCombatProfileLibrary };