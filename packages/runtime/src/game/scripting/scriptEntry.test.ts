import { describe, expect, it } from "@effect/vitest";
import * as Effect from "effect/Effect";

import { readScriptEntry } from "./scriptEntry";

const message = (namespace: unknown) =>
  readScriptEntry(namespace, "scripts/farm.js", {}).pipe(
    Effect.match({
      onFailure: (error) => error.message,
      onSuccess: () => "unexpected success",
    }),
  );

describe("script entry", () => {
  it.effect("runs a generator default export", () =>
    Effect.gen(function* () {
      const entry = yield* readScriptEntry(
        {
          default: function* () {
            return "finished";
          },
        },
        "farm.js",
        {},
      );
      expect(yield* Effect.gen(entry.main)).toBe("finished");
      expect(entry.inputValues).toEqual({});
    }),
  );

  it.effect(
    "rejects a plain function and a missing default with the display path",
    () =>
      Effect.gen(function* () {
        expect(yield* message({ default: () => "finished" })).toBe(
          "scripts/farm.js must export a generator function as its default export.",
        );
        expect(yield* message({})).toBe(
          "scripts/farm.js must export a generator function as its default export.",
        );
      }),
  );

  it.effect("applies input defaults and validates supplied values", () =>
    Effect.gen(function* () {
      const entry = yield* readScriptEntry(
        {
          default: function* () {
            return "finished";
          },
          inputs: {
            id: "farm",
            fields: [
              {
                key: "target",
                label: "Target",
                type: "string",
                required: true,
                default: "Artix",
              },
              { key: "count", label: "Count", type: "number", default: 3 },
              {
                key: "mode",
                label: "Mode",
                type: "select",
                options: ["fast", "safe"],
                default: "safe",
              },
            ],
          },
        },
        "farm.js",
        { count: 7, mode: "unknown", extra: "ignored" },
      );
      expect(entry.inputValues).toEqual({
        target: "Artix",
        count: 7,
        mode: "safe",
      });
    }),
  );

  it.effect("reports missing required inputs", () =>
    Effect.gen(function* () {
      expect(
        yield* message({
          default: function* () {},
          inputs: {
            id: "farm",
            fields: [
              {
                key: "target",
                label: "Target",
                type: "string",
                required: true,
              },
            ],
          },
        }),
      ).toBe("scripts/farm.js is missing required inputs: target.");
    }),
  );

  it.effect("rejects an invalid inputs export", () =>
    Effect.gen(function* () {
      expect(
        yield* message({
          default: function* () {},
          inputs: { id: "farm", fields: [{ type: "unsupported" }] },
        }),
      ).toBe("scripts/farm.js has an invalid inputs export.");
    }),
  );
});
