import { PositiveInt, TrimmedNonEmptyString } from "./baseSchemas.mjs";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";
import * as SchemaTransformation from "effect/SchemaTransformation";
// src/environment.ts
const EnvironmentItemBuckets = [
	"ac-member",
	"ac-non-member",
	"non-ac-member",
	"non-ac-non-member"
];
const EnvironmentItemBucketSchema = Schema.Literals(EnvironmentItemBuckets);
const EnvironmentItemRulesSchema = Schema.Struct({
	buckets: Schema.Array(EnvironmentItemBucketSchema),
	rejectElse: Schema.Boolean
});
const EnvironmentDropPolicySchema = Schema.Struct({
	/** Accept member-only AC-tagged items. */
	acceptAcMemberOnlyDrops: Schema.Boolean,
	/** Accept non-member AC-tagged items. */
	acceptAcNonMemberDrops: Schema.Boolean,
	/** Accept member-only non-AC items. */
	acceptNonAcMemberOnlyDrops: Schema.Boolean,
	/** Accept non-member non-AC items. */
	acceptNonAcNonMemberDrops: Schema.Boolean,
	/** Reject any unregistered drop that is not accepted by this policy. */
	rejectUnregisteredDrops: Schema.Boolean
});
const EnvironmentQuestAutoRegisterOptionsSchema = Schema.Struct({
	requirements: Schema.Boolean,
	rewards: Schema.Boolean
});
const EnvironmentAutomationOptionsSchema = Schema.Struct({
	boosts: Schema.Boolean,
	drops: Schema.Boolean,
	quests: Schema.Boolean
});
const EnvironmentAutomationCapabilities = [
	"boosts",
	"drops",
	"quests"
];
const EnvironmentAutomationCapabilitySchema = Schema.Literals(EnvironmentAutomationCapabilities);
const EnvironmentDropItemDataSchema = Schema.Struct({
	coins: Schema.Boolean,
	memberOnly: Schema.Boolean,
	name: Schema.String
});
const EnvironmentStateSchema = Schema.Struct({
	automation: EnvironmentAutomationOptionsSchema,
	questIds: Schema.Array(PositiveInt),
	questAutoRegister: EnvironmentQuestAutoRegisterOptionsSchema,
	questRewards: Schema.Record(PositiveInt, PositiveInt),
	itemNames: Schema.Array(Schema.String),
	itemNotificationNames: Schema.Array(Schema.String),
	itemRules: EnvironmentItemRulesSchema,
	boosts: Schema.Array(Schema.String)
});
const DEFAULT_ENVIRONMENT_ITEM_RULES = {
	buckets: [],
	rejectElse: false
};
const DEFAULT_ENVIRONMENT_DROP_POLICY = {
	acceptAcMemberOnlyDrops: false,
	acceptAcNonMemberDrops: false,
	acceptNonAcMemberOnlyDrops: false,
	acceptNonAcNonMemberDrops: false,
	rejectUnregisteredDrops: false
};
const DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER = {
	requirements: false,
	rewards: false
};
const DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS = {
	boosts: true,
	drops: true,
	quests: true
};
const PositiveIntFromNumber = Schema.Number.pipe(Schema.decodeTo(PositiveInt, SchemaTransformation.transform({
	decode: Math.trunc,
	encode: (value) => value
})));
const decodeQuestIdentifier = Schema.decodeUnknownOption(PositiveIntFromNumber);
const decodeTrimmedName = Schema.decodeUnknownOption(TrimmedNonEmptyString);
const isEnvironmentItemBucketValue = Schema.is(EnvironmentItemBucketSchema);
const toPositiveInt = (value) => Option.getOrUndefined(decodeQuestIdentifier(value));
const normalizeNames = (values) => {
	const names = /* @__PURE__ */ new Map();
	for (const value of values) {
		const decoded = decodeTrimmedName(value);
		if (Option.isSome(decoded)) {
			const key = decoded.value.toLowerCase();
			if (!names.has(key)) names.set(key, decoded.value);
		}
	}
	return Array.from(names.values()).toSorted((left, right) => left.localeCompare(right));
};
const normalizeQuestIds = (values) => Array.from(new Set(values.flatMap((value) => toPositiveInt(value) ?? []))).toSorted((left, right) => left - right);
const normalizeQuestRewards = (questIds, rewards) => {
	const registered = new Set(questIds);
	return Object.fromEntries(Object.entries(rewards).flatMap(([questId, rewardItemId]) => {
		const normalizedQuestId = toPositiveInt(Number(questId));
		const normalizedRewardItemId = toPositiveInt(rewardItemId);
		return normalizedQuestId !== void 0 && normalizedRewardItemId !== void 0 && registered.has(normalizedQuestId) ? [[normalizedQuestId, normalizedRewardItemId]] : [];
	}).toSorted(([left], [right]) => left - right));
};
const isEnvironmentItemBucket = (value) => isEnvironmentItemBucketValue(value);
const normalizeEnvironmentItemRules = (rules) => {
	const requested = new Set(rules.buckets.filter(isEnvironmentItemBucketValue));
	return {
		buckets: EnvironmentItemBuckets.filter((bucket) => requested.has(bucket)),
		rejectElse: rules.rejectElse
	};
};
const bucketPolicyKeys = {
	"ac-member": "acceptAcMemberOnlyDrops",
	"ac-non-member": "acceptAcNonMemberDrops",
	"non-ac-member": "acceptNonAcMemberOnlyDrops",
	"non-ac-non-member": "acceptNonAcNonMemberDrops"
};
const environmentItemRulesToDropPolicy = (rules) => {
	const buckets = new Set(normalizeEnvironmentItemRules(rules).buckets);
	return {
		acceptAcMemberOnlyDrops: buckets.has("ac-member"),
		acceptAcNonMemberDrops: buckets.has("ac-non-member"),
		acceptNonAcMemberOnlyDrops: buckets.has("non-ac-member"),
		acceptNonAcNonMemberDrops: buckets.has("non-ac-non-member"),
		rejectUnregisteredDrops: rules.rejectElse
	};
};
const environmentDropPolicyToItemRules = (policy) => ({
	buckets: EnvironmentItemBuckets.filter((bucket) => policy[bucketPolicyKeys[bucket]]),
	rejectElse: policy.rejectUnregisteredDrops
});
const patchEnvironmentDropPolicy = (rules, patch) => environmentDropPolicyToItemRules({
	...environmentItemRulesToDropPolicy(rules),
	...patch
});
const normalizeEnvironmentQuestAutoRegisterOptions = (options) => ({
	requirements: options.requirements,
	rewards: options.rewards
});
const normalizeEnvironmentAutomationOptions = (options) => ({
	boosts: options.boosts,
	drops: options.drops,
	quests: options.quests
});
const isEnvironmentItemRules = Schema.is(EnvironmentItemRulesSchema);
const isEnvironmentQuestAutoRegisterOptions = Schema.is(EnvironmentQuestAutoRegisterOptionsSchema);
const classifyEnvironmentDropItem = (item) => {
	if (item.coins && item.memberOnly) return "ac-member";
	if (item.coins) return "ac-non-member";
	return item.memberOnly ? "non-ac-member" : "non-ac-non-member";
};
const hasEnvironmentItemName = (state, itemName) => {
	const key = itemName.trim().toLowerCase();
	return state.itemNames.some((registered) => registered.toLowerCase() === key);
};
const resolveEnvironmentDropAction = (state, item) => {
	if (hasEnvironmentItemName(state, item.name)) return "accept";
	if (state.itemRules.buckets.includes(classifyEnvironmentDropItem(item))) return "accept";
	return state.itemRules.rejectElse ? "reject" : "ignore";
};
const createEmptyEnvironmentState = () => ({
	automation: DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS,
	questIds: [],
	questAutoRegister: DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER,
	questRewards: {},
	itemNames: [],
	itemNotificationNames: [],
	itemRules: DEFAULT_ENVIRONMENT_ITEM_RULES,
	boosts: []
});
const normalizeEnvironmentState = (state) => {
	const questIds = normalizeQuestIds(state.questIds);
	const itemNames = normalizeNames(state.itemNames);
	const itemNameKeys = new Set(itemNames.map((name) => name.toLowerCase()));
	return {
		automation: normalizeEnvironmentAutomationOptions(state.automation),
		questIds,
		questAutoRegister: normalizeEnvironmentQuestAutoRegisterOptions(state.questAutoRegister),
		questRewards: normalizeQuestRewards(questIds, state.questRewards),
		itemNames,
		itemNotificationNames: normalizeNames(state.itemNotificationNames).filter((name) => itemNameKeys.has(name.toLowerCase())),
		itemRules: normalizeEnvironmentItemRules(state.itemRules),
		boosts: normalizeNames(state.boosts)
	};
};
const updateEnvironmentState = (state, patch) => normalizeEnvironmentState({
	...state,
	...patch
});
const addEnvironmentQuest = (state, questId, rewardItemId) => {
	const normalizedQuestId = toPositiveInt(questId);
	if (normalizedQuestId === void 0) return normalizeEnvironmentState(state);
	const normalizedRewardItemId = rewardItemId === void 0 ? void 0 : toPositiveInt(rewardItemId);
	return updateEnvironmentState(state, {
		questIds: [...state.questIds, normalizedQuestId],
		...normalizedRewardItemId === void 0 ? {} : { questRewards: {
			...state.questRewards,
			[normalizedQuestId]: normalizedRewardItemId
		} }
	});
};
const addEnvironmentQuests = (state, quests) => quests.reduce((current, quest) => addEnvironmentQuest(current, quest.questId, quest.rewardItemId), state);
const removeEnvironmentQuest = (state, questId) => {
	const normalizedQuestId = toPositiveInt(questId);
	if (normalizedQuestId === void 0) return normalizeEnvironmentState(state);
	const { [normalizedQuestId]: _removed, ...questRewards } = state.questRewards;
	return updateEnvironmentState(state, {
		questIds: state.questIds.filter((id) => id !== normalizedQuestId),
		questRewards
	});
};
const setEnvironmentQuestReward = (state, questId, rewardItemId) => {
	const normalizedQuestId = toPositiveInt(questId);
	const normalizedRewardItemId = toPositiveInt(rewardItemId);
	return normalizedQuestId === void 0 || normalizedRewardItemId === void 0 ? normalizeEnvironmentState(state) : addEnvironmentQuest(state, normalizedQuestId, normalizedRewardItemId);
};
const clearEnvironmentQuestReward = (state, questId) => {
	const normalizedQuestId = toPositiveInt(questId);
	if (normalizedQuestId === void 0) return normalizeEnvironmentState(state);
	const { [normalizedQuestId]: _removed, ...questRewards } = state.questRewards;
	return updateEnvironmentState(state, { questRewards });
};
const clearEnvironmentQuests = (state) => updateEnvironmentState(state, {
	questIds: [],
	questRewards: {}
});
const setEnvironmentQuestAutoRegisterOptions = (state, questAutoRegister) => updateEnvironmentState(state, { questAutoRegister });
const setEnvironmentAutomationOptions = (state, automation) => updateEnvironmentState(state, { automation });
const setEnvironmentAutomationEnabled = (state, capability, enabled) => setEnvironmentAutomationOptions(state, {
	...state.automation,
	[capability]: enabled
});
const addEnvironmentItem = (state, name) => updateEnvironmentState(state, { itemNames: [...state.itemNames, name] });
const addEnvironmentItems = (state, names) => updateEnvironmentState(state, { itemNames: [...state.itemNames, ...names] });
const removeEnvironmentItem = (state, name) => {
	const key = name.trim().toLowerCase();
	return updateEnvironmentState(state, { itemNames: state.itemNames.filter((registered) => registered.toLowerCase() !== key) });
};
const setEnvironmentItemRules = (state, itemRules) => updateEnvironmentState(state, { itemRules });
const setEnvironmentItemNotification = (state, name, enabled) => {
	const key = name.trim().toLowerCase();
	const registered = state.itemNames.find((itemName) => itemName.toLowerCase() === key);
	if (registered === void 0) return normalizeEnvironmentState(state);
	return updateEnvironmentState(state, { itemNotificationNames: enabled ? [...state.itemNotificationNames, registered] : state.itemNotificationNames.filter((itemName) => itemName.toLowerCase() !== key) });
};
const setEnvironmentDropPolicy = (state, policy) => setEnvironmentItemRules(state, patchEnvironmentDropPolicy(state.itemRules, policy));
const clearEnvironmentItems = (state) => updateEnvironmentState(state, {
	itemNames: [],
	itemNotificationNames: []
});
const addEnvironmentBoost = (state, name) => updateEnvironmentState(state, { boosts: [...state.boosts, name] });
const addEnvironmentBoosts = (state, boosts) => updateEnvironmentState(state, { boosts: [...state.boosts, ...boosts] });
const removeEnvironmentBoost = (state, name) => {
	const key = name.trim().toLowerCase();
	return updateEnvironmentState(state, { boosts: state.boosts.filter((registered) => registered.toLowerCase() !== key) });
};
const clearEnvironmentBoosts = (state) => updateEnvironmentState(state, { boosts: [] });
const clearEnvironmentState = (state) => updateEnvironmentState(state, {
	boosts: [],
	itemNames: [],
	itemNotificationNames: [],
	questIds: [],
	questRewards: {}
});
const areEnvironmentStatesEqual = (left, right) => JSON.stringify(normalizeEnvironmentState(left)) === JSON.stringify(normalizeEnvironmentState(right));
export { DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS, DEFAULT_ENVIRONMENT_DROP_POLICY, DEFAULT_ENVIRONMENT_ITEM_RULES, DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER, EnvironmentAutomationCapabilities, EnvironmentAutomationCapabilitySchema, EnvironmentAutomationOptionsSchema, EnvironmentDropItemDataSchema, EnvironmentDropPolicySchema, EnvironmentItemBucketSchema, EnvironmentItemBuckets, EnvironmentItemRulesSchema, EnvironmentQuestAutoRegisterOptionsSchema, EnvironmentStateSchema, addEnvironmentBoost, addEnvironmentBoosts, addEnvironmentItem, addEnvironmentItems, addEnvironmentQuest, addEnvironmentQuests, areEnvironmentStatesEqual, classifyEnvironmentDropItem, clearEnvironmentBoosts, clearEnvironmentItems, clearEnvironmentQuestReward, clearEnvironmentQuests, clearEnvironmentState, createEmptyEnvironmentState, environmentDropPolicyToItemRules, environmentItemRulesToDropPolicy, hasEnvironmentItemName, isEnvironmentItemBucket, isEnvironmentItemRules, isEnvironmentQuestAutoRegisterOptions, normalizeEnvironmentAutomationOptions, normalizeEnvironmentItemRules, normalizeEnvironmentQuestAutoRegisterOptions, normalizeEnvironmentState, patchEnvironmentDropPolicy, removeEnvironmentBoost, removeEnvironmentItem, removeEnvironmentQuest, resolveEnvironmentDropAction, setEnvironmentAutomationEnabled, setEnvironmentAutomationOptions, setEnvironmentDropPolicy, setEnvironmentItemNotification, setEnvironmentItemRules, setEnvironmentQuestAutoRegisterOptions, setEnvironmentQuestReward };
