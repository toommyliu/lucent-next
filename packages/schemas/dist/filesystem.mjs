import * as Schema from "effect/Schema";
// src/filesystem.ts
const SCRIPT_FILE_SYSTEM_MAX_BYTES = 8388608;
const SCRIPT_FILE_SYSTEM_MAX_ENTRIES = 1e3;
const FileSystemOperationSchema = Schema.Literals([
	"exists",
	"list",
	"read-json",
	"read-text",
	"remove",
	"write-json",
	"write-text"
]);
const FileSystemErrorReasonSchema = Schema.Literals([
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
	path: Schema.optionalKey(Schema.String),
	reason: FileSystemErrorReasonSchema
};
const FileSystemFailureSchema = Schema.Struct(FileSystemErrorFields);
/** Describes a failed filesystem action. */
var FileSystemError = class extends Schema.TaggedError()("FileSystemError", FileSystemErrorFields) {
	get message() {
		const target = this.path === void 0 ? "" : ` at ${this.path}`;
		return `Filesystem ${this.operation} failed${target}: ${this.reason}.`;
	}
};
const FileSystemEntrySchema = Schema.Struct({
	kind: Schema.Literals([
		"file",
		"directory",
		"symbolic-link",
		"other"
	]),
	name: Schema.String
});
export { FileSystemEntrySchema, FileSystemError, FileSystemErrorReasonSchema, FileSystemFailureSchema, FileSystemOperationSchema, SCRIPT_FILE_SYSTEM_MAX_BYTES, SCRIPT_FILE_SYSTEM_MAX_ENTRIES };
