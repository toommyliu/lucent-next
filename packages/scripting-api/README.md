`src/contract.ts` owns the public scripting API signatures. `src/types.ts` owns
their supporting options, callbacks, and documented event views. Shared game
data comes from `@lucent/game` and its generated protocol types. `src/schema.ts`
owns the public schema module declarations.

The runtime imports `@lucent/scripting-api/contract` for its public API annotations
and `@lucent/scripting-api/schema` for the schema module's types. These imports
are type-only. An incompatible implementation fails `pnpm typecheck`, except
the `lucent/schema` module and the `events.on`, `events.once` and `packet.on`
overloads, which the runtime casts to their contract types.

User scripts type-check against `script-api.d.ts`. Regenerate it with
`pnpm --filter @lucent/scripting-api generate`; do not edit it by hand.
`pnpm check:scripting-api` requires byte-identical declarations. The generator
reads the contract and shared domain types without loading runtime sources.
