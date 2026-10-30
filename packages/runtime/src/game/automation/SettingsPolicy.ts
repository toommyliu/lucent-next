import * as Effect from "effect/Effect";

import { Api } from "../flash/api/Api";
import { callHost, HostBoundaryFailure } from "./HostCall";

export const run = Effect.gen(function* () {
  const api = yield* Api;
  let failing = false;
  const tick = (actionsOnly: boolean) =>
    Effect.gen(function* () {
      if (!(yield* api.player.isReady())) return;
      yield* Effect.sync(() => {
        const reply = callHost(
          "command",
          JSON.stringify({ kind: "reapplySettings", actionsOnly }),
        );
        if (JSON.parse(reply) !== null) {
          throw new Error("Invalid reapplySettings reply");
        }
      });
    }).pipe(
      Effect.tap(() =>
        Effect.sync(() => {
          if (failing) console.info("settings policy recovered");
          failing = false;
        }),
      ),
      Effect.catchDefect((error) => {
        if (!(error instanceof HostBoundaryFailure)) return Effect.die(error);
        return Effect.sync(() => {
          if (!failing) console.warn("settings policy tick failed", error);
          failing = true;
        });
      }),
    );
  const repeat = (actionsOnly: boolean, interval: "1 second" | "500 millis") =>
    Effect.forever(
      tick(actionsOnly).pipe(Effect.andThen(Effect.sleep(interval))),
    );
  yield* Effect.all([repeat(false, "1 second"), repeat(true, "500 millis")], {
    concurrency: "unbounded",
    discard: true,
  });
});
