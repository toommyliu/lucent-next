import * as Effect from "effect/Effect";
import * as ManagedRuntime from "effect/ManagedRuntime";

import { Api, type ApiService } from "../game/flash/api/Api";
import { apiLayer } from "../game/flash/apiLayer";

const runtime = ManagedRuntime.make(apiLayer);
runtime.runSync(Effect.void);

export const run = <A, E>(
  program: (api: ApiService) => Effect.Effect<A, E>,
): Promise<A> =>
  runtime.runPromise(
    Effect.gen(function* () {
      return yield* program(yield* Api);
    }),
  );
