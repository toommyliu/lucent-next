import * as Cause from "effect/Cause";
import type * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";

export const makeRetryLoop = (name: string) => {
  const failing = new Set<symbol>();

  return <A, E, R>(
    tick: () => Effect.Effect<A, E, R>,
    interval: Duration.Input,
  ): Effect.Effect<never, never, R> =>
    Effect.suspend(() => {
      const loop = Symbol();
      return Effect.forever(
        Effect.suspend(tick).pipe(
          Effect.tap(() =>
            Effect.sync(() => {
              if (failing.delete(loop) && failing.size === 0) {
                console.info(`${name} recovered`);
              }
            }),
          ),
          Effect.catchCause((cause) =>
            Cause.hasInterrupts(cause)
              ? Effect.interrupt
              : Effect.sync(() => {
                  if (failing.size === 0) {
                    console.warn(`${name} tick failed`, Cause.pretty(cause));
                  }
                  failing.add(loop);
                }),
          ),
          Effect.andThen(Effect.sleep(interval)),
        ),
      ).pipe(Effect.ensuring(Effect.sync(() => failing.delete(loop))));
    });
};
