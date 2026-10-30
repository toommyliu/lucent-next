import { describe, expect, it } from "@effect/vitest";
import * as Deferred from "effect/Deferred";
import * as Effect from "effect/Effect";
import * as FiberMap from "effect/FiberMap";
import * as Ref from "effect/Ref";
import * as TestClock from "effect/testing/TestClock";

import type { Position } from "@lucent/game";
import type { ApiService } from "../flash/api/Api";
import type { Event, EventSelector } from "../flash/contract/Event";
import { makeAutoZone } from "./AutoZone";

describe("AutoZone", () => {
  it.effect("interrupts an obsolete delayed transition", () =>
    Effect.gen(function* () {
      const positions = yield* Ref.make<readonly { x: number; y: number }[]>(
        [],
      );
      const fibers = yield* FiberMap.make<string>();
      let handleEvent: ((event: Event) => Effect.Effect<void>) | undefined;
      const api = {
        events: {
          on: (
            _selector: EventSelector | undefined,
            handler: (event: Event) => Effect.Effect<void>,
          ) => {
            handleEvent = handler;
            return Effect.succeed(() => undefined);
          },
        },
        map: { getName: () => Effect.succeed("queeniona") },
        player: {
          auras: { get: () => Effect.succeed({}) },
          walkTo: (position: Position) =>
            Ref.update(positions, (current) => [...current, position]).pipe(
              Effect.as(true),
            ),
        },
      } as unknown as ApiService;
      const autoZone = yield* makeAutoZone(api, fibers);

      const mapState = yield* autoZone.setMap("queeniona");
      const enabledState = yield* autoZone.setEnabled(true);
      const readState = yield* autoZone.getState();
      expect(mapState).not.toBe(enabledState);
      expect(enabledState).not.toBe(readState);
      yield* handleEvent!({ type: "zone", map: "queeniona", zone: "A" });
      yield* handleEvent!({ type: "zone", map: "queeniona", zone: "B" });
      yield* TestClock.adjust("500 millis");
      yield* Effect.yieldNow;

      const moved = yield* Ref.get(positions);
      expect(moved).toHaveLength(1);
      expect(moved[0]!.x).toBeGreaterThanOrEqual(111);
      expect(moved[0]!.x).toBeLessThanOrEqual(272);
      yield* handleEvent!({ type: "zone", map: "queeniona", zone: "A" });
      yield* autoZone.applyConfig({ enabled: true, map: "ledgermayne" });
      yield* TestClock.adjust("500 millis");
      expect(yield* Ref.get(positions)).toEqual(moved);
      expect(yield* autoZone.getState()).toEqual({ enabled: true, map: "ledgermayne" });
    }),
  );
  for (const change of ["map", "disable"] as const) {
    it.effect(`cancels an active walk on ${change}`, () =>
      Effect.gen(function* () {
        const started = yield* Deferred.make<void>();
        const walking = yield* Ref.make(false);
        const interrupted = yield* Ref.make(0);
        const fibers = yield* FiberMap.make<string>();
        let handleEvent: ((event: Event) => Effect.Effect<void>) | undefined;
        const api = {
          events: {
            on: (_selector: EventSelector | undefined, handler: (event: Event) => Effect.Effect<void>) => {
              handleEvent = handler;
              return Effect.succeed(() => undefined);
            },
          },
          map: { getName: () => Effect.succeed("ledgermayne") },
          player: {
            walkTo: (_position: Position) => Ref.set(walking, true).pipe(
              Effect.andThen(Deferred.succeed(started, undefined)),
              Effect.andThen(Effect.never),
              Effect.onInterrupt(() => Ref.update(interrupted, (count) => count + 1).pipe(
                Effect.andThen(Ref.set(walking, false)),
              )),
            ),
          },
        } as unknown as ApiService;
        const autoZone = yield* makeAutoZone(api, fibers, { enabled: true, map: "ledgermayne" });
        yield* handleEvent!({ type: "zone", map: "ledgermayne", zone: "A" });
        yield* Deferred.await(started);
        expect(yield* Ref.get(walking)).toBe(true);
        yield* autoZone.applyConfig(change === "map"
          ? { enabled: true, map: "moreskulls" }
          : { enabled: false, map: "ledgermayne" });
        expect(yield* Ref.get(interrupted)).toBe(1);
        expect(yield* Ref.get(walking)).toBe(false);
      }),
    );
  }

});
