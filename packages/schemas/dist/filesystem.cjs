Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/filesystem.ts
const SCRIPT_FILE_SYSTEM_MAX_BYTES = 8388608;
const SCRIPT_FILE_SYSTEM_MAX_ENTRIES = 1e3;
const FileSystemOperationSchema = effect_Schema.Literals([
	"exists",
	"list",
	"read-json",
	"read-text",
	"remove",
	"write-json",
	"write-text"
]);
const FileSystemErrorReasonSchema = effect_Schema.Literals([
	"invalid-path",
	"not-found",
	"not-file",
	"not-directory",
	"directory-not-empty",
	"too-large",
	"too-many-entries",
	"invalid-utf8",
	"invalid-json",
	"not-json-serializable",
	"permission-denied",
	"busy",
	"session-closed",
	"unavailable"
]);
const FileSystemErrorFields = {
	operation: FileSystemOperationSchema,
	path: effect_Schema.optionalKey(effect_Schema.String),
	reason: FileSystemErrorReasonSchema
};
const FileSystemFailureSchema = effect_Schema.Struct(FileSystemErrorFields);
/** Describes a failed filesystem action. */
var FileSystemError = class extends effect_Schema.TaggedError()("FileSystemError", FileSystemErrorFields) {
	get message() {
		const target = this.path === void 0 ? "" : ` at ${this.path}`;
		return `Filesystem ${this.operation} failed${target}: ${this.reason}.`;
	}
};
const FileSystemEntrySchema = effect_Schema.Struct({
	kind: effect_Schema.Literals([
		"file",
		"directory",
		"symbolic-link",
		"other"
	]),
	name: effect_Schema.String
});
exports.FileSystemEntrySchema = FileSystemEntrySchema;
exports.FileSystemError = FileSystemError;
exports.FileSystemErrorReasonSchema = FileSystemErrorReasonSchema;
exports.FileSystemFailureSchema = FileSystemFailureSchema;
exports.FileSystemOperationSchema = FileSystemOperationSchema;
exports.SCRIPT_FILE_SYSTEM_MAX_BYTES = SCRIPT_FILE_SYSTEM_MAX_BYTES;
exports.SCRIPT_FILE_SYSTEM_MAX_ENTRIES = SCRIPT_FILE_SYSTEM_MAX_ENTRIES;
