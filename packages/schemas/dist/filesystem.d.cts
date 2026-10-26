import * as Schema from "effect/Schema";
declare const SCRIPT_FILE_SYSTEM_MAX_BYTES: number;
declare const SCRIPT_FILE_SYSTEM_MAX_ENTRIES = 1000;
declare const FileSystemOperationSchema: Schema.Literals<readonly ["exists", "list", "read-json", "read-text", "remove", "write-json", "write-text"]>;
type FileSystemOperation = typeof FileSystemOperationSchema.Type;
declare const FileSystemErrorReasonSchema: Schema.Literals<readonly ["invalid-path", "not-found", "not-file", "not-directory", "directory-not-empty", "too-large", "too-many-entries", "invalid-utf8", "invalid-json", "not-json-serializable", "permission-denied", "busy", "session-closed", "unavailable"]>;
type FileSystemErrorReason = typeof FileSystemErrorReasonSchema.Type;
declare const FileSystemFailureSchema: Schema.Struct<{
  readonly operation: Schema.Literals<readonly ["exists", "list", "read-json", "read-text", "remove", "write-json", "write-text"]>;
  readonly path: Schema.optionalKey<Schema.String>;
  readonly reason: Schema.Literals<readonly ["invalid-path", "not-found", "not-file", "not-directory", "directory-not-empty", "too-large", "too-many-entries", "invalid-utf8", "invalid-json", "not-json-serializable", "permission-denied", "busy", "session-closed", "unavailable"]>;
}>;
type FileSystemFailure = typeof FileSystemFailureSchema.Type;
declare const FileSystemError_base: Schema.Class<FileSystemError, Schema.TaggedStruct<"FileSystemError", {
  readonly operation: Schema.Literals<readonly ["exists", "list", "read-json", "read-text", "remove", "write-json", "write-text"]>;
  readonly path: Schema.optionalKey<Schema.String>;
  readonly reason: Schema.Literals<readonly ["invalid-path", "not-found", "not-file", "not-directory", "directory-not-empty", "too-large", "too-many-entries", "invalid-utf8", "invalid-json", "not-json-serializable", "permission-denied", "busy", "session-closed", "unavailable"]>;
}>, import("effect/Cause").YieldableError>;
/** Describes a failed filesystem action. */
declare class FileSystemError extends FileSystemError_base {
  get message(): string;
}
declare const FileSystemEntrySchema: Schema.Struct<{
  readonly kind: Schema.Literals<readonly ["file", "directory", "symbolic-link", "other"]>;
  readonly name: Schema.String;
}>;
type FileSystemEntry = typeof FileSystemEntrySchema.Type;
export { FileSystemEntry, FileSystemEntrySchema, FileSystemError, FileSystemErrorReason, FileSystemErrorReasonSchema, FileSystemFailure, FileSystemFailureSchema, FileSystemOperation, FileSystemOperationSchema, SCRIPT_FILE_SYSTEM_MAX_BYTES, SCRIPT_FILE_SYSTEM_MAX_ENTRIES };