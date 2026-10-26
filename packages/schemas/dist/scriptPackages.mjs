import { NonNegativeInt, boundedInt } from "./baseSchemas.mjs";
import { parseScriptPathSegments } from "./scriptPath.mjs";
import * as Schema from "effect/Schema";
// src/scriptPackages.ts
const hasScriptPathSyntax = (value) => parseScriptPathSegments(value) !== null;
const isScriptPackageRepositorySubdirectory = (value) => value.trim() === value && value.normalize("NFC") === value && hasScriptPathSyntax(value);
const ScriptPackageRepositorySubdirectorySchema = Schema.String.check(Schema.makeFilter(isScriptPackageRepositorySubdirectory, { expected: "a portable repository-relative path using forward slashes" }));
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
const ScriptPackageNameSchema = Schema.String.check(Schema.makeFilter((name) => name.trim() === name && !isReservedScriptPackageName(name) && hasScriptPathSyntax(name), { expected: "a safe, case-sensitive, non-reserved package name" }));
const ScriptPackageDirectorySchema = Schema.String.check(Schema.makeFilter((value) => value.trim() === value && value.normalize("NFC") === value && !value.includes("/") && hasScriptPathSyntax(value), { expected: "a portable single folder name" }));
const ScriptRelativePathSchema = Schema.String.check(Schema.makeFilter(hasScriptPathSyntax, { expected: "a safe relative path using forward slashes" }));
const ScriptReferenceSchema = Schema.Union([Schema.Struct({
	kind: Schema.Literal("loose"),
	path: ScriptRelativePathSchema
}), Schema.Struct({
	kind: Schema.Literal("package"),
	packageName: ScriptPackageNameSchema,
	path: ScriptRelativePathSchema
})]);
const ScriptPackageCompatibilitySchema = Schema.Union([
	Schema.Struct({
		status: Schema.Literal("compatible"),
		currentVersion: Schema.String,
		requiredVersion: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("incompatible"),
		currentVersion: Schema.String,
		requiredVersion: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("unknown"),
		currentVersion: Schema.String,
		warning: Schema.optionalKey(Schema.String)
	})
]);
const ScriptModuleImportSchema = Schema.Union([Schema.Struct({
	kind: Schema.Literal("builtin"),
	specifier: Schema.Literals(SCRIPT_BUILTIN_MODULE_SPECIFIERS)
}), Schema.Struct({
	kind: Schema.Literal("module"),
	moduleId: Schema.String
})]);
const ScriptModuleSourceSchema = Schema.Struct({
	id: Schema.String,
	format: Schema.Literals(["commonjs", "unsupported-esm"]),
	imports: Schema.Record(Schema.String, ScriptModuleImportSchema),
	localPath: Schema.String,
	path: Schema.String,
	packageName: Schema.optionalKey(Schema.String),
	revision: Schema.String,
	source: Schema.String
});
const ScriptExecutionPackageSchema = Schema.Struct({
	name: Schema.String,
	rootPath: Schema.String,
	mainModuleId: Schema.NullOr(Schema.String),
	compatibility: ScriptPackageCompatibilitySchema
});
const ScriptExecutionSnapshotSchema = Schema.Struct({
	entryModuleId: Schema.String,
	modules: Schema.Array(ScriptModuleSourceSchema),
	packages: Schema.Array(ScriptExecutionPackageSchema),
	revision: Schema.String
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
const ScriptCatalogEntrySchema = Schema.Struct({
	name: Schema.String,
	path: Schema.String,
	relativePath: Schema.String,
	packageName: Schema.optionalKey(Schema.String),
	reference: ScriptReferenceSchema
});
const ScriptPackageIntegritySchema = Schema.Literals([
	"modified",
	"unmanaged",
	"verified"
]);
const ScriptPackageRepositorySourceFields = {
	repositoryUrl: Schema.String,
	requestedRef: Schema.optionalKey(Schema.String),
	credentialId: Schema.optionalKey(Schema.String)
};
const ScriptPackageSourceSchema = Schema.Union([Schema.Struct({
	kind: Schema.Literal("repository"),
	...ScriptPackageRepositorySourceFields,
	resolvedCommit: Schema.String
}), Schema.Struct({
	kind: Schema.Literal("directory"),
	...ScriptPackageRepositorySourceFields,
	resolvedCommit: Schema.optionalKey(Schema.String),
	resolvedTree: Schema.String,
	subdirectory: ScriptPackageRepositorySubdirectorySchema
})]);
const ScriptPackageRevisionSchema = Schema.Union([Schema.Struct({
	kind: Schema.Literal("commit"),
	sha: Schema.String
}), Schema.Struct({
	kind: Schema.Literal("tree"),
	sha: Schema.String
})]);
const ScriptPackageUpdateStateSchema = Schema.Union([
	Schema.Struct({ status: Schema.Literal("unchecked") }),
	Schema.Struct({
		status: Schema.Literal("current"),
		checkedAt: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("available"),
		checkedAt: Schema.String,
		revision: ScriptPackageRevisionSchema
	}),
	Schema.Struct({
		status: Schema.Literal("unknown"),
		checkedAt: Schema.optionalKey(Schema.String),
		message: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("rate-limited"),
		retryAt: Schema.String,
		message: Schema.String
	})
]);
const ScriptPackageDependencyIssueSchema = Schema.Union([
	Schema.Struct({
		reason: Schema.Literal("missing"),
		packageName: Schema.String,
		requiredVersion: Schema.String
	}),
	Schema.Struct({
		reason: Schema.Literal("version-mismatch"),
		installedVersion: Schema.String,
		packageName: Schema.String,
		requiredVersion: Schema.String
	}),
	Schema.Struct({
		reason: Schema.Literal("version-unavailable"),
		packageName: Schema.String,
		requiredVersion: Schema.String
	}),
	Schema.Struct({
		reason: Schema.Literal("unavailable"),
		packageName: Schema.String,
		requiredVersion: Schema.String
	})
]);
const ScriptPackageDependencyStatusSchema = Schema.Union([Schema.Struct({ status: Schema.Literal("ready") }), Schema.Struct({
	status: Schema.Literal("blocked"),
	issues: Schema.Array(ScriptPackageDependencyIssueSchema)
})]);
const ValidScriptPackageSchema = Schema.Struct({
	status: Schema.Literal("valid"),
	compatibility: ScriptPackageCompatibilitySchema,
	dependencyStatus: ScriptPackageDependencyStatusSchema,
	description: Schema.optionalKey(Schema.String),
	integrity: ScriptPackageIntegritySchema,
	name: Schema.String,
	path: Schema.String,
	source: Schema.optionalKey(ScriptPackageSourceSchema),
	update: ScriptPackageUpdateStateSchema,
	version: Schema.optionalKey(Schema.String),
	warning: Schema.optionalKey(Schema.String)
});
const InvalidScriptPackageSchema = Schema.Struct({
	status: Schema.Literal("invalid"),
	diagnostic: Schema.String,
	name: Schema.optionalKey(Schema.String),
	path: Schema.String
});
const ScriptPackageSummarySchema = Schema.Union([ValidScriptPackageSchema, InvalidScriptPackageSchema]);
const ScriptCatalogSchema = Schema.Struct({
	packages: Schema.Array(ScriptPackageSummarySchema),
	revision: Schema.String,
	scripts: Schema.Array(ScriptCatalogEntrySchema)
});
const SCRIPT_CATALOG_PAGE_MAX_ENTRIES = 1024;
const ScriptCatalogOverviewSchema = Schema.Struct({
	packages: Schema.Array(ScriptPackageSummarySchema),
	revision: Schema.String,
	scriptCount: NonNegativeInt
});
const ScriptCatalogPageRequestSchema = Schema.Struct({
	limit: boundedInt(1, SCRIPT_CATALOG_PAGE_MAX_ENTRIES),
	offset: NonNegativeInt,
	query: Schema.String,
	revision: Schema.String
});
const ScriptCatalogPageSchema = Schema.Struct({
	entries: Schema.Array(ScriptCatalogEntrySchema),
	offset: NonNegativeInt,
	revision: Schema.String,
	total: NonNegativeInt
});
const ScriptCatalogChangeSchema = Schema.Struct({ revision: Schema.String });
const GitHubCredentialSummarySchema = Schema.Struct({
	id: Schema.String,
	label: Schema.String
});
const ScriptPackageNonEmptyStringSchema = Schema.String.check(Schema.makeFilter((value) => value.trim() !== "", { expected: "a non-empty string" }));
const GitHubCredentialWriteSchema = Schema.Struct({
	id: Schema.optionalKey(Schema.String),
	label: ScriptPackageNonEmptyStringSchema,
	token: ScriptPackageNonEmptyStringSchema
});
const ScriptPackageInstallRequestSchema = Schema.Struct({
	credentialId: Schema.optionalKey(Schema.String),
	ref: Schema.optionalKey(Schema.String),
	replaceExisting: Schema.optionalKey(Schema.Boolean),
	repositoryUrl: ScriptPackageNonEmptyStringSchema,
	subdirectory: Schema.optionalKey(ScriptPackageRepositorySubdirectorySchema)
});
const ScriptPackageUpdateRequestSchema = Schema.Struct({
	packageName: ScriptPackageNameSchema,
	replaceModified: Schema.optionalKey(Schema.Boolean)
});
const ScriptPackageRestoreRequestSchema = Schema.Struct({ packageName: ScriptPackageNameSchema });
const ScriptPackageRemoveRequestSchema = Schema.Struct({
	confirmModified: Schema.optionalKey(Schema.Boolean),
	packageName: ScriptPackageNameSchema
});
const ScriptPackageMutationResultSchema = Schema.Union([
	Schema.Struct({
		status: Schema.Literal("completed"),
		catalog: ScriptCatalogOverviewSchema,
		packageName: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("unchanged"),
		catalog: ScriptCatalogOverviewSchema,
		packageName: Schema.String
	}),
	Schema.Struct({
		status: Schema.Literal("confirmation-required"),
		packageName: Schema.String,
		reason: Schema.Literals(["existing-package", "local-modifications"])
	})
]);
export { GitHubCredentialSummarySchema, GitHubCredentialWriteSchema, InvalidScriptPackageSchema, SCRIPT_BUILTIN_MODULE_SPECIFIERS, SCRIPT_CATALOG_PAGE_MAX_ENTRIES, ScriptCatalogChangeSchema, ScriptCatalogEntrySchema, ScriptCatalogOverviewSchema, ScriptCatalogPageRequestSchema, ScriptCatalogPageSchema, ScriptCatalogSchema, ScriptExecutionPackageSchema, ScriptExecutionSnapshotSchema, ScriptModuleImportSchema, ScriptModuleSourceSchema, ScriptPackageCompatibilitySchema, ScriptPackageDependencyIssueSchema, ScriptPackageDependencyStatusSchema, ScriptPackageDirectorySchema, ScriptPackageInstallRequestSchema, ScriptPackageIntegritySchema, ScriptPackageMutationResultSchema, ScriptPackageNameSchema, ScriptPackageRemoveRequestSchema, ScriptPackageRepositorySubdirectorySchema, ScriptPackageRestoreRequestSchema, ScriptPackageRevisionSchema, ScriptPackageSourceSchema, ScriptPackageSummarySchema, ScriptPackageUpdateRequestSchema, ScriptPackageUpdateStateSchema, ScriptReferenceSchema, ScriptRelativePathSchema, ValidScriptPackageSchema, isRelativeScriptModuleSpecifier, isScriptBuiltinModuleSpecifier, isScriptPackageRepositorySubdirectory, resolveRelativeScriptModulePath, scriptModulePathCandidates };
