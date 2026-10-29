import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import * as Schema from "effect/Schema";

import type { BridgeService } from "../bridge/Bridge";
import type { HostStateService } from "../state/HostState";
import type { Wait } from "./Wait";

export interface CellPositionOptions {
  /** Destination cell. */
  readonly cell?: string;
  /** Destination pad. */
  readonly pad?: string;
}

export const makeMap = (
  bridge: BridgeService,
  host: HostStateService,
  wait: Wait,
) => {
  const isLoaded = () =>
    bridge
      .invoke("world.isLoaded", undefined, Schema.Boolean)
      .pipe(Effect.map(Option.getOrElse(() => false)));

  const getCellPads = () =>
    host.command({ kind: "refreshCellPads" }).pipe(
      Effect.flatMap(
        Option.match({
          onNone: () => host.read({ kind: "cellPads" }),
          onSome: Effect.succeed,
        }),
      ),
    );

  const getCells = () =>
    host.command({ kind: "refreshCells" }).pipe(
      Effect.flatMap(
        Option.match({
          onNone: () => host.read({ kind: "cells" }),
          onSome: Effect.succeed,
        }),
      ),
    );

  const getMap = () => host.read({ kind: "map" });

  const getId = () => getMap().pipe(Effect.map((map) => map.id));

  const getMapItem = (itemId: number) => {
    if (!Number.isSafeInteger(itemId) || itemId <= 0) return Effect.void;
    return wait.forGameAction("getMapItem").pipe(
      Effect.flatMap((ready) =>
        ready
          ? bridge.invoke("world.getMapItem", [itemId], Schema.Void)
          : Effect.succeed(Option.none()),
      ),
      Effect.asVoid,
    );
  };

  const getName = () => getMap().pipe(Effect.map((map) => map.name));

  const getRoomNumber = () =>
    getMap().pipe(Effect.map((map) => map.roomNumber));

  const loadSwf = (swf: string) =>
    bridge.invoke("world.loadSwf", [swf], Schema.Void).pipe(Effect.asVoid);

  const reload = () =>
    bridge.invoke("world.reload", undefined, Schema.Void).pipe(Effect.asVoid);

  const setSpawnPoint = (options?: CellPositionOptions) =>
    bridge
      .invoke("world.setSpawnPoint", [options?.cell, options?.pad], Schema.Void)
      .pipe(Effect.asVoid);

  return {
    getCellPads,
    getCells,
    getId,
    getMapItem,
    getName,
    getRoomNumber,
    isLoaded,
    loadSwf,
    reload,
    setSpawnPoint,
  };
};

export type Map = ReturnType<typeof makeMap>;
