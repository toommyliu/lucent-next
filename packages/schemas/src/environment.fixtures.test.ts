import { describe, expect, it } from "vitest";
import reducers from "../test-fixtures/environment/reducers.json";
import * as S from "./environment";
import type { EnvironmentEdit } from "../../game/src/protocol";
import order from "../test-fixtures/environment/order.json";
import fixtures from "../test-fixtures/environment/normalization.json";
import { clearEnvironmentState, normalizeEnvironmentState, type EnvironmentState } from "./environment";
describe("environment normalization corpus", () => {
  for (const fixture of fixtures) it(fixture.name, () => {
    const input = fixture.input as EnvironmentState;
    expect(JSON.stringify(fixture.clear ? clearEnvironmentState(input) : normalizeEnvironmentState(input))).toBe(JSON.stringify(fixture.expected));
  });
});

it("name ordering corpus matches Node", () => {
  for (const [left, right, expected] of order as [string, string, number][]) {
    expect(Math.sign(left.localeCompare(right, "en"))).toBe(expected);
  }
});

const replay = (s: EnvironmentState, e: EnvironmentEdit): EnvironmentState => {
  switch (e.kind) {
    case "clear": return S.clearEnvironmentState(s);
    case "addQuest": return S.addEnvironmentQuest(s, e.questId, e.rewardItemId);
    case "removeQuest": return S.removeEnvironmentQuest(s, e.questId);
    case "setQuestReward": return S.setEnvironmentQuestReward(s, e.questId, e.rewardItemId);
    case "clearQuestReward": return S.clearEnvironmentQuestReward(s, e.questId);
    case "clearQuests": return S.clearEnvironmentQuests(s);
    case "setQuestAutoRegister": return S.setEnvironmentQuestAutoRegisterOptions(s, { ...s.questAutoRegister, ...e });
    case "setAutomationEnabled": return S.setEnvironmentAutomationEnabled(s, e.capability, e.enabled);
    case "addItem": return S.addEnvironmentItem(s, e.name);
    case "removeItem": return S.removeEnvironmentItem(s, e.name);
    case "setItemNotification": return S.setEnvironmentItemNotification(s, e.name, e.enabled);
    case "setItemRules": return S.setEnvironmentItemRules(s, e.rules);
    case "updateDropPolicy": return S.setEnvironmentDropPolicy(s, e.patch);
    case "clearItems": return S.clearEnvironmentItems(s);
    case "addBoost": return S.addEnvironmentBoost(s, e.name);
    case "removeBoost": return S.removeEnvironmentBoost(s, e.name);
    case "clearBoosts": return S.clearEnvironmentBoosts(s);
    default: { const exhaustive: never = e; return exhaustive; }
  }
};

it("reducer corpus matches the schema including key order", () => {
  for (const sequence of reducers) {
    let state = sequence.input as EnvironmentState;
    for (const step of sequence.steps) {
      state = replay(state, step.edit as EnvironmentEdit);
      expect(JSON.stringify(state), JSON.stringify(step.edit)).toBe(JSON.stringify(step.expected));
    }
  }
});
