import type { QueryOutputs } from "@lucent/game/protocol";
import * as Effect from "effect/Effect";

export const run = Effect.suspend(() => {
  const host = __host;
  const readiness = Effect.try({
    try: () =>
      JSON.parse(host.read('{"kind":"readiness"}')) as QueryOutputs["readiness"],
    catch: (cause) => cause,
  });
  const loaded = (method: string) =>
    Effect.try({
      try: () => {
        const reply = host.swf(method, "[]");
        return reply !== undefined && JSON.parse(reply) === true;
      },
      catch: (cause) => cause,
    });
  const tick = (actionsOnly: boolean) =>
    Effect.gen(function* () {
      if ((yield* readiness).missing.length !== 0) return;
      if (
        !(yield* loaded("auth.isLoggedIn")) ||
        !(yield* loaded("world.isLoaded")) ||
        !(yield* loaded("player.isLoaded"))
      ) {
        return;
      }
      yield* Effect.try({
        try: () =>
          host.command(JSON.stringify({ kind: "reapplySettings", actionsOnly })),
        catch: (cause) => cause,
      });
    });
  const repeat = (actionsOnly: boolean, interval: "1 second" | "500 millis") =>
    Effect.forever(
      tick(actionsOnly).pipe(
        Effect.catch((error) =>
          Effect.sync(() => console.warn("settings policy tick failed", error)),
        ),
        Effect.andThen(Effect.sleep(interval)),
      ),
    );
  return Effect.all([repeat(false, "1 second"), repeat(true, "500 millis")], {
    concurrency: "unbounded",
    discard: true,
  });
});
