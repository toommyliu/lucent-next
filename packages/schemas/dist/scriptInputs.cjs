Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_scriptPackages = require("./scriptPackages.cjs");
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/scriptInputs.ts
const ScriptInputValueSchema = effect_Schema.Union([
	effect_Schema.String,
	effect_Schema.Number,
	effect_Schema.Boolean,
	effect_Schema.Array(effect_Schema.String)
]);
const ScriptInputTypeSchema = effect_Schema.Literals([
	"string",
	"number",
	"boolean",
	"select",
	"multi-select"
]);
const ScriptInputFieldBaseSchema = {
	key: effect_Schema.String,
	label: effect_Schema.String,
	description: effect_Schema.optionalKey(effect_Schema.String),
	required: effect_Schema.optionalKey(effect_Schema.Boolean)
};
const ScriptStringInputFieldSchema = effect_Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: effect_Schema.Literal("string"),
	default: effect_Schema.optionalKey(effect_Schema.String)
});
const ScriptNumberInputFieldSchema = effect_Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: effect_Schema.Literal("number"),
	default: effect_Schema.optionalKey(effect_Schema.Number)
});
const ScriptBooleanInputFieldSchema = effect_Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: effect_Schema.Literal("boolean"),
	default: effect_Schema.optionalKey(effect_Schema.Boolean)
});
const ScriptSelectInputFieldSchema = effect_Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: effect_Schema.Literal("select"),
	options: effect_Schema.Array(effect_Schema.String),
	default: effect_Schema.optionalKey(effect_Schema.String)
}).check(effect_Schema.makeFilter(({ default: defaultValue, options }) => options.length > 0 && (defaultValue === void 0 || options.includes(defaultValue)), { expected: "select input options to be non-empty and contain the default value" }));
const ScriptMultiSelectInputFieldSchema = effect_Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: effect_Schema.Literal("multi-select"),
	options: effect_Schema.Array(effect_Schema.String),
	default: effect_Schema.optionalKey(effect_Schema.Array(effect_Schema.String))
}).check(effect_Schema.makeFilter(({ default: defaultValue, options }) => {
	const optionSet = new Set(options);
	return options.length > 0 && optionSet.size === options.length && (defaultValue === void 0 || new Set(defaultValue).size === defaultValue.length && defaultValue.every((value) => optionSet.has(value)));
}, { expected: "multi-select input options to be non-empty and unique, and contain every unique default value" }));
const ScriptInputFieldSchema = effect_Schema.Union([
	ScriptStringInputFieldSchema,
	ScriptNumberInputFieldSchema,
	ScriptBooleanInputFieldSchema,
	ScriptSelectInputFieldSchema,
	ScriptMultiSelectInputFieldSchema
]);
const ScriptInputsDefinitionSchema = effect_Schema.Struct({
	id: effect_Schema.String,
	fields: effect_Schema.Array(ScriptInputFieldSchema)
}).check(effect_Schema.makeFilter(({ fields }) => new Set(fields.map((field) => field.key)).size === fields.length, { expected: "script input field keys to be unique" }));
const ScriptFileReferenceSchema = effect_Schema.Struct({
	path: effect_Schema.String,
	name: effect_Schema.String,
	reference: effect_Schema.optionalKey(require_scriptPackages.ScriptReferenceSchema)
});
const ScriptFileSchema = effect_Schema.Struct({
	path: effect_Schema.String,
	name: effect_Schema.String,
	source: effect_Schema.String,
	inputs: effect_Schema.NullOr(ScriptInputsDefinitionSchema),
	revision: effect_Schema.String,
	reference: effect_Schema.optionalKey(require_scriptPackages.ScriptReferenceSchema),
	snapshot: effect_Schema.optionalKey(require_scriptPackages.ScriptExecutionSnapshotSchema)
});
const ScriptOpenFileResultSchema = effect_Schema.Union([effect_Schema.Struct({ canceled: effect_Schema.Literal(true) }), effect_Schema.Struct({
	canceled: effect_Schema.Literal(false),
	file: ScriptFileSchema
})]);
const ScriptSelectFileResultSchema = effect_Schema.Union([effect_Schema.Struct({ canceled: effect_Schema.Literal(true) }), effect_Schema.Struct({
	canceled: effect_Schema.Literal(false),
	file: ScriptFileReferenceSchema
})]);
const ScriptFileResolutionSchema = effect_Schema.Union([
	effect_Schema.Struct({
		status: effect_Schema.Literal("found"),
		file: ScriptFileSchema
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("missing"),
		path: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("failed"),
		path: effect_Schema.String,
		message: effect_Schema.String,
		detailsText: effect_Schema.optionalKey(effect_Schema.String)
	})
]);
const ScriptInputValuesSchema = effect_Schema.Record(effect_Schema.String, ScriptInputValueSchema);
const defaultScriptInputValue = (field) => Object.hasOwn(field, "default") ? field.default : void 0;
const normalizeValue = (field, value) => {
	switch (field.type) {
		case "boolean": return typeof value === "boolean" ? value : void 0;
		case "number": return typeof value === "number" && Number.isFinite(value) ? value : void 0;
		case "multi-select": {
			if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) return;
			const selected = new Set(value);
			return field.options.filter((option) => selected.has(option));
		}
		case "select": return typeof value === "string" && field.options.includes(value) ? value : void 0;
		case "string": return typeof value === "string" ? value : void 0;
	}
};
const normalizeScriptInputValues = (definition, rawValues = {}) => {
	const values = {};
	for (const field of definition.fields) {
		const normalized = normalizeValue(field, rawValues[field.key]);
		if (normalized !== void 0) {
			values[field.key] = normalized;
			continue;
		}
		const fallback = defaultScriptInputValue(field);
		if (fallback !== void 0) {
			const normalizedFallback = normalizeValue(field, fallback);
			if (normalizedFallback !== void 0) values[field.key] = normalizedFallback;
		}
	}
	return values;
};
const findMissingRequiredScriptInputs = (definition, values) => definition.fields.flatMap((field) => {
	if (field.required !== true || !Object.hasOwn(values, field.key)) return field.required === true ? [field.key] : [];
	const value = values[field.key];
	return field.type === "multi-select" && (!Array.isArray(value) || value.length === 0) ? [field.key] : [];
});
const validateScriptInputValues = (definition, values) => {
	const normalized = normalizeScriptInputValues(definition, values);
	const fieldKeys = findMissingRequiredScriptInputs(definition, normalized);
	return fieldKeys.length === 0 ? {
		status: "ok",
		values: normalized
	} : {
		fieldKeys,
		status: "missing-required",
		values: normalized
	};
};
exports.ScriptBooleanInputFieldSchema = ScriptBooleanInputFieldSchema;
exports.ScriptFileReferenceSchema = ScriptFileReferenceSchema;
exports.ScriptFileResolutionSchema = ScriptFileResolutionSchema;
exports.ScriptFileSchema = ScriptFileSchema;
exports.ScriptInputFieldSchema = ScriptInputFieldSchema;
exports.ScriptInputTypeSchema = ScriptInputTypeSchema;
exports.ScriptInputValueSchema = ScriptInputValueSchema;
exports.ScriptInputValuesSchema = ScriptInputValuesSchema;
exports.ScriptInputsDefinitionSchema = ScriptInputsDefinitionSchema;
exports.ScriptMultiSelectInputFieldSchema = ScriptMultiSelectInputFieldSchema;
exports.ScriptNumberInputFieldSchema = ScriptNumberInputFieldSchema;
exports.ScriptOpenFileResultSchema = ScriptOpenFileResultSchema;
exports.ScriptSelectFileResultSchema = ScriptSelectFileResultSchema;
exports.ScriptSelectInputFieldSchema = ScriptSelectInputFieldSchema;
exports.ScriptStringInputFieldSchema = ScriptStringInputFieldSchema;
exports.defaultScriptInputValue = defaultScriptInputValue;
exports.findMissingRequiredScriptInputs = findMissingRequiredScriptInputs;
exports.normalizeScriptInputValues = normalizeScriptInputValues;
exports.validateScriptInputValues = validateScriptInputValues;
