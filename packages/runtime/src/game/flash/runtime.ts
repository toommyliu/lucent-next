import * as Layer from "effect/Layer";
import * as Logger from "effect/Logger";
import type * as LogLevel from "effect/LogLevel";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as References from "effect/References";

import type { RendererLogRecord } from "../../shared/ipc/rendererLog";
import { desktopLogRecord } from "../../shared/logRecord";

import * as Army from "../army/Army";
import * as Automation from "../automation/Automation";
import * as Environment from "../environment/Environment";
import * as SettingsPolicy from "../automation/SettingsPolicy";
import * as Scripting from "../scripting/ScriptRunner";
import * as ScriptDialogs from "../scripting/ScriptDialogs";
import { apiLayer } from "./apiLayer";
import * as DiagnosticSink from "./bridge/DiagnosticSink";

export { apiLayer } from "./apiLayer";

const rendererLogLevels: Partial<
  Record<LogLevel.LogLevel, RendererLogRecord["level"]>
> = {
  Fatal: "error",
  Error: "error",
  Warn: "warn",
  Info: "info",
  Debug: "debug",
};

export const loggingLayer = Layer.mergeAll(
  Logger.layer(
    [
      Logger.make<unknown, void>((options) => {
        const level = rendererLogLevels[options.logLevel];
        if (level === undefined) return;
        try {
          const { message, component, data, error, repeatKey } =
            desktopLogRecord(options);
          window.desktop.log.write({
            level,
            message,
            component: typeof component === "string" ? component : "renderer",
            ...(data === undefined ? {} : { data }),
            ...(error === undefined ? {} : { error }),
            ...(repeatKey === undefined ? {} : { repeatKey }),
          });
        } catch {}
      }),
    ],
    { mergeWithExisting: false },
  ),
  Layer.sync(References.MinimumLogLevel, () =>
    typeof window !== "undefined" && window.desktop.debug ? "Debug" : "Info",
  ),
);

const diagnosticLayer =
  typeof window !== "undefined" && window.desktop.debug
    ? DiagnosticSink.debugLayer
    : DiagnosticSink.noopLayer;

const consumerLayer = Layer.mergeAll(
  Automation.layer,
  Army.layer,
  Environment.layer,
  ScriptDialogs.layer,
).pipe(Layer.provideMerge(apiLayer), Layer.provideMerge(diagnosticLayer));

export const liveLayer = Layer.mergeAll(
  SettingsPolicy.layer,
  Scripting.layer,
).pipe(Layer.provideMerge(consumerLayer));

export const flashRuntime = ManagedRuntime.make(
  liveLayer.pipe(Layer.provideMerge(loggingLayer)),
);
