import { ScriptExecutionSnapshotSchema, ScriptReferenceSchema } from "./scriptPackages.mjs";
import * as Schema from "effect/Schema";
// src/scriptInputs.ts
const ScriptInputValueSchema = Schema.Union([
	Schema.String,
	Schema.Number,
	Schema.Boolean,
	Schema.Array(Schema.String)
]);
const ScriptInputTypeSchema = Schema.Literals([
	"string",
	"number",
	"boolean",
	"select",
	"multi-select"
]);
const ScriptInputFieldBaseSchema = {
	key: Schema.String,
	label: Schema.String,
	description: Schema.optionalKey(Schema.String),
	required: Schema.optionalKey(Schema.Boolean)
};
const ScriptStringInputFieldSchema = Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: Schema.Literal("string"),
	default: Schema.optionalKey(Schema.String)
});
const ScriptNumberInputFieldSchema = Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: Schema.Literal("number"),
	default: Schema.optionalKey(Schema.Number)
});
const ScriptBooleanInputFieldSchema = Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: Schema.Literal("boolean"),
	default: Schema.optionalKey(Schema.Boolean)
});
const ScriptSelectInputFieldSchema = Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: Schema.Literal("select"),
	options: Schema.Array(Schema.String),
	default: Schema.optionalKey(Schema.String)
}).check(Schema.makeFilter(({ default: defaultValue, options }) => options.length > 0 && (defaultValue === void 0 || options.includes(defaultValue)), { expected: "select input options to be non-empty and contain the default value" }));
const ScriptMultiSelectInputFieldSchema = Schema.Struct({
	...ScriptInputFieldBaseSchema,
	type: Schema.Literal("multi-select"),
	options: Schema.Array(Schema.String),
	default: Schema.optionalKey(Schema.Array(Schema.String))
}).check(Schema.makeFilter(({ default: defaultValue, options }) => {
	const optionSet = new Set(options);
	return options.length > 0 && optionSet.size === options.length && (defaultValue === void 0 || new Set(defaultValue).size === defaultValue.length && defaultValue.every((value) => optionSet.has(value)));
}, { expected: "multi-select input options to be non-empty and unique, and contain every unique default value" }));
const ScriptInputFieldSchema = Schema.Union([
	ScriptStringInputFieldSchema,
	ScriptNumberInputFieldSchema,
	ScriptBooleanInputFieldSchema,
	ScriptSelectInputFieldSchema,
	ScriptMultiSelectInputFieldSchema
]);
const ScriptInputsDefinitionSchema = Schema.Struct({
	id: Schema.String,
	fields: Schema.Array(ScriptInputFieldSchema)
}).check(Schema.makeFilter(({ fields }) => new Set(fields.map((field) => field.key)).size === fields.length, { expected: "script input field keys to be unique" }));
const ScriptFileReferenceSchema = Schema.Struct({
	path: Schema.String,
	name: Schema.String,
	reference: Schema.optionalKey(ScriptReferenceSchema)
});
const ScriptFileSchema = Schema.Struct({
	path: Schema.String,
	name: Schema.String,
	source: Schema.String,
	inputs: Schema.NullOr(ScriptInputsDefinitionSchema),
	revision: Schema.String,
	reference: Schema.optionalKey(ScriptReferenceSchema),
	snapshot: Schema.optionalKey(ScriptExecutionSnapshotSchema)
});
const ScriptOpenFileResultSchema = Schema.Union([Schema.Struct({ canceled: Schema.Literal(true) }), Schema.Struct({
	canceled: Schema.Literal(false),
	file: ScriptFileSchema
})]);
const ScriptSelectFileResultSchema = Schema.Union([Schema.Struct({ canceled: Schema.Literal(true) }), Schema.Struct({
	canceled: Schema.Literal(false),
	file: ScriptFileReferenceSchema
})]);
const ScriptFileResolutionSchema = Schema.Union([
	Schema.Struct({
		status: Schema.Literal("found"),
		file: ScriptFileSchema
	}),
	Schema.Struct({
		status: Schema.Literal("missing"),
		path: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("failed"),
		path: Schema.String,
		message: Schema.String,
		detailsText: Schema.optionalKey(Schema.String)
	})
]);
const ScriptInputValuesSchema = Schema.Record(Schema.String, ScriptInputValueSchema);
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
export { ScriptBooleanInputFieldSchema, ScriptFileReferenceSchema, ScriptFileResolutionSchema, ScriptFileSchema, ScriptInputFieldSchema, ScriptInputTypeSchema, ScriptInputValueSchema, ScriptInputValuesSchema, ScriptInputsDefinitionSchema, ScriptMultiSelectInputFieldSchema, ScriptNumberInputFieldSchema, ScriptOpenFileResultSchema, ScriptSelectFileResultSchema, ScriptSelectInputFieldSchema, ScriptStringInputFieldSchema, defaultScriptInputValue, findMissingRequiredScriptInputs, normalizeScriptInputValues, validateScriptInputValues };
