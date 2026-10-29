import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import type { DeepReadonly } from "@lucent/game";
import type { ReadinessFlags, ReadinessSnapshot } from "@lucent/game/protocol";

import type { HostStateService } from "../state/HostState";

export type ProjectionReadinessState = Readonly<ReadinessFlags>;

export type ProjectionReadinessSnapshot = DeepReadonly<ReadinessSnapshot>;

export const makeProjectionReadiness = (host: HostStateService) => {
  const inspect = Effect.fn("ProjectionReadiness.inspect")(function* () {
    const snapshot: ProjectionReadinessSnapshot = yield* host.read({
      kind: "readiness",
    });
    return snapshot;
  });

  const get = Effect.fn("ProjectionReadiness.get")(function* () {
    return (yield* inspect()).state;
  });

  const isReady = Effect.fn("ProjectionReadiness.isReady")(function* () {
    return (yield* inspect()).missing.length === 0;
  });

  return { get, inspect, isReady };
};

export class ProjectionReadiness extends Context.Service<
  ProjectionReadiness,
  ReturnType<typeof makeProjectionReadiness>
>()("lucent/renderer/flash/ProjectionReadiness") {}
