import "./host/globals";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";

import { run as settingsPolicy } from "./game/automation/SettingsPolicy";
import { apiLayer } from "./game/flash/apiLayer";
import { DiagnosticSink } from "./game/flash/bridge/DiagnosticSink";
import { turn, wake } from "./host/loop";

const botLayer = apiLayer.pipe(
  Layer.provide(
    Layer.succeed(DiagnosticSink, {
      report: (_phase, operation, cause) => {
        throw new Error(`Failed ${operation}`, { cause });
      },
    }),
  ),
);
const runtime = ManagedRuntime.make(botLayer);
runtime.runSync(Effect.void);
runtime.runPromise(settingsPolicy);

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake }) });
