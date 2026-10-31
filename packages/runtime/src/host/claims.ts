import * as Effect from "effect/Effect";

export const acquireConsumableClaim = (
  itemId: number | undefined,
  host: Pick<typeof __host, "acquireConsumableClaim" | "releaseClaim">,
) =>
  Effect.sync(() => {
    const grant = JSON.parse(
      host.acquireConsumableClaim(JSON.stringify(itemId ?? null)),
    ) as {
      readonly id: string;
      readonly first: boolean;
    } | null;
    return grant === null
      ? { acquired: false, first: false, release: Effect.void }
      : {
          acquired: true,
          first: grant.first,
          release: Effect.sync(() => host.releaseClaim(grant.id)),
        };
  });
