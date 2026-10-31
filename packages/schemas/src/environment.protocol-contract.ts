import type * as Protocol from "../../game/src/protocol";
import type { AssertEqual } from "../../game/src/protocol-contract";
import type { DeepReadonly } from "../../game/src/model";
import type { EnvironmentState } from "./environment";
type Assert<T extends true> = T;
export type EnvironmentProtocolContract = Assert<AssertEqual<EnvironmentState, DeepReadonly<Protocol.EnvironmentState>>>;
