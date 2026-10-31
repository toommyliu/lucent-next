import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { tsImport } from "tsx/esm/api";
const S = await tsImport("../packages/schemas/src/environment.ts", import.meta.url);
let seed = 53;
const rnd = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
const pick = a => a[Math.floor(rnd() * a.length)];
const ids = [-1, 0, 0.5, 1, 2.7, 3, 4.9, 4, 802, 802.9, 9007199254740991, 9007199254740992, 4294967296];
const names = ["apple", "Apple", " APPLE ", "Banana", "banana ", "﻿Zorbak", "zorbak", "a-b", "ab", "a b", "A!", "a?", "!Blade", "[Blade]", "10th Token", "2nd Token", "Darkon's Receipt", "Darkon Receipt", "XP Boost", "xp boost", "Gold Boost", "", "   ", "\t\n", "a\u0001b", "ab\u007f", "_x", "~x", "Ab", "aB"];
const buckets = ["ac-member", "ac-non-member", "non-ac-member", "non-ac-non-member"];
const bool = () => rnd() < 0.5;
const kinds = {
  clear: () => [{ kind: "clear" }, s => S.clearEnvironmentState(s)],
  addQuest: () => { const q = pick(ids); const r = rnd() < 0.5 ? undefined : pick(ids); return [{ kind: "addQuest", questId: q, ...(r === undefined ? {} : { rewardItemId: r }) }, s => S.addEnvironmentQuest(s, q, r)]; },
  removeQuest: () => { const q = pick(ids); return [{ kind: "removeQuest", questId: q }, s => S.removeEnvironmentQuest(s, q)]; },
  setQuestReward: () => { const q = pick(ids), r = pick(ids); return [{ kind: "setQuestReward", questId: q, rewardItemId: r }, s => S.setEnvironmentQuestReward(s, q, r)]; },
  clearQuestReward: () => { const q = pick(ids); return [{ kind: "clearQuestReward", questId: q }, s => S.clearEnvironmentQuestReward(s, q)]; },
  clearQuests: () => [{ kind: "clearQuests" }, s => S.clearEnvironmentQuests(s)],
  setQuestAutoRegister: () => { const patch = bool() ? { requirements: bool() } : { rewards: bool() }; return [{ kind: "setQuestAutoRegister", ...patch }, s => S.setEnvironmentQuestAutoRegisterOptions(s, { ...s.questAutoRegister, ...patch })]; },
  setAutomationEnabled: () => { const c = pick(["boosts", "drops", "quests"]), e = bool(); return [{ kind: "setAutomationEnabled", capability: c, enabled: e }, s => S.setEnvironmentAutomationEnabled(s, c, e)]; },
  addItem: () => { const n = pick(names); return [{ kind: "addItem", name: n }, s => S.addEnvironmentItem(s, n)]; },
  addItem2: () => { const n = pick(names); return [{ kind: "addItem", name: n }, s => S.addEnvironmentItem(s, n)]; },
  removeItem: () => { const n = pick(names); return [{ kind: "removeItem", name: n }, s => S.removeEnvironmentItem(s, n)]; },
  setItemNotification: () => { const n = pick(names), e = rnd() < 0.7; return [{ kind: "setItemNotification", name: n, enabled: e }, s => S.setEnvironmentItemNotification(s, n, e)]; },
  setItemRules: () => { const r = { buckets: buckets.filter(() => bool()).reverse(), rejectElse: bool() }; return [{ kind: "setItemRules", rules: r }, s => S.setEnvironmentItemRules(s, r)]; },
  updateDropPolicy: () => { const p = {}; for (const k of ["acceptAcMemberOnlyDrops", "acceptAcNonMemberDrops", "acceptNonAcMemberOnlyDrops", "acceptNonAcNonMemberDrops", "rejectUnregisteredDrops"]) if (rnd() < 0.4) p[k] = bool(); return [{ kind: "updateDropPolicy", patch: p }, s => S.setEnvironmentDropPolicy(s, p)]; },
  clearItems: () => [{ kind: "clearItems" }, s => S.clearEnvironmentItems(s)],
  addBoost: () => { const n = pick(names); return [{ kind: "addBoost", name: n }, s => S.addEnvironmentBoost(s, n)]; },
  removeBoost: () => { const n = pick(names); return [{ kind: "removeBoost", name: n }, s => S.removeEnvironmentBoost(s, n)]; },
  clearBoosts: () => [{ kind: "clearBoosts" }, s => S.clearEnvironmentBoosts(s)],
};
const sequences = [];
for (let n = 0; n < 12; n++) {
  const input = {
    ...S.createEmptyEnvironmentState(),
    questIds: [2, 4294967296, 9007199254740991],
    questRewards: { 2: 9, 4294967296: 10, 9007199254740991: 11 },
    itemNames: ["Apple"],
    itemNotificationNames: ["Apple"],
    boosts: ["XP Boost"],
    automation: { boosts: false, drops: false, quests: false },
    questAutoRegister: { requirements: true, rewards: true },
    itemRules: { buckets: ["non-ac-member"], rejectElse: true },
  };
  let state = input;
  const steps = [];
  for (const kind of Object.keys(kinds)) {
    const [edit, apply] = kinds[kind]();
    state = apply(state);
    steps.push({ edit, expected: state });
  }
  sequences.push({ input, steps });
}
{
  const input = S.createEmptyEnvironmentState();
  const edits = [
    { kind: "addItem", name: " Apple " }, { kind: "addItem", name: "apple" },
    { kind: "addItem", name: "a-b" }, { kind: "addItem", name: "ab" },
    { kind: "setItemNotification", name: " APPLE ", enabled: true },
    { kind: "setItemNotification", name: "apple", enabled: false },
    { kind: "removeItem", name: "APPLE " },
    { kind: "addBoost", name: " XP Boost " }, { kind: "addBoost", name: "xp boost" },
    { kind: "removeBoost", name: "XP BOOST" },
    ...[-1, 0, 0.5, 2.7, 4294967296, 9007199254740991].map(questId => ({ kind: "addQuest", questId, rewardItemId: 9.9 })),
    { kind: "setQuestReward", questId: 2.9, rewardItemId: 12.7 },
    { kind: "clearQuestReward", questId: 2.9 },
    { kind: "removeQuest", questId: 2.9 },
  ];
  let state = input;
  const steps = edits.map(edit => {
    switch (edit.kind) {
      case "addItem": state = S.addEnvironmentItem(state, edit.name); break;
      case "removeItem": state = S.removeEnvironmentItem(state, edit.name); break;
      case "setItemNotification": state = S.setEnvironmentItemNotification(state, edit.name, edit.enabled); break;
      case "addBoost": state = S.addEnvironmentBoost(state, edit.name); break;
      case "removeBoost": state = S.removeEnvironmentBoost(state, edit.name); break;
      case "addQuest": state = S.addEnvironmentQuest(state, edit.questId, edit.rewardItemId); break;
      case "setQuestReward": state = S.setEnvironmentQuestReward(state, edit.questId, edit.rewardItemId); break;
      case "clearQuestReward": state = S.clearEnvironmentQuestReward(state, edit.questId); break;
      case "removeQuest": state = S.removeEnvironmentQuest(state, edit.questId); break;
    }
    return { edit, expected: state };
  });
  sequences.push({ input, steps });
}
const output = sequences.map(({ input, steps }) => `  {"input":${JSON.stringify(input)},"steps":[\n${steps.map(step => "    " + JSON.stringify(step)).join(",\n")}\n  ]}`).join(",\n");
const json = `[\n${output}\n]\n`;
const path = new URL("../packages/schemas/test-fixtures/environment/reducers.json", import.meta.url);
if (process.argv.includes("--check")) assert.equal(readFileSync(path, "utf8"), json);
else writeFileSync(path, json);
console.log(`${sequences.length} sequences; ${sequences.reduce((n, s) => n + s.steps.length, 0)} edits; ${new Set(sequences.flatMap(s => s.steps.map(s => s.edit.kind))).size} edit kinds`);
