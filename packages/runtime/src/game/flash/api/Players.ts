import { toLivePlayer } from "@lucent/game";
import * as Effect from "effect/Effect";

import { isSelectable, type HostStateService } from "../state/HostState";

export const makePlayers = (host: HostStateService) => {
  const get = (selector: string | number) =>
    isSelectable(selector)
      ? host
          .read({ kind: "player", selector })
          .pipe(
            Effect.map((player) =>
              player === null ? null : toLivePlayer(player),
            ),
          )
      : Effect.succeed(null);
  const getAll = () =>
    host
      .read({ kind: "players" })
      .pipe(Effect.map((players) => players.map(toLivePlayer)));
  const getMe = () =>
    host
      .read({ kind: "me" })
      .pipe(
        Effect.map((player) => (player === null ? null : toLivePlayer(player))),
      );

  return {
    get,
    getAll,
    getMe,
  };
};

export type Players = ReturnType<typeof makePlayers>;
