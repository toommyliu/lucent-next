import * as Effect from "effect/Effect";
import * as Schema from "effect/Schema";
import * as Semaphore from "effect/Semaphore";
import * as SubscriptionRef from "effect/SubscriptionRef";

import type { BridgeService } from "../bridge/Bridge";
import type { SettingsPatch as SettingsPatchValue } from "../contract/Settings";
import { makeSettingsState, type SettingsState } from "../state/Settings";

export const MINIMAL_RENDERING_FRAME_RATE = 2;

const normalizeFrameRate = (frameRate: number): number =>
  Math.max(1, Math.min(60, frameRate));

const normalizePatch = (input: SettingsPatchValue): SettingsPatchValue => ({
  ...input,
  ...(input.frameRate === undefined
    ? {}
    : { frameRate: normalizeFrameRate(input.frameRate) }),
  ...(input.walkSpeed === undefined
    ? {}
    : { walkSpeed: Math.max(1, input.walkSpeed) }),
});

const reapplyPatch = (state: SettingsState): SettingsPatchValue => {
  const {
    customGuild,
    customGuildConfigured,
    customName,
    customNameConfigured,
    ...settings
  } = state;

  return {
    ...settings,
    ...(customGuildConfigured ? { customGuild } : {}),
    ...(customNameConfigured ? { customName } : {}),
  };
};

const recurringActionsPatch = (state: SettingsState): SettingsPatchValue => ({
  enemyMagnetEnabled: state.enemyMagnetEnabled,
  infiniteRangeEnabled: state.infiniteRangeEnabled,
  provokeCellEnabled: state.provokeCellEnabled,
  skipCutscenesEnabled: state.skipCutscenesEnabled,
});

const effectiveFrameRate = (state: SettingsState): number =>
  state.renderingMode === "minimal"
    ? MINIMAL_RENDERING_FRAME_RATE
    : state.frameRate;

export const makeSettings = Effect.fnUntraced(function* (
  bridge: BridgeService,
) {
  const stateRef = yield* SubscriptionRef.make(makeSettingsState());
  const stored = {
    get: SubscriptionRef.get(stateRef),
    patch: (patch: Partial<SettingsState>) =>
      SubscriptionRef.updateAndGet(stateRef, (state) => ({
        ...state,
        ...patch,
      })),
  };
  const updates = yield* Semaphore.make(1);
  const command = (
    method: keyof Window["swf"],
    value?: boolean | number | string,
  ): Effect.Effect<void> =>
    bridge
      .invoke(method, value === undefined ? undefined : [value], Schema.Void)
      .pipe(Effect.asVoid);

  const execute = Effect.fnUntraced(function* (
    patch: SettingsPatchValue,
    state: SettingsState,
  ) {
    const effects: Effect.Effect<void>[] = [];
    const enqueue = <K extends keyof SettingsPatchValue>(
      key: K,
      method: keyof Window["swf"],
    ) => {
      const value = patch[key];
      if (value !== undefined && value !== null)
        effects.push(command(method, value));
    };
    enqueue("animationsEnabled", "settings.setAnimationsEnabled");
    enqueue("collisionsEnabled", "settings.setCollisionsEnabled");
    if (patch.customGuild === null)
      effects.push(command("settings.resetCustomGuild"));
    enqueue("customGuild", "settings.setCustomGuild");
    if (patch.customName === null)
      effects.push(command("settings.resetCustomName"));
    enqueue("customName", "settings.setCustomName");
    enqueue("deathAdsVisible", "settings.setDeathAdsVisible");
    enqueue("otherPlayersVisible", "settings.setOtherPlayersVisible");
    enqueue("walkSpeed", "settings.setWalkSpeed");
    if (patch.renderingMode !== undefined) {
      effects.push(
        command("settings.setLagKillerEnabled", state.renderingMode !== "full"),
      );
    }
    if (patch.frameRate !== undefined || patch.renderingMode !== undefined) {
      effects.push(
        command("settings.setFrameRate", effectiveFrameRate(state)),
      );
    }
    if (patch.enemyMagnetEnabled === true) {
      effects.push(command("settings.enemyMagnet"));
    }
    if (patch.infiniteRangeEnabled === true) {
      effects.push(command("settings.infiniteRange"));
    }
    if (patch.provokeCellEnabled === true) {
      effects.push(command("settings.provokeCell"));
    }
    if (patch.skipCutscenesEnabled === true) {
      effects.push(command("settings.skipCutscenes"));
    }
    yield* Effect.all(effects, { discard: true });
  });

  const applyUnlocked = Effect.fnUntraced(function* (
    input: SettingsPatchValue,
  ) {
    const patch = normalizePatch(input);
    const current = yield* stored.get;
    const { customGuild, customName, ...values } = patch;
    const statePatch = {
      ...values,
      ...(customGuild === undefined
        ? {}
        : {
            customGuild: customGuild ?? "",
            customGuildConfigured: customGuild !== null,
          }),
      ...(customName === undefined
        ? {}
        : {
            customName: customName ?? "",
            customNameConfigured: customName !== null,
          }),
    };
    const next: SettingsState = { ...current, ...statePatch };

    yield* execute(patch, next);
    yield* stored.patch(statePatch);
  });

  const apply = (input: SettingsPatchValue) =>
    updates.withPermits(1)(applyUnlocked(input));

  const action = (
    key:
      | "enemyMagnetEnabled"
      | "infiniteRangeEnabled"
      | "provokeCellEnabled"
      | "skipCutscenesEnabled",
    method: keyof Window["swf"],
  ) =>
    updates.withPermits(1)(
      Effect.gen(function* () {
        yield* command(method);
        const state = yield* stored.get;
        yield* stored.patch({ [key]: !state[key] });
      }),
    );

  const reapply = () =>
    updates.withPermits(1)(
      Effect.gen(function* () {
        const state = yield* stored.get;
        yield* execute(reapplyPatch(state), state);
      }),
    );

  const reapplyActions = () =>
    updates.withPermits(1)(
      Effect.gen(function* () {
        const state = yield* stored.get;
        yield* execute(recurringActionsPatch(state), state);
      }),
    );

  const get = () => stored.get;
  const enemyMagnet = () =>
    action("enemyMagnetEnabled", "settings.enemyMagnet");
  const infiniteRange = () =>
    action("infiniteRangeEnabled", "settings.infiniteRange");
  const provokeCell = () =>
    action("provokeCellEnabled", "settings.provokeCell");
  const skipCutscenes = () =>
    action("skipCutscenesEnabled", "settings.skipCutscenes");
  const isAntiCounterEnabled = () =>
    stored.get.pipe(Effect.map((state) => state.antiCounterEnabled));
  return {
    apply,
    enemyMagnet,
    get,
    infiniteRange,
    isAntiCounterEnabled,
    provokeCell,
    reapply,
    reapplyActions,
    skipCutscenes,
  };
});

export type Settings = Effect.Success<ReturnType<typeof makeSettings>>;
