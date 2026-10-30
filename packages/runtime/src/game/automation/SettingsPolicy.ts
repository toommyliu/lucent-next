import * as Effect from "effect/Effect";
import * as Option from "effect/Option";

import { Api } from "../flash/api/Api";
import { DiagnosticSink } from "../flash/bridge/DiagnosticSink";
import { HostState } from "../flash/state/HostState";
import { makeRetryLoop } from "./retryLoop";

export const run = Effect.gen(function* () {
  const api = yield* Api;
  const host = yield* HostState;
  const diagnostics = yield* DiagnosticSink;
  const repeat = makeRetryLoop("settings policy");
  const tick = (actionsOnly: boolean) =>
    Effect.gen(function* () {
      if (!(yield* api.player.isReady())) return;
      const result = yield* host.command({ kind: "reapplySettings", actionsOnly });
      if (Option.isNone(result)) {
        return yield* Effect.die(new Error("Failed reapplySettings", {
          cause: diagnostics.takeCause?.("reapplySettings"),
        }));
      }
      if (result.value !== null) {
        return yield* Effect.die(new Error("Invalid reapplySettings reply"));
      }
    });
  yield* Effect.all(
    [
      repeat(() => tick(false), "1 second"),
      repeat(() => tick(true), "500 millis"),
    ],
    { concurrency: "unbounded", discard: true },
  );
});
