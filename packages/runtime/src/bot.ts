import "./host/globals";

import * as Effect from "effect/Effect";
import * as ManagedRuntime from "effect/ManagedRuntime";

import { run as settingsPolicy } from "./game/automation/SettingsPolicy";
import { apiLayer } from "./game/flash/apiLayer";
import { turn, wake } from "./host/loop";

const runtime = ManagedRuntime.make(apiLayer);
runtime.runSync(Effect.void);
runtime.runPromise(settingsPolicy);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
