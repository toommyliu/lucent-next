import * as Cause from "effect/Cause";
import * as Effect from "effect/Effect";
import type * as Duration from "effect/Duration";

class HostBoundaryFailure extends Error {
  constructor(cause: unknown) {
    super("Auto relogin host boundary failed", { cause });
  }
}

export const callReloginHost = <A>(call: () => A): A => {
  try {
    return call();
  } catch (cause) {
    throw new HostBoundaryFailure(cause);
  }
};

export type ReloginSupervisor = <A, E, R>(
  effect: Effect.Effect<A, E, R>,
  retryAfter?: Duration.Input,
) => Effect.Effect<A, E, R>;

export const makeReloginSupervisor = (onDefect: (error: unknown) => void): ReloginSupervisor =>
  (effect, retryAfter = 500) => Effect.suspend(() => {
    let failed = false;
    const attempt: typeof effect = effect.pipe(
      Effect.tap(() => Effect.sync(() => {
        if (failed) console.warn("auto relogin host boundary recovered");
        failed = false;
      })),
      Effect.catchCause((cause) => {
        if (Cause.hasInterruptsOnly(cause)) return Effect.failCause(cause);
        if (!cause.reasons.every((reason) => Cause.isDieReason(reason) && reason.defect instanceof HostBoundaryFailure)) {
          return Effect.sync(() => onDefect(Cause.squash(cause))).pipe(Effect.andThen(Effect.failCause(cause)));
        }
        return Effect.sync(() => {
          if (!failed) console.warn("auto relogin host boundary failed; retrying");
          failed = true;
        }).pipe(Effect.andThen(Effect.sleep(retryAfter)), Effect.andThen(attempt));
      }),
    );
    return attempt;
  });
