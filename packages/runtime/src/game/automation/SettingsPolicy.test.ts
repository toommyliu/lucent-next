import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Option from "effect/Option";
import * as TestClock from "effect/testing/TestClock";
import { vi } from "vitest";

import { Api } from "../flash/api/Api";
import { HostState, type HostStateService } from "../flash/state/HostState";
import {
  makeFakeApi,
  makeFakeHost,
  makeFakeHostState,
} from "../flash/testing/FakeHost";
import { run } from "./SettingsPolicy";

describe("SettingsPolicy", () => {
  it.effect("retries a command that returns none and logs its recovery", () =>
    Effect.gen(function* () {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const info = vi.spyOn(console, "info").mockImplementation(() => {});
      vi.stubGlobal("window", {
        swf: {
          "auth.isLoggedIn": () => true,
          "world.isLoaded": () => true,
          "player.isLoaded": () => true,
        },
      });
      yield* Effect.addFinalizer(() =>
        Effect.sync(() => {
          vi.unstubAllGlobals();
          vi.restoreAllMocks();
        }),
      );
      const state = makeFakeHostState();
      const api = yield* makeFakeApi(window, state);
      const host = makeFakeHost(state, window);
      let fails = true;
      let attempts = 0;
      const command: HostStateService["command"] = (input) => {
        if (input.kind === "reapplySettings") {
          attempts++;
          if (fails) return Effect.succeed(Option.none());
        }
        return host.command(input);
      };
      const fiber = yield* run.pipe(
        Effect.provideService(Api, api),
        Effect.provideService(HostState, { ...host, command }),
        Effect.forkScoped,
      );
      yield* TestClock.adjust(1000);
      expect(attempts).toBe(5);
      expect(warn).toHaveBeenCalledTimes(1);
      expect(warn.mock.calls[0]?.[0]).toBe("settings policy tick failed");
      expect(String(warn.mock.calls[0]?.[1]).split("\n")[0]).toBe(
        "Error: Failed reapplySettings",
      );
      expect(info).not.toHaveBeenCalled();
      fails = false;
      yield* TestClock.adjust(1000);
      expect(attempts).toBe(8);
      expect(warn).toHaveBeenCalledTimes(1);
      expect(info.mock.calls).toEqual([["settings policy recovered"]]);
      yield* Fiber.interrupt(fiber);
    }),
  );
});
