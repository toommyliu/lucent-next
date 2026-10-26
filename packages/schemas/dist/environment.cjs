Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_baseSchemas = require("./baseSchemas.cjs");
let effect_Option = require("effect/Option");
effect_Option = require_rolldown_runtime.__toESM(effect_Option);
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
let effect_SchemaTransformation = require("effect/SchemaTransformation");
effect_SchemaTransformation = require_rolldown_runtime.__toESM(effect_SchemaTransformation);
// src/environment.ts
const EnvironmentItemBuckets = [
	"ac-member",
	"ac-non-member",
	"non-ac-member",
	"non-ac-non-member"
];
const EnvironmentItemBucketSchema = effect_Schema.Literals(EnvironmentItemBuckets);
const EnvironmentItemRulesSchema = effect_Schema.Struct({
	buckets: effect_Schema.Array(EnvironmentItemBucketSchema),
	rejectElse: effect_Schema.Boolean
});
const EnvironmentDropPolicySchema = effect_Schema.Struct({
	/** Accept member-only AC-tagged items. */
	acceptAcMemberOnlyDrops: effect_Schema.Boolean,
	/** Accept non-member AC-tagged items. */
	acceptAcNonMemberDrops: effect_Schema.Boolean,
	/** Accept member-only non-AC items. */
	acceptNonAcMemberOnlyDrops: effect_Schema.Boolean,
	/** Accept non-member non-AC items. */
	acceptNonAcNonMemberDrops: effect_Schema.Boolean,
	/** Reject any unregistered drop that is not accepted by this policy. */
	rejectUnregisteredDrops: effect_Schema.Boolean
});
const EnvironmentQuestAutoRegisterOptionsSchema = effect_Schema.Struct({
	requirements: effect_Schema.Boolean,
	rewards: effect_Schema.Boolean
});
const EnvironmentAutomationOptionsSchema = effect_Schema.Struct({
	boosts: effect_Schema.Boolean,
	drops: effect_Schema.Boolean,
	quests: effect_Schema.Boolean
});
const EnvironmentAutomationCapabilities = [
	"boosts",
	"drops",
	"quests"
];
const EnvironmentAutomationCapabilitySchema = effect_Schema.Literals(EnvironmentAutomationCapabilities);
const EnvironmentDropItemDataSchema = effect_Schema.Struct({
	coins: effect_Schema.Boolean,
	memberOnly: effect_Schema.Boolean,
	name: effect_Schema.String
});
const EnvironmentStateSchema = effect_Schema.Struct({
	automation: EnvironmentAutomationOptionsSchema,
	questIds: effect_Schema.Array(require_baseSchemas.PositiveInt),
	questAutoRegister: EnvironmentQuestAutoRegisterOptionsSchema,
	questRewards: effect_Schema.Record(require_baseSchemas.PositiveInt, require_baseSchemas.PositiveInt),
	itemNames: effect_Schema.Array(effect_Schema.String),
	itemNotificationNames: effect_Schema.Array(effect_Schema.String),
	itemRules: EnvironmentItemRulesSchema,
	boosts: effect_Schema.Array(effect_Schema.String)
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
const PositiveIntFromNumber = effect_Schema.Number.pipe(effect_Schema.decodeTo(require_baseSchemas.PositiveInt, effect_SchemaTransformation.transform({
	decode: Math.trunc,
	encode: (value) => value
})));
const decodeQuestIdentifier = effect_Schema.decodeUnknownOption(PositiveIntFromNumber);
const decodeTrimmedName = effect_Schema.decodeUnknownOption(require_baseSchemas.TrimmedNonEmptyString);
const isEnvironmentItemBucketValue = effect_Schema.is(EnvironmentItemBucketSchema);
const toPositiveInt = (value) => effect_Option.getOrUndefined(decodeQuestIdentifier(value));
const normalizeNames = (values) => {
	const names = /* @__PURE__ */ new Map();
	for (const value of values) {
		const decoded = decodeTrimmedName(value);
		if (effect_Option.isSome(decoded)) {
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
const isEnvironmentItemRules = effect_Schema.is(EnvironmentItemRulesSchema);
const isEnvironmentQuestAutoRegisterOptions = effect_Schema.is(EnvironmentQuestAutoRegisterOptionsSchema);
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
exports.DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS = DEFAULT_ENVIRONMENT_AUTOMATION_OPTIONS;
exports.DEFAULT_ENVIRONMENT_DROP_POLICY = DEFAULT_ENVIRONMENT_DROP_POLICY;
exports.DEFAULT_ENVIRONMENT_ITEM_RULES = DEFAULT_ENVIRONMENT_ITEM_RULES;
exports.DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER = DEFAULT_ENVIRONMENT_QUEST_AUTO_REGISTER;
exports.EnvironmentAutomationCapabilities = EnvironmentAutomationCapabilities;
exports.EnvironmentAutomationCapabilitySchema = EnvironmentAutomationCapabilitySchema;
exports.EnvironmentAutomationOptionsSchema = EnvironmentAutomationOptionsSchema;
exports.EnvironmentDropItemDataSchema = EnvironmentDropItemDataSchema;
exports.EnvironmentDropPolicySchema = EnvironmentDropPolicySchema;
exports.EnvironmentItemBucketSchema = EnvironmentItemBucketSchema;
exports.EnvironmentItemBuckets = EnvironmentItemBuckets;
exports.EnvironmentItemRulesSchema = EnvironmentItemRulesSchema;
exports.EnvironmentQuestAutoRegisterOptionsSchema = EnvironmentQuestAutoRegisterOptionsSchema;
exports.EnvironmentStateSchema = EnvironmentStateSchema;
exports.addEnvironmentBoost = addEnvironmentBoost;
exports.addEnvironmentBoosts = addEnvironmentBoosts;
exports.addEnvironmentItem = addEnvironmentItem;
exports.addEnvironmentItems = addEnvironmentItems;
exports.addEnvironmentQuest = addEnvironmentQuest;
exports.addEnvironmentQuests = addEnvironmentQuests;
exports.areEnvironmentStatesEqual = areEnvironmentStatesEqual;
exports.classifyEnvironmentDropItem = classifyEnvironmentDropItem;
exports.clearEnvironmentBoosts = clearEnvironmentBoosts;
exports.clearEnvironmentItems = clearEnvironmentItems;
exports.clearEnvironmentQuestReward = clearEnvironmentQuestReward;
exports.clearEnvironmentQuests = clearEnvironmentQuests;
exports.clearEnvironmentState = clearEnvironmentState;
exports.createEmptyEnvironmentState = createEmptyEnvironmentState;
exports.environmentDropPolicyToItemRules = environmentDropPolicyToItemRules;
exports.environmentItemRulesToDropPolicy = environmentItemRulesToDropPolicy;
exports.hasEnvironmentItemName = hasEnvironmentItemName;
exports.isEnvironmentItemBucket = isEnvironmentItemBucket;
exports.isEnvironmentItemRules = isEnvironmentItemRules;
exports.isEnvironmentQuestAutoRegisterOptions = isEnvironmentQuestAutoRegisterOptions;
exports.normalizeEnvironmentAutomationOptions = normalizeEnvironmentAutomationOptions;
exports.normalizeEnvironmentItemRules = normalizeEnvironmentItemRules;
exports.normalizeEnvironmentQuestAutoRegisterOptions = normalizeEnvironmentQuestAutoRegisterOptions;
exports.normalizeEnvironmentState = normalizeEnvironmentState;
exports.patchEnvironmentDropPolicy = patchEnvironmentDropPolicy;
exports.removeEnvironmentBoost = removeEnvironmentBoost;
exports.removeEnvironmentItem = removeEnvironmentItem;
exports.removeEnvironmentQuest = removeEnvironmentQuest;
exports.resolveEnvironmentDropAction = resolveEnvironmentDropAction;
exports.setEnvironmentAutomationEnabled = setEnvironmentAutomationEnabled;
exports.setEnvironmentAutomationOptions = setEnvironmentAutomationOptions;
exports.setEnvironmentDropPolicy = setEnvironmentDropPolicy;
exports.setEnvironmentItemNotification = setEnvironmentItemNotification;
exports.setEnvironmentItemRules = setEnvironmentItemRules;
exports.setEnvironmentQuestAutoRegisterOptions = setEnvironmentQuestAutoRegisterOptions;
exports.setEnvironmentQuestReward = setEnvironmentQuestReward;
