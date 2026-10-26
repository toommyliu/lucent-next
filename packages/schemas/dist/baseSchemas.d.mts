import * as Schema from "effect/Schema";
declare const TrimmedString: Schema.decodeTo<Schema.String, Schema.String, never, never>;
declare const TrimmedNonEmptyString: Schema.decodeTo<Schema.String, Schema.String, never, never>;
declare const NonNegativeInt: Schema.Int;
declare const PositiveInt: Schema.Int;
declare const boundedInt: (minimum: number, maximum: number) => Schema.Int;
export { NonNegativeInt, PositiveInt, TrimmedNonEmptyString, TrimmedString, boundedInt };