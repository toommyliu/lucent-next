import type { EnvironmentEdit, EnvironmentState } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";
import type { EnvironmentShape } from "../game/environment/Environment";

export const makeEnvironmentService = (
  host: Pick<typeof __host, "read" | "command">,
  unavailable: Pick<EnvironmentShape, "fetchBoosts" | "syncToAll">,
): EnvironmentShape => {
  const identifier = (value: number) => Number.isFinite(value) ? value : 0;
  const getState = () => Effect.sync(() => JSON.parse(host.read('{"kind":"environment"}')) as EnvironmentState);
  const set = (edit: EnvironmentEdit) => Effect.try(() => JSON.parse(host.command(JSON.stringify({ kind: "updateEnvironment", edit }))) as EnvironmentState);
  const setAutomation = (capability: "boosts" | "drops" | "quests", enabled: boolean) => set({ kind: "setAutomationEnabled", capability, enabled });
  const setDropPolicy: EnvironmentShape["setDropPolicy"] = patch => set({ kind: "updateDropPolicy", patch });
  const setQuestAutoRegister: EnvironmentShape["setQuestAutoRegister"] = options => set({ kind: "setQuestAutoRegister", options });
  return {
    ...unavailable,
    getState,
    clear: () => set({ kind: "clear" }),
    addQuest: (questId, rewardItemId) => set({ kind: "addQuest", questId: identifier(questId), ...(rewardItemId === undefined ? {} : { rewardItemId: identifier(rewardItemId) }) }),
    removeQuest: questId => set({ kind: "removeQuest", questId: identifier(questId) }),
    setQuestReward: (questId, rewardItemId) => set({ kind: "setQuestReward", questId: identifier(questId), rewardItemId: identifier(rewardItemId) }),
    clearQuestReward: questId => set({ kind: "clearQuestReward", questId: identifier(questId) }),
    clearQuests: () => set({ kind: "clearQuests" }),
    setQuestAutoRegister,
    setAutoRegisterRequirements: requirements => getState().pipe(Effect.flatMap(state => setQuestAutoRegister({ ...state.questAutoRegister, requirements }))),
    setAutoRegisterRewards: rewards => getState().pipe(Effect.flatMap(state => setQuestAutoRegister({ ...state.questAutoRegister, rewards }))),
    setBoostAutomationEnabled: enabled => setAutomation("boosts", enabled),
    setDropAutomationEnabled: enabled => setAutomation("drops", enabled),
    setQuestAutomationEnabled: enabled => setAutomation("quests", enabled),
    addItem: name => set({ kind: "addItem", name }),
    removeItem: name => set({ kind: "removeItem", name }),
    clearItems: () => set({ kind: "clearItems" }),
    setItemNotification: (name, enabled) => set({ kind: "setItemNotification", name, enabled }),
    setItemRules: rules => set({ kind: "setItemRules", rules: { ...rules, buckets: [...rules.buckets] } }),
    setDropPolicy,
    setAcceptAcMemberOnlyDrops: enabled => setDropPolicy({ acceptAcMemberOnlyDrops: enabled }),
    setAcceptAcNonMemberDrops: enabled => setDropPolicy({ acceptAcNonMemberDrops: enabled }),
    setAcceptNonAcMemberOnlyDrops: enabled => setDropPolicy({ acceptNonAcMemberOnlyDrops: enabled }),
    setAcceptNonAcNonMemberDrops: enabled => setDropPolicy({ acceptNonAcNonMemberDrops: enabled }),
    setRejectUnregisteredDrops: enabled => setDropPolicy({ rejectUnregisteredDrops: enabled }),
    addBoost: name => set({ kind: "addBoost", name }),
    removeBoost: name => set({ kind: "removeBoost", name }),
    clearBoosts: () => set({ kind: "clearBoosts" }),
  };
};
