import * as Schema from "effect/Schema";
declare const EnvironmentItemBuckets: readonly ["ac-member", "ac-non-member", "non-ac-member", "non-ac-non-member"];
declare const EnvironmentItemBucketSchema: Schema.Literals<readonly ["ac-member", "ac-non-member", "non-ac-member", "non-ac-non-member"]>;
declare const EnvironmentItemRulesSchema: Schema.Struct<{
  readonly buckets: Schema.$Array<Schema.Literals<readonly ["ac-member", "ac-non-member", "non-ac-member", "non-ac-non-member"]>>;
  readonly rejectElse: Schema.Boolean;
}>;
declare const EnvironmentDropPolicySchema: Schema.Struct<{
  /** Accept member-only AC-tagged items. */
  readonly acceptAcMemberOnlyDrops: Schema.Boolean;
  /** Accept non-member AC-tagged items. */
  readonly acceptAcNonMemberDrops: Schema.Boolean;
  /** Accept member-only non-AC items. */
  readonly acceptNonAcMemberOnlyDrops: Schema.Boolean;
  /** Accept non-member non-AC items. */
  readonly acceptNonAcNonMemberDrops: Schema.Boolean;
  /** Reject any unregistered drop that is not accepted by this policy. */
  readonly rejectUnregisteredDrops: Schema.Boolean;
}>;
declare const EnvironmentQuestAutoRegisterOptionsSchema: Schema.Struct<{
  readonly requirements: Schema.Boolean;
  readonly rewards: Schema.Boolean;
}>;
declare const EnvironmentAutomationOptionsSchema: Schema.Struct<{
  readonly boosts: Schema.Boolean;
  readonly drops: Schema.Boolean;
  readonly quests: Schema.Boolean;
}>;
declare const EnvironmentAutomationCapabilities: readonly ["boosts", "drops", "quests"];
declare const EnvironmentAutomationCapabilitySchema: Schema.Literals<readonly ["boosts", "drops", "quests"]>;
declare const EnvironmentDropItemDataSchema: Schema.Struct<{
  readonly coins: Schema.Boolean;
  readonly memberOnly: Schema.Boolean;
  readonly name: Schema.String;
}>;
declare const EnvironmentStateSchema: Schema.Struct<{
  readonly automation: Schema.Struct<{
    readonly boosts: Schema.Boolean;
    readonly drops: Schema.Boolean;
    readonly quests: Schema.Boolean;
  }>;
  readonly questIds: Schema.$Array<Schema.Int>;
  readonly questAutoRegister: Schema.Struct<{
    readonly requirements: Schema.Boolean;
    readonly rewards: Schema.Boolean;
  }>;
  readonly questRewards: Schema.$Record<Schema.Int, Schema.Int>;
  readonly itemNames: Schema.$Array<Schema.String>;
  readonly itemNotificationNames: Schema.$Array<Schema.String>;
  readonly itemRules: Schema.Struct<{
    readonly buckets: Schema.$Array<Schema.Literals<readonly ["ac-member", "ac-non-member", "non-ac-member", "non-ac-non-member"]>>;
    readonly rejectElse: Schema.Boolean;
  }>;
  readonly boosts: Schema.$Array<Schema.String>;
}>;
type EnvironmentItemBucket = typeof EnvironmentItemBucketSchema.Type;
/**
 * @scriptingExpandSchema
 */
type EnvironmentItemRules = typeof EnvironmentItemRulesSchema.Type;
/**
 * Handling options for drops that are not registered by name.
 *
 * @scriptingExpandSchema
 */
type EnvironmentDropPolicy = typeof EnvironmentDropPolicySchema.Type;
/**
 * @scriptingExpandSchema
 */
type EnvironmentQuestAutoRegisterOptions = typeof EnvironmentQuestAutoRegisterOptionsSchema.Type;
/**
 * @scriptingExpandSchema
 */
type EnvironmentAutomationOptions = typeof EnvironmentAutomationOptionsSchema.Type;
type EnvironmentAutomationCapability = typeof EnvironmentAutomationCapabilitySchema.Type;
/**
 * @scriptingExpandSchema
 */
type EnvironmentDropItemData = typeof EnvironmentDropItemDataSchema.Type;
/**
 * @scriptingExpandSchema
 */
