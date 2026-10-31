import type { CombatProfile } from "@lucent/schemas/combatProfiles";
import { EntityState } from "@lucent/game";
import * as Effect from "effect/Effect";
import * as Exit from "effect/Exit";

import { acquireConsumableClaim } from "../../../../host/claims";
import { isDirectInventoryConsumable, type Inventory } from "../Inventory";
import type { Player } from "../Player";

interface ConsumableSkillItem {
  readonly itemId: number;
}

interface Dependencies {
  readonly getConsumableSkillItem: () => Effect.Effect<ConsumableSkillItem | null>;
  readonly inventory: Pick<Inventory, "equip" | "get">;
  readonly player: Pick<Player, "getState">;
}

export interface CombatProfileConsumablePreparation {
  readonly release: Effect.Effect<void>;
  readonly skill5ItemId?: number;
  readonly warning?: string;
}

const profileUsesSkill5 = (profile: CombatProfile): boolean =>
  profile.consumable !== undefined ||
  profile.steps.some((step) => step.skill === 5) ||
  (profile.messageTriggers ?? []).some((trigger) => trigger.skill === 5);

const unavailable = (warning: string): CombatProfileConsumablePreparation => ({
  release: Effect.void,
  warning,
});

export const makeCombatProfileConsumables = (deps: Dependencies) => {
  const host = globalThis.__host;
  const prepare = Effect.fn("CombatProfileConsumables.prepare")(
    function* (
      profile: CombatProfile,
    ): Effect.fn.Return<CombatProfileConsumablePreparation> {
      if (!profileUsesSkill5(profile)) {
        return { release: Effect.void };
      }

      if (profile.consumable === undefined) {
        const current = yield* deps.getConsumableSkillItem();
        if (current === null) {
          const claim = yield* acquireConsumableClaim(undefined, host);
          return { release: claim.release };
        }

        const claim = yield* acquireConsumableClaim(current.itemId, host);
        return {
          release: claim.release,
          skill5ItemId: current.itemId,
        };
      }

      const item = yield* deps.inventory.get(profile.consumable);
      if (item === null) {
        return unavailable(
          `${profile.consumable} was not found. Skill 5 will use whichever consumable is equipped.`,
        );
      }
      if (
        item.category !== "Item" ||
        item.link.trim() === "" ||
        item.link.trim().toLowerCase() === "none" ||
        isDirectInventoryConsumable(item.link)
      ) {
        return unavailable(
          `${item.name} cannot be equipped. Skill 5 will use whichever consumable is equipped.`,
        );
      }

      return yield* Effect.acquireUseRelease(
        acquireConsumableClaim(item.itemId, host),
        (claim) => Effect.gen(function* () {
          if (!claim.acquired) {
            return unavailable(
              `Could not equip ${item.name} because another combat profile controls preflight equipment. Skill 5 will use whichever consumable is equipped.`,
            );
          }
          if (!claim.first) {
            return {
              release: claim.release,
              skill5ItemId: item.itemId,
            };
          }

          const current = yield* deps.getConsumableSkillItem();
          if (current?.itemId !== item.itemId) {
            if ((yield* deps.player.getState()) !== EntityState.Idle) {
              yield* claim.release;
              return unavailable(
                `Could not equip ${item.name} while you were in combat. Skill 5 will use whichever consumable is equipped.`,
              );
            }
            if (!(yield* deps.inventory.equip(item.itemId))) {
              yield* claim.release;
              return unavailable(
                `Could not equip ${item.name}. Check its requirements. Skill 5 will use whichever consumable is equipped.`,
              );
            }
          }

          return {
            release: claim.release,
            skill5ItemId: item.itemId,
          };
        }),
        (claim, exit) => (Exit.isFailure(exit) ? claim.release : Effect.void),
      );
    },
  );

  return { prepare };
};
