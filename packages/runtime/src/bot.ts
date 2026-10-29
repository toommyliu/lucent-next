import "./host/globals";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";

import { layer as settingsPolicy } from "./game/automation/SettingsPolicy";
import { apiLayer } from "./game/flash/apiLayer";
import { turn, wake } from "./host/loop";

const bot = ManagedRuntime.make(settingsPolicy.pipe(Layer.provide(apiLayer)));
bot.runSync(Effect.void);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
