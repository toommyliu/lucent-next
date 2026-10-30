import * as Cause from "effect/Cause";
import * as Effect from "effect/Effect";
import type * as Duration from "effect/Duration";

export type ReloginSupervisor = <A, E, R>(
  effect: Effect.Effect<A, E, R>,
  retryAfter?: Duration.Input,
) => Effect.Effect<A, E, R>;

export const retryReloginTick: ReloginSupervisor = (effect, retryAfter = 500) =>
  Effect.suspend(() => {
    let failed = false;
    const tick: typeof effect = effect.pipe(
      Effect.tap(() => Effect.sync(() => {
        if (failed) console.warn("auto relogin tick recovered");
        failed = false;
      })),
      Effect.catchCause((cause) => {
        if (Cause.hasInterrupts(cause)) return Effect.failCause(cause);
        return Effect.sync(() => {
          if (!failed) console.warn("auto relogin tick failed; retrying", Cause.pretty(cause));
          failed = true;
        }).pipe(Effect.andThen(Effect.sleep(retryAfter)), Effect.andThen(tick));
      }),
    );
    return tick;
  });
