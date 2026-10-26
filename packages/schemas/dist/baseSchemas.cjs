Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
let effect_Effect = require("effect/Effect");
effect_Effect = require_rolldown_runtime.__toESM(effect_Effect);
let effect_SchemaTransformation = require("effect/SchemaTransformation");
effect_SchemaTransformation = require_rolldown_runtime.__toESM(effect_SchemaTransformation);
// src/baseSchemas.ts
const TrimmedString = effect_Schema.String.pipe(effect_Schema.decodeTo(effect_Schema.String, effect_SchemaTransformation.transformEffect({
	decode: (value) => effect_Effect.succeed(value.trim()),
	encode: (value) => effect_Effect.succeed(value.trim())
})));
const TrimmedNonEmptyString = TrimmedString.check(effect_Schema.isNonEmpty());
const NonNegativeInt = effect_Schema.Int.check(effect_Schema.isGreaterThanOrEqualTo(0));
const PositiveInt = effect_Schema.Int.check(effect_Schema.isGreaterThanOrEqualTo(1));
const boundedInt = (minimum, maximum) => effect_Schema.Int.check(effect_Schema.isBetween({
	minimum,
	maximum
}));
exports.NonNegativeInt = NonNegativeInt;
exports.PositiveInt = PositiveInt;
exports.TrimmedNonEmptyString = TrimmedNonEmptyString;
exports.TrimmedString = TrimmedString;
exports.boundedInt = boundedInt;
