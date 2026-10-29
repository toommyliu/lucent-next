import { toLiveItem } from "@lucent/game";
import type { ItemQuery, LiveItem, ShopItemQuery } from "@lucent/game";
import type {
  Command,
  CommandOutputs,
  ItemContainer,
  Query,
  QueryOutputs,
} from "@lucent/game/protocol";
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Option from "effect/Option";

import { DiagnosticSink } from "../bridge/DiagnosticSink";

type QueryOf<K extends Query["kind"]> = Extract<Query, { kind: K }>;
type CommandOf<K extends Command["kind"]> = Extract<Command, { kind: K }>;

export interface HostStateService {
  readonly read: <K extends Query["kind"]>(
    query: QueryOf<K>,
  ) => Effect.Effect<QueryOutputs[K]>;
  readonly command: <K extends Command["kind"]>(
    command: CommandOf<K>,
  ) => Effect.Effect<Option.Option<CommandOutputs[K]>>;
}

export class HostState extends Context.Service<HostState, HostStateService>()(
  "lucent/renderer/flash/HostState",
) {}

export const layer = Layer.effect(
  HostState,
  Effect.gen(function* () {
    const diagnosticSink = yield* DiagnosticSink;

    const read = <K extends Query["kind"]>(query: QueryOf<K>) =>
      Effect.sync(
        () => JSON.parse(__host.read(JSON.stringify(query))) as QueryOutputs[K],
      );

    const command = <K extends Command["kind"]>(command: CommandOf<K>) =>
      Effect.sync((): Option.Option<CommandOutputs[K]> => {
        try {
          return Option.some(
            JSON.parse(__host.command(JSON.stringify(command))),
          );
        } catch (cause) {
          diagnosticSink.report(
            "invoke",
            command.kind,
            cause,
            command.kind === "login" ? undefined : [command],
          );
          return Option.none();
        }
      });

    return { command, read };
  }),
);

export const isSelectable = (selector: unknown): boolean =>
  typeof selector !== "number" || Number.isFinite(selector);

export const readItem = (
  host: HostStateService,
  container: ItemContainer,
  selector: ItemQuery | ShopItemQuery,
): Effect.Effect<LiveItem | null> =>
  isSelectable(selector)
    ? host
        .read({ kind: "item", container, selector })
        .pipe(Effect.map((item) => (item === null ? null : toLiveItem(item))))
    : Effect.succeed(null);

export const readItems = (
  host: HostStateService,
  container: ItemContainer,
): Effect.Effect<readonly LiveItem[]> =>
  host
    .read({ kind: "items", container })
    .pipe(Effect.map((items) => items.map(toLiveItem)));

export const itemQuantity = (
  host: HostStateService,
  container: ItemContainer,
  selector: ItemQuery | ShopItemQuery,
): Effect.Effect<number> =>
  readItem(host, container, selector).pipe(
    Effect.map((item) => item?.quantity ?? 0),
  );

export const isHydrated = (
  host: HostStateService,
  container: ItemContainer,
): Effect.Effect<boolean> =>
  host
    .read({ kind: "itemHydration", container })
    .pipe(Effect.map((version) => version > 0));
