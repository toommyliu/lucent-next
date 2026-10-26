import * as Schema from "effect/Schema";
import * as Effect from "effect/Effect";
import * as SchemaTransformation from "effect/SchemaTransformation";
// src/baseSchemas.ts
const TrimmedString = Schema.String.pipe(Schema.decodeTo(Schema.String, SchemaTransformation.transformEffect({
	decode: (value) => Effect.succeed(value.trim()),
	encode: (value) => Effect.succeed(value.trim())
})));
const TrimmedNonEmptyString = TrimmedString.check(Schema.isNonEmpty());
const NonNegativeInt = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0));
const PositiveInt = Schema.Int.check(Schema.isGreaterThanOrEqualTo(1));
const boundedInt = (minimum, maximum) => Schema.Int.check(Schema.isBetween({
	minimum,
	maximum
}));
export { NonNegativeInt, PositiveInt, TrimmedNonEmptyString, TrimmedString, boundedInt };
