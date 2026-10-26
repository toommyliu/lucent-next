Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
const require_rolldown_runtime = require("./rolldown-runtime-CmQyd0Gu.cjs");
const require_baseSchemas = require("./baseSchemas.cjs");
const require_scriptPath = require("./scriptPath.cjs");
let effect_Schema = require("effect/Schema");
effect_Schema = require_rolldown_runtime.__toESM(effect_Schema);
// src/scriptPackages.ts
const hasScriptPathSyntax = (value) => require_scriptPath.parseScriptPathSegments(value) !== null;
const isScriptPackageRepositorySubdirectory = (value) => value.trim() === value && value.normalize("NFC") === value && hasScriptPathSyntax(value);
const ScriptPackageRepositorySubdirectorySchema = effect_Schema.String.check(effect_Schema.makeFilter(isScriptPackageRepositorySubdirectory, { expected: "a portable repository-relative path using forward slashes" }));
const SCRIPT_BUILTIN_MODULE_SPECIFIERS = [
	"effect",
	"lucent/api",
	"lucent/autorelogin",
	"lucent/autozone",
	"lucent/filesystem",
	"lucent/http",
	"lucent/schema",
	"lucent/script"
];
const scriptBuiltinModuleSpecifiers = new Set(SCRIPT_BUILTIN_MODULE_SPECIFIERS);
const isScriptBuiltinModuleSpecifier = (specifier) => scriptBuiltinModuleSpecifiers.has(specifier);
const isReservedScriptPackageName = (name) => name === "effect" || name === "lucent" || name.startsWith("lucent/");
const ScriptPackageNameSchema = effect_Schema.String.check(effect_Schema.makeFilter((name) => name.trim() === name && !isReservedScriptPackageName(name) && hasScriptPathSyntax(name), { expected: "a safe, case-sensitive, non-reserved package name" }));
const ScriptPackageDirectorySchema = effect_Schema.String.check(effect_Schema.makeFilter((value) => value.trim() === value && value.normalize("NFC") === value && !value.includes("/") && hasScriptPathSyntax(value), { expected: "a portable single folder name" }));
const ScriptRelativePathSchema = effect_Schema.String.check(effect_Schema.makeFilter(hasScriptPathSyntax, { expected: "a safe relative path using forward slashes" }));
const ScriptReferenceSchema = effect_Schema.Union([effect_Schema.Struct({
	kind: effect_Schema.Literal("loose"),
	path: ScriptRelativePathSchema
}), effect_Schema.Struct({
	kind: effect_Schema.Literal("package"),
	packageName: ScriptPackageNameSchema,
	path: ScriptRelativePathSchema
})]);
const ScriptPackageCompatibilitySchema = effect_Schema.Union([
	effect_Schema.Struct({
		status: effect_Schema.Literal("compatible"),
		currentVersion: effect_Schema.String,
		requiredVersion: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("incompatible"),
		currentVersion: effect_Schema.String,
		requiredVersion: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("unknown"),
		currentVersion: effect_Schema.String,
		warning: effect_Schema.optionalKey(effect_Schema.String)
	})
]);
const ScriptModuleImportSchema = effect_Schema.Union([effect_Schema.Struct({
	kind: effect_Schema.Literal("builtin"),
	specifier: effect_Schema.Literals(SCRIPT_BUILTIN_MODULE_SPECIFIERS)
}), effect_Schema.Struct({
	kind: effect_Schema.Literal("module"),
	moduleId: effect_Schema.String
})]);
const ScriptModuleSourceSchema = effect_Schema.Struct({
	id: effect_Schema.String,
	format: effect_Schema.Literals(["commonjs", "unsupported-esm"]),
	imports: effect_Schema.Record(effect_Schema.String, ScriptModuleImportSchema),
	localPath: effect_Schema.String,
	path: effect_Schema.String,
	packageName: effect_Schema.optionalKey(effect_Schema.String),
	revision: effect_Schema.String,
	source: effect_Schema.String
});
const ScriptExecutionPackageSchema = effect_Schema.Struct({
	name: effect_Schema.String,
	rootPath: effect_Schema.String,
	mainModuleId: effect_Schema.NullOr(effect_Schema.String),
	compatibility: ScriptPackageCompatibilitySchema
});
const ScriptExecutionSnapshotSchema = effect_Schema.Struct({
	entryModuleId: effect_Schema.String,
	modules: effect_Schema.Array(ScriptModuleSourceSchema),
	packages: effect_Schema.Array(ScriptExecutionPackageSchema),
	revision: effect_Schema.String
});
const isRelativeScriptModuleSpecifier = (specifier) => specifier === "." || specifier === ".." || specifier.startsWith("./") || specifier.startsWith("../");
const resolveRelativeScriptModulePath = (importerPath, specifier) => {
	const base = importerPath.split("/").slice(0, -1);
	for (const segment of specifier.split("/")) {
		if (segment === "" || segment === ".") continue;
		if (segment === "..") {
			if (base.length === 0) return null;
			base.pop();
			continue;
		}
		base.push(segment);
	}
	return base.join("/");
};
const scriptModulePathCandidates = (path) => path.endsWith(".js") || path.endsWith(".cjs") ? [path] : [
	path,
	`${path}.js`,
	`${path}.cjs`,
	`${path}/index.js`,
	`${path}/index.cjs`
];
const ScriptCatalogEntrySchema = effect_Schema.Struct({
	name: effect_Schema.String,
	path: effect_Schema.String,
	relativePath: effect_Schema.String,
	packageName: effect_Schema.optionalKey(effect_Schema.String),
	reference: ScriptReferenceSchema
});
const ScriptPackageIntegritySchema = effect_Schema.Literals([
	"modified",
	"unmanaged",
	"verified"
]);
const ScriptPackageRepositorySourceFields = {
	repositoryUrl: effect_Schema.String,
	requestedRef: effect_Schema.optionalKey(effect_Schema.String),
	credentialId: effect_Schema.optionalKey(effect_Schema.String)
};
const ScriptPackageSourceSchema = effect_Schema.Union([effect_Schema.Struct({
	kind: effect_Schema.Literal("repository"),
	...ScriptPackageRepositorySourceFields,
	resolvedCommit: effect_Schema.String
}), effect_Schema.Struct({
	kind: effect_Schema.Literal("directory"),
	...ScriptPackageRepositorySourceFields,
	resolvedCommit: effect_Schema.optionalKey(effect_Schema.String),
	resolvedTree: effect_Schema.String,
	subdirectory: ScriptPackageRepositorySubdirectorySchema
})]);
const ScriptPackageRevisionSchema = effect_Schema.Union([effect_Schema.Struct({
	kind: effect_Schema.Literal("commit"),
	sha: effect_Schema.String
}), effect_Schema.Struct({
	kind: effect_Schema.Literal("tree"),
	sha: effect_Schema.String
})]);
const ScriptPackageUpdateStateSchema = effect_Schema.Union([
	effect_Schema.Struct({ status: effect_Schema.Literal("unchecked") }),
	effect_Schema.Struct({
		status: effect_Schema.Literal("current"),
		checkedAt: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("available"),
		checkedAt: effect_Schema.String,
		revision: ScriptPackageRevisionSchema
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("unknown"),
		checkedAt: effect_Schema.optionalKey(effect_Schema.String),
		message: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("rate-limited"),
		retryAt: effect_Schema.String,
		message: effect_Schema.String
	})
]);
const ScriptPackageDependencyIssueSchema = effect_Schema.Union([
	effect_Schema.Struct({
		reason: effect_Schema.Literal("missing"),
		packageName: effect_Schema.String,
		requiredVersion: effect_Schema.String
	}),
	effect_Schema.Struct({
		reason: effect_Schema.Literal("version-mismatch"),
		installedVersion: effect_Schema.String,
		packageName: effect_Schema.String,
		requiredVersion: effect_Schema.String
	}),
	effect_Schema.Struct({
		reason: effect_Schema.Literal("version-unavailable"),
		packageName: effect_Schema.String,
		requiredVersion: effect_Schema.String
	}),
	effect_Schema.Struct({
		reason: effect_Schema.Literal("unavailable"),
		packageName: effect_Schema.String,
		requiredVersion: effect_Schema.String
	})
]);
const ScriptPackageDependencyStatusSchema = effect_Schema.Union([effect_Schema.Struct({ status: effect_Schema.Literal("ready") }), effect_Schema.Struct({
	status: effect_Schema.Literal("blocked"),
	issues: effect_Schema.Array(ScriptPackageDependencyIssueSchema)
})]);
const ValidScriptPackageSchema = effect_Schema.Struct({
	status: effect_Schema.Literal("valid"),
	compatibility: ScriptPackageCompatibilitySchema,
	dependencyStatus: ScriptPackageDependencyStatusSchema,
	description: effect_Schema.optionalKey(effect_Schema.String),
	integrity: ScriptPackageIntegritySchema,
	name: effect_Schema.String,
	path: effect_Schema.String,
	source: effect_Schema.optionalKey(ScriptPackageSourceSchema),
	update: ScriptPackageUpdateStateSchema,
	version: effect_Schema.optionalKey(effect_Schema.String),
	warning: effect_Schema.optionalKey(effect_Schema.String)
});
const InvalidScriptPackageSchema = effect_Schema.Struct({
	status: effect_Schema.Literal("invalid"),
	diagnostic: effect_Schema.String,
	name: effect_Schema.optionalKey(effect_Schema.String),
	path: effect_Schema.String
});
const ScriptPackageSummarySchema = effect_Schema.Union([ValidScriptPackageSchema, InvalidScriptPackageSchema]);
const ScriptCatalogSchema = effect_Schema.Struct({
	packages: effect_Schema.Array(ScriptPackageSummarySchema),
	revision: effect_Schema.String,
	scripts: effect_Schema.Array(ScriptCatalogEntrySchema)
});
const SCRIPT_CATALOG_PAGE_MAX_ENTRIES = 1024;
const ScriptCatalogOverviewSchema = effect_Schema.Struct({
	packages: effect_Schema.Array(ScriptPackageSummarySchema),
	revision: effect_Schema.String,
	scriptCount: require_baseSchemas.NonNegativeInt
});
const ScriptCatalogPageRequestSchema = effect_Schema.Struct({
	limit: require_baseSchemas.boundedInt(1, SCRIPT_CATALOG_PAGE_MAX_ENTRIES),
	offset: require_baseSchemas.NonNegativeInt,
	query: effect_Schema.String,
	revision: effect_Schema.String
});
const ScriptCatalogPageSchema = effect_Schema.Struct({
	entries: effect_Schema.Array(ScriptCatalogEntrySchema),
	offset: require_baseSchemas.NonNegativeInt,
	revision: effect_Schema.String,
	total: require_baseSchemas.NonNegativeInt
});
const ScriptCatalogChangeSchema = effect_Schema.Struct({ revision: effect_Schema.String });
const GitHubCredentialSummarySchema = effect_Schema.Struct({
	id: effect_Schema.String,
	label: effect_Schema.String
});
const ScriptPackageNonEmptyStringSchema = effect_Schema.String.check(effect_Schema.makeFilter((value) => value.trim() !== "", { expected: "a non-empty string" }));
const GitHubCredentialWriteSchema = effect_Schema.Struct({
	id: effect_Schema.optionalKey(effect_Schema.String),
	label: ScriptPackageNonEmptyStringSchema,
	token: ScriptPackageNonEmptyStringSchema
});
const ScriptPackageInstallRequestSchema = effect_Schema.Struct({
	credentialId: effect_Schema.optionalKey(effect_Schema.String),
	ref: effect_Schema.optionalKey(effect_Schema.String),
	replaceExisting: effect_Schema.optionalKey(effect_Schema.Boolean),
	repositoryUrl: ScriptPackageNonEmptyStringSchema,
	subdirectory: effect_Schema.optionalKey(ScriptPackageRepositorySubdirectorySchema)
});
const ScriptPackageUpdateRequestSchema = effect_Schema.Struct({
	packageName: ScriptPackageNameSchema,
	replaceModified: effect_Schema.optionalKey(effect_Schema.Boolean)
});
const ScriptPackageRestoreRequestSchema = effect_Schema.Struct({ packageName: ScriptPackageNameSchema });
const ScriptPackageRemoveRequestSchema = effect_Schema.Struct({
	confirmModified: effect_Schema.optionalKey(effect_Schema.Boolean),
	packageName: ScriptPackageNameSchema
});
const ScriptPackageMutationResultSchema = effect_Schema.Union([
	effect_Schema.Struct({
		status: effect_Schema.Literal("completed"),
		catalog: ScriptCatalogOverviewSchema,
		packageName: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("unchanged"),
		catalog: ScriptCatalogOverviewSchema,
		packageName: effect_Schema.String
	}),
	effect_Schema.Struct({
		status: effect_Schema.Literal("confirmation-required"),
		packageName: effect_Schema.String,
		reason: effect_Schema.Literals(["existing-package", "local-modifications"])
	})
]);
exports.GitHubCredentialSummarySchema = GitHubCredentialSummarySchema;
exports.GitHubCredentialWriteSchema = GitHubCredentialWriteSchema;
exports.InvalidScriptPackageSchema = InvalidScriptPackageSchema;
exports.SCRIPT_BUILTIN_MODULE_SPECIFIERS = SCRIPT_BUILTIN_MODULE_SPECIFIERS;
exports.SCRIPT_CATALOG_PAGE_MAX_ENTRIES = SCRIPT_CATALOG_PAGE_MAX_ENTRIES;
exports.ScriptCatalogChangeSchema = ScriptCatalogChangeSchema;
exports.ScriptCatalogEntrySchema = ScriptCatalogEntrySchema;
exports.ScriptCatalogOverviewSchema = ScriptCatalogOverviewSchema;
exports.ScriptCatalogPageRequestSchema = ScriptCatalogPageRequestSchema;
exports.ScriptCatalogPageSchema = ScriptCatalogPageSchema;
exports.ScriptCatalogSchema = ScriptCatalogSchema;
exports.ScriptExecutionPackageSchema = ScriptExecutionPackageSchema;
exports.ScriptExecutionSnapshotSchema = ScriptExecutionSnapshotSchema;
exports.ScriptModuleImportSchema = ScriptModuleImportSchema;
exports.ScriptModuleSourceSchema = ScriptModuleSourceSchema;
exports.ScriptPackageCompatibilitySchema = ScriptPackageCompatibilitySchema;
exports.ScriptPackageDependencyIssueSchema = ScriptPackageDependencyIssueSchema;
exports.ScriptPackageDependencyStatusSchema = ScriptPackageDependencyStatusSchema;
exports.ScriptPackageDirectorySchema = ScriptPackageDirectorySchema;
exports.ScriptPackageInstallRequestSchema = ScriptPackageInstallRequestSchema;
exports.ScriptPackageIntegritySchema = ScriptPackageIntegritySchema;
exports.ScriptPackageMutationResultSchema = ScriptPackageMutationResultSchema;
exports.ScriptPackageNameSchema = ScriptPackageNameSchema;
exports.ScriptPackageRemoveRequestSchema = ScriptPackageRemoveRequestSchema;
exports.ScriptPackageRepositorySubdirectorySchema = ScriptPackageRepositorySubdirectorySchema;
exports.ScriptPackageRestoreRequestSchema = ScriptPackageRestoreRequestSchema;
exports.ScriptPackageRevisionSchema = ScriptPackageRevisionSchema;
exports.ScriptPackageSourceSchema = ScriptPackageSourceSchema;
exports.ScriptPackageSummarySchema = ScriptPackageSummarySchema;
exports.ScriptPackageUpdateRequestSchema = ScriptPackageUpdateRequestSchema;
exports.ScriptPackageUpdateStateSchema = ScriptPackageUpdateStateSchema;
exports.ScriptReferenceSchema = ScriptReferenceSchema;
exports.ScriptRelativePathSchema = ScriptRelativePathSchema;
exports.ValidScriptPackageSchema = ValidScriptPackageSchema;
exports.isRelativeScriptModuleSpecifier = isRelativeScriptModuleSpecifier;
exports.isScriptBuiltinModuleSpecifier = isScriptBuiltinModuleSpecifier;
exports.isScriptPackageRepositorySubdirectory = isScriptPackageRepositorySubdirectory;
exports.resolveRelativeScriptModulePath = resolveRelativeScriptModulePath;
exports.scriptModulePathCandidates = scriptModulePathCandidates;
