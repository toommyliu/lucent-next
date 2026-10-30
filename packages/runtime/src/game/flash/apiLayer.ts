import * as Layer from "effect/Layer";

import * as Api from "./api/Api";
import * as ScriptReactions from "./api/ScriptReactions";
import * as Bridge from "./bridge/Bridge";
import * as Gateway from "./bridge/Gateway";
import * as HostState from "./state/HostState";

const gatewayLayer = Gateway.layer.pipe(Layer.provideMerge(Bridge.layer));
const hostLayer = HostState.layer.pipe(Layer.provideMerge(gatewayLayer));
const servicesLayer = Api.layer.pipe(Layer.provideMerge(hostLayer));

export const scriptApiLayer = ScriptReactions.layer.pipe(
  Layer.provideMerge(servicesLayer),
);
