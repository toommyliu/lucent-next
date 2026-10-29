import { describe, expect, it } from "vitest";

import { LiveAura, type AuraKind } from "./aura";
import { EntityState } from "./entity";
import { LiveMonster } from "./monster";

const aura = (kind: AuraKind, name: string) =>
  new LiveAura({ duration: 0, kind, name, persistent: false, stack: 1 });

const monster = (auras: readonly LiveAura[]) =>
  new LiveMonster(
    {
      aggressive: false,
      cell: "r1",
      hp: 100,
      level: 1,
      maxHp: 100,
      maxMp: 0,
      monsterId: 1,
      monsterMapId: 48,
      mp: 0,
      name: "Monster",
      race: "None",
      state: EntityState.InCombat,
    },
    auras,
  );

describe("entity auras", () => {
  it("preserves exact names and cross-kind first-match order", () => {
    const passive = aura("passive", "Focus");
    const active = aura("active", "Focus");
    const entity = monster([
      passive,
      active,
      aura("active", "focus"),
      aura("active", " Focus "),
    ]);
    expect(entity.auras.map((entry) => `${entry.kind}:${entry.name}`)).toEqual([
      "passive:Focus",
      "active:Focus",
      "active:focus",
      "active: Focus ",
    ]);
    expect(entity.getAura(" FOCUS ")).toBe(passive);
    expect(entity.getAura(" FOCUS ", { kind: "active" })).toBe(active);
    expect(entity.hasAura(" FoCuS ")).toBe(true);
  });
});
