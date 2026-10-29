import { afterEach, describe, expect, it, vi } from "@effect/vitest";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";

import { DiagnosticSink } from "../bridge/DiagnosticSink";
import { item } from "../testing/FakeHost";
import {
  HostState,
  isHydrated,
  itemQuantity,
  layer,
  readItem,
} from "./HostState";

const reports: unknown[][] = [];
const live = layer.pipe(
  Layer.provide(
    Layer.succeed(DiagnosticSink, {
      report: (...args) => {
        reports.push(args);
      },
    }),
  ),
);

const stubHost = (host: {
  readonly read?: (query: string) => string;
  readonly command?: (command: string) => string;
}) => {
  vi.stubGlobal("__host", host);
};

afterEach(() => {
  vi.unstubAllGlobals();
  reports.length = 0;
});

describe("HostState", () => {
  it.effect("sends the query as JSON and parses the answer", () =>
    Effect.gen(function* () {
      const queries: string[] = [];
      stubHost({
        read: (query) => {
          queries.push(query);
          return '{"id":5,"name":"battleon","roomNumber":1}';
        },
      });
      const host = yield* HostState;
      expect(yield* host.read({ kind: "map" })).toEqual({
        id: 5,
        name: "battleon",
        roomNumber: 1,
      });
      expect(queries).toEqual(['{"kind":"map"}']);
    }).pipe(Effect.provide(live)),
  );

  it.effect("turns a failed command into none and reports why", () =>
    Effect.gen(function* () {
      stubHost({
        command: () => {
          throw new Error("auth.getServers has no loader callback");
        },
      });
      const host = yield* HostState;
      expect(yield* host.command({ kind: "refreshServers" })).toEqual(
        Option.none(),
      );
      expect(reports).toEqual([
        [
          "invoke",
          "refreshServers",
          new Error("auth.getServers has no loader callback"),
          [{ kind: "refreshServers" }],
        ],
      ]);
    }).pipe(Effect.provide(live)),
  );

  it.effect("keeps login credentials out of a failed command report", () =>
    Effect.gen(function* () {
      stubHost({
        command: () => {
          throw new Error("auth.login has no loader callback");
        },
      });
      const host = yield* HostState;
      expect(
        yield* host.command({
          kind: "login",
          password: "secret",
          username: "hero",
        }),
      ).toEqual(Option.none());
      expect(reports).toEqual([
        [
          "invoke",
          "login",
          new Error("auth.login has no loader callback"),
          undefined,
        ],
      ]);
    }).pipe(Effect.provide(live)),
  );

  it.effect("returns a successful command's answer", () =>
    Effect.gen(function* () {
      stubHost({ command: () => "3" });
      const host = yield* HostState;
      expect(
        yield* host.command({ kind: "refreshContainer", container: "bank" }),
      ).toEqual(Option.some(3));
    }).pipe(Effect.provide(live)),
  );

  it.effect("reads the quantity of the matched item, else zero", () =>
    Effect.gen(function* () {
      const answers: Record<string, string> = {
        '{"kind":"item","container":"bank","selector":"Ore"}': JSON.stringify(
          item("bank", { itemId: 7, name: "Ore", quantity: 3 }),
        ),
        '{"kind":"item","container":"bank","selector":"Gem"}': "null",
      };
      stubHost({ read: (query) => answers[query] ?? "unexpected" });
      const host = yield* HostState;
      expect(yield* itemQuantity(host, "bank", "Ore")).toBe(3);
      expect(yield* itemQuantity(host, "bank", "Gem")).toBe(0);
    }).pipe(Effect.provide(live)),
  );

  it.effect("treats a container as hydrated once it has been replaced", () =>
    Effect.gen(function* () {
      let version = 0;
      stubHost({ read: () => String(version) });
      const host = yield* HostState;
      expect(yield* isHydrated(host, "inventory")).toBe(false);
      version = 2;
      expect(yield* isHydrated(host, "inventory")).toBe(true);
    }).pipe(Effect.provide(live)),
  );

  it.effect("misses a selector the host cannot parse without asking it", () =>
    Effect.gen(function* () {
      const queries: string[] = [];
      stubHost({
        read: (query) => {
          queries.push(query);
          return JSON.stringify(
            item("inventory", { itemId: 5, name: "Sword" }),
          );
        },
      });
      const host = yield* HostState;
      expect(yield* readItem(host, "inventory", Number.NaN)).toBeNull();
      expect(
        yield* readItem(host, "inventory", { itemId: Number.NaN }),
      ).toBeNull();
      expect(
        yield* readItem(host, "inventory", {
          shopItemId: Number.POSITIVE_INFINITY,
        }),
      ).toBeNull();
      expect((yield* readItem(host, "inventory", { itemId: 5 }))?.name).toBe(
        "Sword",
      );
      expect(queries).toEqual([
        '{"kind":"item","container":"inventory","selector":{"itemId":5}}',
      ]);
    }).pipe(Effect.provide(live)),
  );
});
