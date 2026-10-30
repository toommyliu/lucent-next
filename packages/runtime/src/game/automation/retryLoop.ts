import * as Cause from "effect/Cause";
import type * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";

export const makeRetryAttempt = (name: string) => {
  const failing = new Set<symbol>();

  return <A, E, R>(
    tick: () => Effect.Effect<A, E, R>,
    interval: Duration.Input,
  ): Effect.Effect<A, never, R> =>
    Effect.suspend(() => {
      const token = Symbol();
      const attempt = (): Effect.Effect<A, never, R> =>
        Effect.suspend(tick).pipe(
          Effect.tap(() =>
            Effect.sync(() => {
              if (failing.delete(token) && failing.size === 0) {
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
                  failing.add(token);
                }).pipe(
                  Effect.andThen(Effect.sleep(interval)),
                  Effect.andThen(Effect.suspend(attempt)),
                ),
          ),
        );
      return attempt().pipe(
        Effect.ensuring(Effect.sync(() => failing.delete(token))),
      );
    });
};
