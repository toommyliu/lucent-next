import * as Context from "effect/Context";
import type * as Effect from "effect/Effect";
import type { DeepReadonly } from "@lucent/game";
import type { ReadinessSnapshot } from "@lucent/game/protocol";

import type { HostStateService } from "../state/HostState";

export type ProjectionReadinessSnapshot = DeepReadonly<ReadinessSnapshot>;

export const makeProjectionReadiness = (host: HostStateService) => ({
  inspect: (): Effect.Effect<ProjectionReadinessSnapshot> =>
    host.read({ kind: "readiness" }),
});

export class ProjectionReadiness extends Context.Service<
  ProjectionReadiness,
  ReturnType<typeof makeProjectionReadiness>
>()("lucent/renderer/flash/ProjectionReadiness") {}
