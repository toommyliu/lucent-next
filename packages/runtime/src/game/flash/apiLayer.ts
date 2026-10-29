import * as Layer from "effect/Layer";

import * as Api from "./api/Api";
import * as Bridge from "./bridge/Bridge";
import * as Gateway from "./bridge/Gateway";
import * as HostState from "./state/HostState";

const gatewayLayer = Gateway.layer.pipe(Layer.provideMerge(Bridge.layer));
const hostLayer = HostState.layer.pipe(Layer.provideMerge(gatewayLayer));
export const apiLayer = Api.layer.pipe(Layer.provideMerge(hostLayer));
