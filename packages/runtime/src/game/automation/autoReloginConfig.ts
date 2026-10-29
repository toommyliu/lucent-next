import * as EffectNumber from "effect/Number";

export const defaultAutoReloginDelay = 3_000;

export const normalizeAutoReloginDelay = (delayMs: number): number =>
  Number.isFinite(delayMs)
    ? EffectNumber.clamp({ minimum: 0, maximum: 300_000 })(delayMs)
    : defaultAutoReloginDelay;
