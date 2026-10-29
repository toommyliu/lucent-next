import "./host/globals";

import * as Effect from "effect/Effect";

import { run as settingsPolicy } from "./game/automation/SettingsPolicy";
import { turn, wake } from "./host/loop";

Effect.runPromise(settingsPolicy);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
