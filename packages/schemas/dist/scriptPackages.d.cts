import * as Schema from "effect/Schema";
declare const isScriptPackageRepositorySubdirectory: (value: string) => boolean;
declare const ScriptPackageRepositorySubdirectorySchema: Schema.String;
type ScriptPackageRepositorySubdirectory = typeof ScriptPackageRepositorySubdirectorySchema.Type;
declare const SCRIPT_BUILTIN_MODULE_SPECIFIERS: readonly ["effect", "lucent/api", "lucent/autorelogin", "lucent/autozone", "lucent/filesystem", "lucent/http", "lucent/schema", "lucent/script"];
type ScriptBuiltinModuleSpecifier = (typeof SCRIPT_BUILTIN_MODULE_SPECIFIERS)[number];
declare const isScriptBuiltinModuleSpecifier: (specifier: string) => specifier is ScriptBuiltinModuleSpecifier;
declare const ScriptPackageNameSchema: Schema.String;
type ScriptPackageName = typeof ScriptPackageNameSchema.Type;
declare const ScriptPackageDirectorySchema: Schema.String;
type ScriptPackageDirectory = typeof ScriptPackageDirectorySchema.Type;
declare const ScriptRelativePathSchema: Schema.String;
declare const ScriptReferenceSchema: Schema.Union<readonly [Schema.Struct<{
  readonly kind: Schema.Literal<"loose">;
  readonly path: Schema.String;
}>, Schema.Struct<{
  readonly kind: Schema.Literal<"package">;
  readonly packageName: Schema.String;
  readonly path: Schema.String;
}>]>;
type ScriptReference = typeof ScriptReferenceSchema.Type;
declare const ScriptPackageCompatibilitySchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"compatible">;
  readonly currentVersion: Schema.String;
  readonly requiredVersion: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"incompatible">;
  readonly currentVersion: Schema.String;
  readonly requiredVersion: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"unknown">;
  readonly currentVersion: Schema.String;
  readonly warning: Schema.optionalKey<Schema.String>;
}>]>;
type ScriptPackageCompatibility = typeof ScriptPackageCompatibilitySchema.Type;
declare const ScriptModuleImportSchema: Schema.Union<readonly [Schema.Struct<{
  readonly kind: Schema.Literal<"builtin">;
  readonly specifier: Schema.Literals<readonly ["effect", "lucent/api", "lucent/autorelogin", "lucent/autozone", "lucent/filesystem", "lucent/http", "lucent/schema", "lucent/script"]>;
}>, Schema.Struct<{
  readonly kind: Schema.Literal<"module">;
  readonly moduleId: Schema.String;
}>]>;
type ScriptModuleImport = typeof ScriptModuleImportSchema.Type;
declare const ScriptModuleSourceSchema: Schema.Struct<{
  readonly id: Schema.String;
  readonly format: Schema.Literals<readonly ["commonjs", "unsupported-esm"]>;
  readonly imports: Schema.$Record<Schema.String, Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"builtin">;
    readonly specifier: Schema.Literals<readonly ["effect", "lucent/api", "lucent/autorelogin", "lucent/autozone", "lucent/filesystem", "lucent/http", "lucent/schema", "lucent/script"]>;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"module">;
    readonly moduleId: Schema.String;
  }>]>>;
  readonly localPath: Schema.String;
  readonly path: Schema.String;
  readonly packageName: Schema.optionalKey<Schema.String>;
  readonly revision: Schema.String;
  readonly source: Schema.String;
}>;
type ScriptModuleSource = typeof ScriptModuleSourceSchema.Type;
declare const ScriptExecutionPackageSchema: Schema.Struct<{
  readonly name: Schema.String;
  readonly rootPath: Schema.String;
  readonly mainModuleId: Schema.NullOr<Schema.String>;
  readonly compatibility: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"compatible">;
    readonly currentVersion: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"incompatible">;
    readonly currentVersion: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"unknown">;
    readonly currentVersion: Schema.String;
    readonly warning: Schema.optionalKey<Schema.String>;
  }>]>;
}>;
type ScriptExecutionPackage = typeof ScriptExecutionPackageSchema.Type;
declare const ScriptExecutionSnapshotSchema: Schema.Struct<{
  readonly entryModuleId: Schema.String;
  readonly modules: Schema.$Array<Schema.Struct<{
    readonly id: Schema.String;
    readonly format: Schema.Literals<readonly ["commonjs", "unsupported-esm"]>;
    readonly imports: Schema.$Record<Schema.String, Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"builtin">;
      readonly specifier: Schema.Literals<readonly ["effect", "lucent/api", "lucent/autorelogin", "lucent/autozone", "lucent/filesystem", "lucent/http", "lucent/schema", "lucent/script"]>;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"module">;
      readonly moduleId: Schema.String;
    }>]>>;
    readonly localPath: Schema.String;
    readonly path: Schema.String;
    readonly packageName: Schema.optionalKey<Schema.String>;
    readonly revision: Schema.String;
    readonly source: Schema.String;
  }>>;
  readonly packages: Schema.$Array<Schema.Struct<{
    readonly name: Schema.String;
    readonly rootPath: Schema.String;
    readonly mainModuleId: Schema.NullOr<Schema.String>;
    readonly compatibility: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"compatible">;
      readonly currentVersion: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"incompatible">;
      readonly currentVersion: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"unknown">;
      readonly currentVersion: Schema.String;
      readonly warning: Schema.optionalKey<Schema.String>;
    }>]>;
  }>>;
  readonly revision: Schema.String;
}>;
type ScriptExecutionSnapshot = typeof ScriptExecutionSnapshotSchema.Type;
declare const isRelativeScriptModuleSpecifier: (specifier: string) => boolean;
declare const resolveRelativeScriptModulePath: (importerPath: string, specifier: string) => string | null;
declare const scriptModulePathCandidates: (path: string) => readonly string[];
declare const ScriptCatalogEntrySchema: Schema.Struct<{
  readonly name: Schema.String;
  readonly path: Schema.String;
  readonly relativePath: Schema.String;
  readonly packageName: Schema.optionalKey<Schema.String>;
  readonly reference: Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"loose">;
    readonly path: Schema.String;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"package">;
    readonly packageName: Schema.String;
    readonly path: Schema.String;
  }>]>;
}>;
type ScriptCatalogEntry = typeof ScriptCatalogEntrySchema.Type;
declare const ScriptPackageIntegritySchema: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
type ScriptPackageIntegrity = typeof ScriptPackageIntegritySchema.Type;
declare const ScriptPackageSourceSchema: Schema.Union<readonly [Schema.Struct<{
  readonly resolvedCommit: Schema.String;
  readonly repositoryUrl: Schema.String;
  readonly requestedRef: Schema.optionalKey<Schema.String>;
  readonly credentialId: Schema.optionalKey<Schema.String>;
  readonly kind: Schema.Literal<"repository">;
}>, Schema.Struct<{
  readonly resolvedCommit: Schema.optionalKey<Schema.String>;
  readonly resolvedTree: Schema.String;
  readonly subdirectory: Schema.String;
  readonly repositoryUrl: Schema.String;
  readonly requestedRef: Schema.optionalKey<Schema.String>;
  readonly credentialId: Schema.optionalKey<Schema.String>;
  readonly kind: Schema.Literal<"directory">;
}>]>;
type ScriptPackageSource = typeof ScriptPackageSourceSchema.Type;
declare const ScriptPackageRevisionSchema: Schema.Union<readonly [Schema.Struct<{
  readonly kind: Schema.Literal<"commit">;
  readonly sha: Schema.String;
}>, Schema.Struct<{
  readonly kind: Schema.Literal<"tree">;
  readonly sha: Schema.String;
}>]>;
type ScriptPackageRevision = typeof ScriptPackageRevisionSchema.Type;
declare const ScriptPackageUpdateStateSchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"unchecked">;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"current">;
  readonly checkedAt: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"available">;
  readonly checkedAt: Schema.String;
  readonly revision: Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"commit">;
    readonly sha: Schema.String;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"tree">;
    readonly sha: Schema.String;
  }>]>;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"unknown">;
  readonly checkedAt: Schema.optionalKey<Schema.String>;
  readonly message: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"rate-limited">;
  readonly retryAt: Schema.String;
  readonly message: Schema.String;
}>]>;
type ScriptPackageUpdateState = typeof ScriptPackageUpdateStateSchema.Type;
declare const ScriptPackageDependencyIssueSchema: Schema.Union<readonly [Schema.Struct<{
  readonly reason: Schema.Literal<"missing">;
  readonly packageName: Schema.String;
  readonly requiredVersion: Schema.String;
}>, Schema.Struct<{
  readonly reason: Schema.Literal<"version-mismatch">;
  readonly installedVersion: Schema.String;
  readonly packageName: Schema.String;
  readonly requiredVersion: Schema.String;
}>, Schema.Struct<{
  readonly reason: Schema.Literal<"version-unavailable">;
  readonly packageName: Schema.String;
  readonly requiredVersion: Schema.String;
}>, Schema.Struct<{
  readonly reason: Schema.Literal<"unavailable">;
  readonly packageName: Schema.String;
  readonly requiredVersion: Schema.String;
}>]>;
type ScriptPackageDependencyIssue = typeof ScriptPackageDependencyIssueSchema.Type;
declare const ScriptPackageDependencyStatusSchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"ready">;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"blocked">;
  readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
    readonly reason: Schema.Literal<"missing">;
    readonly packageName: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly reason: Schema.Literal<"version-mismatch">;
    readonly installedVersion: Schema.String;
    readonly packageName: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly reason: Schema.Literal<"version-unavailable">;
    readonly packageName: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly reason: Schema.Literal<"unavailable">;
    readonly packageName: Schema.String;
    readonly requiredVersion: Schema.String;
  }>]>>;
}>]>;
type ScriptPackageDependencyStatus = typeof ScriptPackageDependencyStatusSchema.Type;
declare const ValidScriptPackageSchema: Schema.Struct<{
  readonly status: Schema.Literal<"valid">;
  readonly compatibility: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"compatible">;
    readonly currentVersion: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"incompatible">;
    readonly currentVersion: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"unknown">;
    readonly currentVersion: Schema.String;
    readonly warning: Schema.optionalKey<Schema.String>;
  }>]>;
  readonly dependencyStatus: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"ready">;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"blocked">;
    readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
      readonly reason: Schema.Literal<"missing">;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly reason: Schema.Literal<"version-mismatch">;
      readonly installedVersion: Schema.String;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly reason: Schema.Literal<"version-unavailable">;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly reason: Schema.Literal<"unavailable">;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>]>>;
  }>]>;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly integrity: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
  readonly name: Schema.String;
  readonly path: Schema.String;
  readonly source: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
    readonly resolvedCommit: Schema.String;
    readonly repositoryUrl: Schema.String;
    readonly requestedRef: Schema.optionalKey<Schema.String>;
    readonly credentialId: Schema.optionalKey<Schema.String>;
    readonly kind: Schema.Literal<"repository">;
  }>, Schema.Struct<{
    readonly resolvedCommit: Schema.optionalKey<Schema.String>;
    readonly resolvedTree: Schema.String;
    readonly subdirectory: Schema.String;
    readonly repositoryUrl: Schema.String;
    readonly requestedRef: Schema.optionalKey<Schema.String>;
    readonly credentialId: Schema.optionalKey<Schema.String>;
    readonly kind: Schema.Literal<"directory">;
  }>]>>;
  readonly update: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"unchecked">;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"current">;
    readonly checkedAt: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"available">;
    readonly checkedAt: Schema.String;
    readonly revision: Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"commit">;
      readonly sha: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"tree">;
      readonly sha: Schema.String;
    }>]>;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"unknown">;
    readonly checkedAt: Schema.optionalKey<Schema.String>;
    readonly message: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"rate-limited">;
    readonly retryAt: Schema.String;
    readonly message: Schema.String;
  }>]>;
  readonly version: Schema.optionalKey<Schema.String>;
  readonly warning: Schema.optionalKey<Schema.String>;
}>;
type ValidScriptPackage = typeof ValidScriptPackageSchema.Type;
declare const InvalidScriptPackageSchema: Schema.Struct<{
  readonly status: Schema.Literal<"invalid">;
  readonly diagnostic: Schema.String;
  readonly name: Schema.optionalKey<Schema.String>;
  readonly path: Schema.String;
}>;
type InvalidScriptPackage = typeof InvalidScriptPackageSchema.Type;
declare const ScriptPackageSummarySchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"valid">;
  readonly compatibility: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"compatible">;
    readonly currentVersion: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"incompatible">;
    readonly currentVersion: Schema.String;
    readonly requiredVersion: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"unknown">;
    readonly currentVersion: Schema.String;
    readonly warning: Schema.optionalKey<Schema.String>;
  }>]>;
  readonly dependencyStatus: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"ready">;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"blocked">;
    readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
      readonly reason: Schema.Literal<"missing">;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly reason: Schema.Literal<"version-mismatch">;
      readonly installedVersion: Schema.String;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly reason: Schema.Literal<"version-unavailable">;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly reason: Schema.Literal<"unavailable">;
      readonly packageName: Schema.String;
      readonly requiredVersion: Schema.String;
    }>]>>;
  }>]>;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly integrity: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
  readonly name: Schema.String;
  readonly path: Schema.String;
  readonly source: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
    readonly resolvedCommit: Schema.String;
    readonly repositoryUrl: Schema.String;
    readonly requestedRef: Schema.optionalKey<Schema.String>;
    readonly credentialId: Schema.optionalKey<Schema.String>;
    readonly kind: Schema.Literal<"repository">;
  }>, Schema.Struct<{
    readonly resolvedCommit: Schema.optionalKey<Schema.String>;
    readonly resolvedTree: Schema.String;
    readonly subdirectory: Schema.String;
    readonly repositoryUrl: Schema.String;
    readonly requestedRef: Schema.optionalKey<Schema.String>;
    readonly credentialId: Schema.optionalKey<Schema.String>;
    readonly kind: Schema.Literal<"directory">;
  }>]>>;
  readonly update: Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"unchecked">;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"current">;
    readonly checkedAt: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"available">;
    readonly checkedAt: Schema.String;
    readonly revision: Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"commit">;
      readonly sha: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"tree">;
      readonly sha: Schema.String;
    }>]>;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"unknown">;
    readonly checkedAt: Schema.optionalKey<Schema.String>;
    readonly message: Schema.String;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"rate-limited">;
    readonly retryAt: Schema.String;
    readonly message: Schema.String;
  }>]>;
  readonly version: Schema.optionalKey<Schema.String>;
  readonly warning: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"invalid">;
  readonly diagnostic: Schema.String;
  readonly name: Schema.optionalKey<Schema.String>;
  readonly path: Schema.String;
}>]>;
type ScriptPackageSummary = typeof ScriptPackageSummarySchema.Type;
declare const ScriptCatalogSchema: Schema.Struct<{
  readonly packages: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"valid">;
    readonly compatibility: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"compatible">;
      readonly currentVersion: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"incompatible">;
      readonly currentVersion: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"unknown">;
      readonly currentVersion: Schema.String;
      readonly warning: Schema.optionalKey<Schema.String>;
    }>]>;
    readonly dependencyStatus: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"ready">;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"blocked">;
      readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
        readonly reason: Schema.Literal<"missing">;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly reason: Schema.Literal<"version-mismatch">;
        readonly installedVersion: Schema.String;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly reason: Schema.Literal<"version-unavailable">;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly reason: Schema.Literal<"unavailable">;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>]>>;
    }>]>;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly integrity: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
    readonly name: Schema.String;
    readonly path: Schema.String;
    readonly source: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
      readonly resolvedCommit: Schema.String;
      readonly repositoryUrl: Schema.String;
      readonly requestedRef: Schema.optionalKey<Schema.String>;
      readonly credentialId: Schema.optionalKey<Schema.String>;
      readonly kind: Schema.Literal<"repository">;
    }>, Schema.Struct<{
      readonly resolvedCommit: Schema.optionalKey<Schema.String>;
      readonly resolvedTree: Schema.String;
      readonly subdirectory: Schema.String;
      readonly repositoryUrl: Schema.String;
      readonly requestedRef: Schema.optionalKey<Schema.String>;
      readonly credentialId: Schema.optionalKey<Schema.String>;
      readonly kind: Schema.Literal<"directory">;
    }>]>>;
    readonly update: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"unchecked">;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"current">;
      readonly checkedAt: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"available">;
      readonly checkedAt: Schema.String;
      readonly revision: Schema.Union<readonly [Schema.Struct<{
        readonly kind: Schema.Literal<"commit">;
        readonly sha: Schema.String;
      }>, Schema.Struct<{
        readonly kind: Schema.Literal<"tree">;
        readonly sha: Schema.String;
      }>]>;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"unknown">;
      readonly checkedAt: Schema.optionalKey<Schema.String>;
      readonly message: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"rate-limited">;
      readonly retryAt: Schema.String;
      readonly message: Schema.String;
    }>]>;
    readonly version: Schema.optionalKey<Schema.String>;
    readonly warning: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"invalid">;
    readonly diagnostic: Schema.String;
    readonly name: Schema.optionalKey<Schema.String>;
    readonly path: Schema.String;
  }>]>>;
  readonly revision: Schema.String;
  readonly scripts: Schema.$Array<Schema.Struct<{
    readonly name: Schema.String;
    readonly path: Schema.String;
    readonly relativePath: Schema.String;
    readonly packageName: Schema.optionalKey<Schema.String>;
    readonly reference: Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"loose">;
      readonly path: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"package">;
      readonly packageName: Schema.String;
      readonly path: Schema.String;
    }>]>;
  }>>;
}>;
type ScriptCatalog = typeof ScriptCatalogSchema.Type;
declare const SCRIPT_CATALOG_PAGE_MAX_ENTRIES = 1024;
declare const ScriptCatalogOverviewSchema: Schema.Struct<{
  readonly packages: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
    readonly status: Schema.Literal<"valid">;
    readonly compatibility: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"compatible">;
      readonly currentVersion: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"incompatible">;
      readonly currentVersion: Schema.String;
      readonly requiredVersion: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"unknown">;
      readonly currentVersion: Schema.String;
      readonly warning: Schema.optionalKey<Schema.String>;
    }>]>;
    readonly dependencyStatus: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"ready">;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"blocked">;
      readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
        readonly reason: Schema.Literal<"missing">;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly reason: Schema.Literal<"version-mismatch">;
        readonly installedVersion: Schema.String;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly reason: Schema.Literal<"version-unavailable">;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly reason: Schema.Literal<"unavailable">;
        readonly packageName: Schema.String;
        readonly requiredVersion: Schema.String;
      }>]>>;
    }>]>;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly integrity: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
    readonly name: Schema.String;
    readonly path: Schema.String;
    readonly source: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
      readonly resolvedCommit: Schema.String;
      readonly repositoryUrl: Schema.String;
      readonly requestedRef: Schema.optionalKey<Schema.String>;
      readonly credentialId: Schema.optionalKey<Schema.String>;
      readonly kind: Schema.Literal<"repository">;
    }>, Schema.Struct<{
      readonly resolvedCommit: Schema.optionalKey<Schema.String>;
      readonly resolvedTree: Schema.String;
      readonly subdirectory: Schema.String;
      readonly repositoryUrl: Schema.String;
      readonly requestedRef: Schema.optionalKey<Schema.String>;
      readonly credentialId: Schema.optionalKey<Schema.String>;
      readonly kind: Schema.Literal<"directory">;
    }>]>>;
    readonly update: Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"unchecked">;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"current">;
      readonly checkedAt: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"available">;
      readonly checkedAt: Schema.String;
      readonly revision: Schema.Union<readonly [Schema.Struct<{
        readonly kind: Schema.Literal<"commit">;
        readonly sha: Schema.String;
      }>, Schema.Struct<{
        readonly kind: Schema.Literal<"tree">;
        readonly sha: Schema.String;
      }>]>;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"unknown">;
      readonly checkedAt: Schema.optionalKey<Schema.String>;
      readonly message: Schema.String;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"rate-limited">;
      readonly retryAt: Schema.String;
      readonly message: Schema.String;
    }>]>;
    readonly version: Schema.optionalKey<Schema.String>;
    readonly warning: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly status: Schema.Literal<"invalid">;
    readonly diagnostic: Schema.String;
    readonly name: Schema.optionalKey<Schema.String>;
    readonly path: Schema.String;
  }>]>>;
  readonly revision: Schema.String;
  readonly scriptCount: Schema.Int;
}>;
type ScriptCatalogOverview = typeof ScriptCatalogOverviewSchema.Type;
declare const ScriptCatalogPageRequestSchema: Schema.Struct<{
  readonly limit: Schema.Int;
  readonly offset: Schema.Int;
  readonly query: Schema.String;
  readonly revision: Schema.String;
}>;
type ScriptCatalogPageRequest = typeof ScriptCatalogPageRequestSchema.Type;
declare const ScriptCatalogPageSchema: Schema.Struct<{
  readonly entries: Schema.$Array<Schema.Struct<{
    readonly name: Schema.String;
    readonly path: Schema.String;
    readonly relativePath: Schema.String;
    readonly packageName: Schema.optionalKey<Schema.String>;
    readonly reference: Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"loose">;
      readonly path: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"package">;
      readonly packageName: Schema.String;
      readonly path: Schema.String;
    }>]>;
  }>>;
  readonly offset: Schema.Int;
  readonly revision: Schema.String;
  readonly total: Schema.Int;
}>;
type ScriptCatalogPage = typeof ScriptCatalogPageSchema.Type;
declare const ScriptCatalogChangeSchema: Schema.Struct<{
  readonly revision: Schema.String;
}>;
type ScriptCatalogChange = typeof ScriptCatalogChangeSchema.Type;
declare const GitHubCredentialSummarySchema: Schema.Struct<{
  readonly id: Schema.String;
  readonly label: Schema.String;
}>;
type GitHubCredentialSummary = typeof GitHubCredentialSummarySchema.Type;
declare const GitHubCredentialWriteSchema: Schema.Struct<{
  readonly id: Schema.optionalKey<Schema.String>;
  readonly label: Schema.String;
  readonly token: Schema.String;
}>;
type GitHubCredentialWrite = typeof GitHubCredentialWriteSchema.Type;
declare const ScriptPackageInstallRequestSchema: Schema.Struct<{
  readonly credentialId: Schema.optionalKey<Schema.String>;
  readonly ref: Schema.optionalKey<Schema.String>;
  readonly replaceExisting: Schema.optionalKey<Schema.Boolean>;
  readonly repositoryUrl: Schema.String;
  readonly subdirectory: Schema.optionalKey<Schema.String>;
}>;
type ScriptPackageInstallRequest = typeof ScriptPackageInstallRequestSchema.Type;
declare const ScriptPackageUpdateRequestSchema: Schema.Struct<{
  readonly packageName: Schema.String;
  readonly replaceModified: Schema.optionalKey<Schema.Boolean>;
}>;
type ScriptPackageUpdateRequest = typeof ScriptPackageUpdateRequestSchema.Type;
declare const ScriptPackageRestoreRequestSchema: Schema.Struct<{
  readonly packageName: Schema.String;
}>;
type ScriptPackageRestoreRequest = typeof ScriptPackageRestoreRequestSchema.Type;
declare const ScriptPackageRemoveRequestSchema: Schema.Struct<{
  readonly confirmModified: Schema.optionalKey<Schema.Boolean>;
  readonly packageName: Schema.String;
}>;
type ScriptPackageRemoveRequest = typeof ScriptPackageRemoveRequestSchema.Type;
declare const ScriptPackageMutationResultSchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"completed">;
  readonly catalog: Schema.Struct<{
    readonly packages: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"valid">;
      readonly compatibility: Schema.Union<readonly [Schema.Struct<{
        readonly status: Schema.Literal<"compatible">;
        readonly currentVersion: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"incompatible">;
        readonly currentVersion: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"unknown">;
        readonly currentVersion: Schema.String;
        readonly warning: Schema.optionalKey<Schema.String>;
      }>]>;
      readonly dependencyStatus: Schema.Union<readonly [Schema.Struct<{
        readonly status: Schema.Literal<"ready">;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"blocked">;
        readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
          readonly reason: Schema.Literal<"missing">;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>, Schema.Struct<{
          readonly reason: Schema.Literal<"version-mismatch">;
          readonly installedVersion: Schema.String;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>, Schema.Struct<{
          readonly reason: Schema.Literal<"version-unavailable">;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>, Schema.Struct<{
          readonly reason: Schema.Literal<"unavailable">;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>]>>;
      }>]>;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly integrity: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
      readonly name: Schema.String;
      readonly path: Schema.String;
      readonly source: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
        readonly resolvedCommit: Schema.String;
        readonly repositoryUrl: Schema.String;
        readonly requestedRef: Schema.optionalKey<Schema.String>;
        readonly credentialId: Schema.optionalKey<Schema.String>;
        readonly kind: Schema.Literal<"repository">;
      }>, Schema.Struct<{
        readonly resolvedCommit: Schema.optionalKey<Schema.String>;
        readonly resolvedTree: Schema.String;
        readonly subdirectory: Schema.String;
        readonly repositoryUrl: Schema.String;
        readonly requestedRef: Schema.optionalKey<Schema.String>;
        readonly credentialId: Schema.optionalKey<Schema.String>;
        readonly kind: Schema.Literal<"directory">;
      }>]>>;
      readonly update: Schema.Union<readonly [Schema.Struct<{
        readonly status: Schema.Literal<"unchecked">;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"current">;
        readonly checkedAt: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"available">;
        readonly checkedAt: Schema.String;
        readonly revision: Schema.Union<readonly [Schema.Struct<{
          readonly kind: Schema.Literal<"commit">;
          readonly sha: Schema.String;
        }>, Schema.Struct<{
          readonly kind: Schema.Literal<"tree">;
          readonly sha: Schema.String;
        }>]>;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"unknown">;
        readonly checkedAt: Schema.optionalKey<Schema.String>;
        readonly message: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"rate-limited">;
        readonly retryAt: Schema.String;
        readonly message: Schema.String;
      }>]>;
      readonly version: Schema.optionalKey<Schema.String>;
      readonly warning: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"invalid">;
      readonly diagnostic: Schema.String;
      readonly name: Schema.optionalKey<Schema.String>;
      readonly path: Schema.String;
    }>]>>;
    readonly revision: Schema.String;
    readonly scriptCount: Schema.Int;
  }>;
  readonly packageName: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"unchanged">;
  readonly catalog: Schema.Struct<{
    readonly packages: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
      readonly status: Schema.Literal<"valid">;
      readonly compatibility: Schema.Union<readonly [Schema.Struct<{
        readonly status: Schema.Literal<"compatible">;
        readonly currentVersion: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"incompatible">;
        readonly currentVersion: Schema.String;
        readonly requiredVersion: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"unknown">;
        readonly currentVersion: Schema.String;
        readonly warning: Schema.optionalKey<Schema.String>;
      }>]>;
      readonly dependencyStatus: Schema.Union<readonly [Schema.Struct<{
        readonly status: Schema.Literal<"ready">;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"blocked">;
        readonly issues: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
          readonly reason: Schema.Literal<"missing">;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>, Schema.Struct<{
          readonly reason: Schema.Literal<"version-mismatch">;
          readonly installedVersion: Schema.String;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>, Schema.Struct<{
          readonly reason: Schema.Literal<"version-unavailable">;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>, Schema.Struct<{
          readonly reason: Schema.Literal<"unavailable">;
          readonly packageName: Schema.String;
          readonly requiredVersion: Schema.String;
        }>]>>;
      }>]>;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly integrity: Schema.Literals<readonly ["modified", "unmanaged", "verified"]>;
      readonly name: Schema.String;
      readonly path: Schema.String;
      readonly source: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
        readonly resolvedCommit: Schema.String;
        readonly repositoryUrl: Schema.String;
        readonly requestedRef: Schema.optionalKey<Schema.String>;
        readonly credentialId: Schema.optionalKey<Schema.String>;
        readonly kind: Schema.Literal<"repository">;
      }>, Schema.Struct<{
        readonly resolvedCommit: Schema.optionalKey<Schema.String>;
        readonly resolvedTree: Schema.String;
        readonly subdirectory: Schema.String;
        readonly repositoryUrl: Schema.String;
        readonly requestedRef: Schema.optionalKey<Schema.String>;
        readonly credentialId: Schema.optionalKey<Schema.String>;
        readonly kind: Schema.Literal<"directory">;
      }>]>>;
      readonly update: Schema.Union<readonly [Schema.Struct<{
        readonly status: Schema.Literal<"unchecked">;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"current">;
        readonly checkedAt: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"available">;
        readonly checkedAt: Schema.String;
        readonly revision: Schema.Union<readonly [Schema.Struct<{
          readonly kind: Schema.Literal<"commit">;
          readonly sha: Schema.String;
        }>, Schema.Struct<{
          readonly kind: Schema.Literal<"tree">;
          readonly sha: Schema.String;
        }>]>;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"unknown">;
        readonly checkedAt: Schema.optionalKey<Schema.String>;
        readonly message: Schema.String;
      }>, Schema.Struct<{
        readonly status: Schema.Literal<"rate-limited">;
        readonly retryAt: Schema.String;
        readonly message: Schema.String;
      }>]>;
      readonly version: Schema.optionalKey<Schema.String>;
      readonly warning: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly status: Schema.Literal<"invalid">;
      readonly diagnostic: Schema.String;
      readonly name: Schema.optionalKey<Schema.String>;
      readonly path: Schema.String;
    }>]>>;
    readonly revision: Schema.String;
    readonly scriptCount: Schema.Int;
  }>;
  readonly packageName: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"confirmation-required">;
  readonly packageName: Schema.String;
  readonly reason: Schema.Literals<readonly ["existing-package", "local-modifications"]>;
}>]>;
type ScriptPackageMutationResult = typeof ScriptPackageMutationResultSchema.Type;
export { GitHubCredentialSummary, GitHubCredentialSummarySchema, GitHubCredentialWrite, GitHubCredentialWriteSchema, InvalidScriptPackage, InvalidScriptPackageSchema, SCRIPT_BUILTIN_MODULE_SPECIFIERS, SCRIPT_CATALOG_PAGE_MAX_ENTRIES, ScriptBuiltinModuleSpecifier, ScriptCatalog, ScriptCatalogChange, ScriptCatalogChangeSchema, ScriptCatalogEntry, ScriptCatalogEntrySchema, ScriptCatalogOverview, ScriptCatalogOverviewSchema, ScriptCatalogPage, ScriptCatalogPageRequest, ScriptCatalogPageRequestSchema, ScriptCatalogPageSchema, ScriptCatalogSchema, ScriptExecutionPackage, ScriptExecutionPackageSchema, ScriptExecutionSnapshot, ScriptExecutionSnapshotSchema, ScriptModuleImport, ScriptModuleImportSchema, ScriptModuleSource, ScriptModuleSourceSchema, ScriptPackageCompatibility, ScriptPackageCompatibilitySchema, ScriptPackageDependencyIssue, ScriptPackageDependencyIssueSchema, ScriptPackageDependencyStatus, ScriptPackageDependencyStatusSchema, ScriptPackageDirectory, ScriptPackageDirectorySchema, ScriptPackageInstallRequest, ScriptPackageInstallRequestSchema, ScriptPackageIntegrity, ScriptPackageIntegritySchema, ScriptPackageMutationResult, ScriptPackageMutationResultSchema, ScriptPackageName, ScriptPackageNameSchema, ScriptPackageRemoveRequest, ScriptPackageRemoveRequestSchema, ScriptPackageRepositorySubdirectory, ScriptPackageRepositorySubdirectorySchema, ScriptPackageRestoreRequest, ScriptPackageRestoreRequestSchema, ScriptPackageRevision, ScriptPackageRevisionSchema, ScriptPackageSource, ScriptPackageSourceSchema, ScriptPackageSummary, ScriptPackageSummarySchema, ScriptPackageUpdateRequest, ScriptPackageUpdateRequestSchema, ScriptPackageUpdateState, ScriptPackageUpdateStateSchema, ScriptReference, ScriptReferenceSchema, ScriptRelativePathSchema, ValidScriptPackage, ValidScriptPackageSchema, isRelativeScriptModuleSpecifier, isScriptBuiltinModuleSpecifier, isScriptPackageRepositorySubdirectory, resolveRelativeScriptModulePath, scriptModulePathCandidates };