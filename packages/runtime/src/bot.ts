import "./host/globals";

import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";

import { run as settingsPolicy } from "./game/automation/SettingsPolicy";
import { Api } from "./game/flash/api/Api";
import { apiLayer } from "./game/flash/apiLayer";
import { DiagnosticSink } from "./game/flash/bridge/DiagnosticSink";
import type { Event } from "./game/flash/contract/Event";
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

const onEvent = (listener: (event: Event) => void) =>
  runtime.runPromise(
    Effect.flatMap(Api, (api) =>
      api.events.on(undefined, (event) => Effect.sync(() => listener(event))),
    ),
  );

Object.assign(globalThis, { __lucent: Object.freeze({ turn, wake, onEvent }) });
