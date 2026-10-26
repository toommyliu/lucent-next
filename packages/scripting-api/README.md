`script-api.d.ts` is the contract. User scripts type-check against it.
`gen/gen-script-types.ts` regenerates it from `packages/runtime` (`pnpm --filter @lucent/scripting-api generate`).
The check is that regeneration is byte-identical to the committed file.
Do not edit the d.ts by hand.