type EnvironmentState = typeof EnvironmentStateSchema.Type;
type EnvironmentDropAction = "accept" | "reject" | "ignore";
declare const DEFAULT_ENVIRONMENT_ITEM_RULES: EnvironmentItemRules;
declare const DEFAULT_ENVIRONMENT_DROP_POLICY: EnvironmentDropPolicy;
declare const DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER: EnvironmentQuestAutoRegisterOptions;
declare const DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS: EnvironmentAutomationOptions;
declare const isEnvironmentItemBucket: (value: unknown) => value is EnvironmentItemBucket;
declare const normalizeEnvironmentItemRules: (rules: EnvironmentItemRules) => EnvironmentItemRules;
declare const environmentItemRulesToDropPolicy: (rules: EnvironmentItemRules) => EnvironmentDropPolicy;
declare const environmentDropPolicyToItemRules: (policy: EnvironmentDropPolicy) => EnvironmentItemRules;
declare const patchEnvironmentDropPolicy: (rules: EnvironmentItemRules, patch: Partial<EnvironmentDropPolicy>) => EnvironmentItemRules;
declare const normalizeEnvironmentQuestAutoRegisterOptions: (options: EnvironmentQuestAutoRegisterOptions) => EnvironmentQuestAutoRegisterOptions;
declare const normalizeEnvironmentAutomationOptions: (options: EnvironmentAutomationOptions) => EnvironmentAutomationOptions;
declare const isEnvironmentItemRules: <I>(input: I) => input is I & {
  readonly buckets: readonly ("ac-member" | "ac-non-member" | "non-ac-member" | "non-ac-non-member")[];
  readonly rejectElse: boolean;
};
declare const isEnvironmentQuestAutoRegisterOptions: <I>(input: I) => input is I & {
  readonly requirements: boolean;
  readonly rewards: boolean;
};
declare const classifyEnvironmentDropItem: (item: EnvironmentDropItemData) => EnvironmentItemBucket;
declare const hasEnvironmentItemName: (state: Pick<EnvironmentState, "itemNames">, itemName: string) => boolean;
declare const resolveEnvironmentDropAction: (state: Pick<EnvironmentState, "itemNames" | "itemRules">, item: EnvironmentDropItemData) => EnvironmentDropAction;
declare const createEmptyEnvironmentState: () => EnvironmentState;
declare const normalizeEnvironmentState: (state: EnvironmentState) => EnvironmentState;
declare const addEnvironmentQuest: (state: EnvironmentState, questId: number, rewardItemId?: number) => EnvironmentState;
interface EnvironmentQuestRegistration {
  readonly questId: number;
  readonly rewardItemId?: number;
}
declare const addEnvironmentQuests: (state: EnvironmentState, quests: readonly EnvironmentQuestRegistration[]) => EnvironmentState;
declare const removeEnvironmentQuest: (state: EnvironmentState, questId: number) => EnvironmentState;
declare const setEnvironmentQuestReward: (state: EnvironmentState, questId: number, rewardItemId: number) => EnvironmentState;
declare const clearEnvironmentQuestReward: (state: EnvironmentState, questId: number) => EnvironmentState;
declare const clearEnvironmentQuests: (state: EnvironmentState) => EnvironmentState;
declare const setEnvironmentQuestAutoRegisterOptions: (state: EnvironmentState, questAutoRegister: EnvironmentQuestAutoRegisterOptions) => EnvironmentState;
declare const setEnvironmentAutomationOptions: (state: EnvironmentState, automation: EnvironmentAutomationOptions) => EnvironmentState;
declare const setEnvironmentAutomationEnabled: (state: EnvironmentState, capability: EnvironmentAutomationCapability, enabled: boolean) => EnvironmentState;
declare const addEnvironmentItem: (state: EnvironmentState, name: string) => EnvironmentState;
declare const addEnvironmentItems: (state: EnvironmentState, names: readonly string[]) => EnvironmentState;
declare const removeEnvironmentItem: (state: EnvironmentState, name: string) => EnvironmentState;
declare const setEnvironmentItemRules: (state: EnvironmentState, itemRules: EnvironmentItemRules) => EnvironmentState;
declare const setEnvironmentItemNotification: (state: EnvironmentState, name: string, enabled: boolean) => EnvironmentState;
declare const setEnvironmentDropPolicy: (state: EnvironmentState, policy: Partial<EnvironmentDropPolicy>) => EnvironmentState;
declare const clearEnvironmentItems: (state: EnvironmentState) => EnvironmentState;
declare const addEnvironmentBoost: (state: EnvironmentState, name: string) => EnvironmentState;
declare const addEnvironmentBoosts: (state: EnvironmentState, boosts: readonly string[]) => EnvironmentState;
declare const removeEnvironmentBoost: (state: EnvironmentState, name: string) => EnvironmentState;
declare const clearEnvironmentBoosts: (state: EnvironmentState) => EnvironmentState;
declare const clearEnvironmentState: (state: EnvironmentState) => EnvironmentState;
declare const areEnvironmentStatesEqual: (left: EnvironmentState, right: EnvironmentState) => boolean;
export { DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS, DEFAULT_ENVIRONMENT_DROP_POLICY, DEFAULT_ENVIRONMENT_ITEM_RULES, DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER, EnvironmentAutomationCapabilities, EnvironmentAutomationCapability, EnvironmentAutomationCapabilitySchema, EnvironmentAutomationOptions, EnvironmentAutomationOptionsSchema, EnvironmentDropAction, EnvironmentDropItemData, EnvironmentDropItemDataSchema, EnvironmentDropPolicy, EnvironmentDropPolicySchema, EnvironmentItemBucket, EnvironmentItemBucketSchema, EnvironmentItemBuckets, EnvironmentItemRules, EnvironmentItemRulesSchema, EnvironmentQuestAutoRegisterOptions, EnvironmentQuestAutoRegisterOptionsSchema, EnvironmentQuestRegistration, EnvironmentState, EnvironmentStateSchema, addEnvironmentBoost, addEnvironmentBoosts, addEnvironmentItem, addEnvironmentItems, addEnvironmentQuest, addEnvironmentQuests, areEnvironmentStatesEqual, classifyEnvironmentDropItem, clearEnvironmentBoosts, clearEnvironmentItems, clearEnvironmentQuestReward, clearEnvironmentQuests, clearEnvironmentState, createEmptyEnvironmentState, environmentDropPolicyToItemRules, environmentItemRulesToDropPolicy, hasEnvironmentItemName, isEnvironmentItemBucket, isEnvironmentItemRules, isEnvironmentQuestAutoRegisterOptions, normalizeEnvironmentAutomationOptions, normalizeEnvironmentItemRules, normalizeEnvironmentQuestAutoRegisterOptions, normalizeEnvironmentState, patchEnvironmentDropPolicy, removeEnvironmentBoost, removeEnvironmentItem, removeEnvironmentQuest, resolveEnvironmentDropAction, setEnvironmentAutomationEnabled, setEnvironmentAutomationOptions, setEnvironmentDropPolicy, setEnvironmentItemNotification, setEnvironmentItemRules, setEnvironmentQuestAutoRegisterOptions, setEnvironmentQuestReward };