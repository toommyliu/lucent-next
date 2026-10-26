import * as Schema from "effect/Schema";
declare const ScriptInputValueSchema: Schema.Union<readonly [Schema.String, Schema.Number, Schema.Boolean, Schema.$Array<Schema.String>]>;
type ScriptInputValue = typeof ScriptInputValueSchema.Type;
declare const ScriptInputTypeSchema: Schema.Literals<readonly ["string", "number", "boolean", "select", "multi-select"]>;
type ScriptInputType = typeof ScriptInputTypeSchema.Type;
declare const ScriptStringInputFieldSchema: Schema.Struct<{
  readonly type: Schema.Literal<"string">;
  readonly default: Schema.optionalKey<Schema.String>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>;
declare const ScriptNumberInputFieldSchema: Schema.Struct<{
  readonly type: Schema.Literal<"number">;
  readonly default: Schema.optionalKey<Schema.Number>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>;
declare const ScriptBooleanInputFieldSchema: Schema.Struct<{
  readonly type: Schema.Literal<"boolean">;
  readonly default: Schema.optionalKey<Schema.Boolean>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>;
declare const ScriptSelectInputFieldSchema: Schema.Struct<{
  readonly type: Schema.Literal<"select">;
  readonly options: Schema.$Array<Schema.String>;
  readonly default: Schema.optionalKey<Schema.String>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>;
declare const ScriptMultiSelectInputFieldSchema: Schema.Struct<{
  readonly type: Schema.Literal<"multi-select">;
  readonly options: Schema.$Array<Schema.String>;
  readonly default: Schema.optionalKey<Schema.$Array<Schema.String>>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>;
declare const ScriptInputFieldSchema: Schema.Union<readonly [Schema.Struct<{
  readonly type: Schema.Literal<"string">;
  readonly default: Schema.optionalKey<Schema.String>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>, Schema.Struct<{
  readonly type: Schema.Literal<"number">;
  readonly default: Schema.optionalKey<Schema.Number>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>, Schema.Struct<{
  readonly type: Schema.Literal<"boolean">;
  readonly default: Schema.optionalKey<Schema.Boolean>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>, Schema.Struct<{
  readonly type: Schema.Literal<"select">;
  readonly options: Schema.$Array<Schema.String>;
  readonly default: Schema.optionalKey<Schema.String>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>, Schema.Struct<{
  readonly type: Schema.Literal<"multi-select">;
  readonly options: Schema.$Array<Schema.String>;
  readonly default: Schema.optionalKey<Schema.$Array<Schema.String>>;
  readonly key: Schema.String;
  readonly label: Schema.String;
  readonly description: Schema.optionalKey<Schema.String>;
  readonly required: Schema.optionalKey<Schema.Boolean>;
}>]>;
type ScriptInputField = typeof ScriptInputFieldSchema.Type;
declare const ScriptInputsDefinitionSchema: Schema.Struct<{
  readonly id: Schema.String;
  readonly fields: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
    readonly type: Schema.Literal<"string">;
    readonly default: Schema.optionalKey<Schema.String>;
    readonly key: Schema.String;
    readonly label: Schema.String;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly required: Schema.optionalKey<Schema.Boolean>;
  }>, Schema.Struct<{
    readonly type: Schema.Literal<"number">;
    readonly default: Schema.optionalKey<Schema.Number>;
    readonly key: Schema.String;
    readonly label: Schema.String;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly required: Schema.optionalKey<Schema.Boolean>;
  }>, Schema.Struct<{
    readonly type: Schema.Literal<"boolean">;
    readonly default: Schema.optionalKey<Schema.Boolean>;
    readonly key: Schema.String;
    readonly label: Schema.String;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly required: Schema.optionalKey<Schema.Boolean>;
  }>, Schema.Struct<{
    readonly type: Schema.Literal<"select">;
    readonly options: Schema.$Array<Schema.String>;
    readonly default: Schema.optionalKey<Schema.String>;
    readonly key: Schema.String;
    readonly label: Schema.String;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly required: Schema.optionalKey<Schema.Boolean>;
  }>, Schema.Struct<{
    readonly type: Schema.Literal<"multi-select">;
    readonly options: Schema.$Array<Schema.String>;
    readonly default: Schema.optionalKey<Schema.$Array<Schema.String>>;
    readonly key: Schema.String;
    readonly label: Schema.String;
    readonly description: Schema.optionalKey<Schema.String>;
    readonly required: Schema.optionalKey<Schema.Boolean>;
  }>]>>;
}>;
type ScriptInputsDefinition = typeof ScriptInputsDefinitionSchema.Type;
declare const ScriptFileReferenceSchema: Schema.Struct<{
  readonly path: Schema.String;
  readonly name: Schema.String;
  readonly reference: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"loose">;
    readonly path: Schema.String;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"package">;
    readonly packageName: Schema.String;
    readonly path: Schema.String;
  }>]>>;
}>;
type ScriptFileReference = typeof ScriptFileReferenceSchema.Type;
declare const ScriptFileSchema: Schema.Struct<{
  readonly path: Schema.String;
  readonly name: Schema.String;
  readonly source: Schema.String;
  readonly inputs: Schema.NullOr<Schema.Struct<{
    readonly id: Schema.String;
    readonly fields: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
      readonly type: Schema.Literal<"string">;
      readonly default: Schema.optionalKey<Schema.String>;
      readonly key: Schema.String;
      readonly label: Schema.String;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly required: Schema.optionalKey<Schema.Boolean>;
    }>, Schema.Struct<{
      readonly type: Schema.Literal<"number">;
      readonly default: Schema.optionalKey<Schema.Number>;
      readonly key: Schema.String;
      readonly label: Schema.String;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly required: Schema.optionalKey<Schema.Boolean>;
    }>, Schema.Struct<{
      readonly type: Schema.Literal<"boolean">;
      readonly default: Schema.optionalKey<Schema.Boolean>;
      readonly key: Schema.String;
      readonly label: Schema.String;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly required: Schema.optionalKey<Schema.Boolean>;
    }>, Schema.Struct<{
      readonly type: Schema.Literal<"select">;
      readonly options: Schema.$Array<Schema.String>;
      readonly default: Schema.optionalKey<Schema.String>;
      readonly key: Schema.String;
      readonly label: Schema.String;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly required: Schema.optionalKey<Schema.Boolean>;
    }>, Schema.Struct<{
      readonly type: Schema.Literal<"multi-select">;
      readonly options: Schema.$Array<Schema.String>;
      readonly default: Schema.optionalKey<Schema.$Array<Schema.String>>;
      readonly key: Schema.String;
      readonly label: Schema.String;
      readonly description: Schema.optionalKey<Schema.String>;
      readonly required: Schema.optionalKey<Schema.Boolean>;
    }>]>>;
  }>>;
  readonly revision: Schema.String;
  readonly reference: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"loose">;
    readonly path: Schema.String;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"package">;
    readonly packageName: Schema.String;
    readonly path: Schema.String;
  }>]>>;
  readonly snapshot: Schema.optionalKey<Schema.Struct<{
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
  }>>;
}>;
type ScriptFile = typeof ScriptFileSchema.Type;
declare const ScriptOpenFileResultSchema: Schema.Union<readonly [Schema.Struct<{
  readonly canceled: Schema.Literal<true>;
}>, Schema.Struct<{
  readonly canceled: Schema.Literal<false>;
  readonly file: Schema.Struct<{
    readonly path: Schema.String;
    readonly name: Schema.String;
    readonly source: Schema.String;
    readonly inputs: Schema.NullOr<Schema.Struct<{
      readonly id: Schema.String;
      readonly fields: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
        readonly type: Schema.Literal<"string">;
        readonly default: Schema.optionalKey<Schema.String>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"number">;
        readonly default: Schema.optionalKey<Schema.Number>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"boolean">;
        readonly default: Schema.optionalKey<Schema.Boolean>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"select">;
        readonly options: Schema.$Array<Schema.String>;
        readonly default: Schema.optionalKey<Schema.String>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"multi-select">;
        readonly options: Schema.$Array<Schema.String>;
        readonly default: Schema.optionalKey<Schema.$Array<Schema.String>>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>]>>;
    }>>;
    readonly revision: Schema.String;
    readonly reference: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"loose">;
      readonly path: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"package">;
      readonly packageName: Schema.String;
      readonly path: Schema.String;
    }>]>>;
    readonly snapshot: Schema.optionalKey<Schema.Struct<{
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
    }>>;
  }>;
}>]>;
type ScriptOpenFileResult = typeof ScriptOpenFileResultSchema.Type;
declare const ScriptSelectFileResultSchema: Schema.Union<readonly [Schema.Struct<{
  readonly canceled: Schema.Literal<true>;
}>, Schema.Struct<{
  readonly canceled: Schema.Literal<false>;
  readonly file: Schema.Struct<{
    readonly path: Schema.String;
    readonly name: Schema.String;
    readonly reference: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"loose">;
      readonly path: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"package">;
      readonly packageName: Schema.String;
      readonly path: Schema.String;
    }>]>>;
  }>;
}>]>;
type ScriptSelectFileResult = typeof ScriptSelectFileResultSchema.Type;
declare const ScriptFileResolutionSchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"found">;
  readonly file: Schema.Struct<{
    readonly path: Schema.String;
    readonly name: Schema.String;
    readonly source: Schema.String;
    readonly inputs: Schema.NullOr<Schema.Struct<{
      readonly id: Schema.String;
      readonly fields: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
        readonly type: Schema.Literal<"string">;
        readonly default: Schema.optionalKey<Schema.String>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"number">;
        readonly default: Schema.optionalKey<Schema.Number>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"boolean">;
        readonly default: Schema.optionalKey<Schema.Boolean>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"select">;
        readonly options: Schema.$Array<Schema.String>;
        readonly default: Schema.optionalKey<Schema.String>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>, Schema.Struct<{
        readonly type: Schema.Literal<"multi-select">;
        readonly options: Schema.$Array<Schema.String>;
        readonly default: Schema.optionalKey<Schema.$Array<Schema.String>>;
        readonly key: Schema.String;
        readonly label: Schema.String;
        readonly description: Schema.optionalKey<Schema.String>;
        readonly required: Schema.optionalKey<Schema.Boolean>;
      }>]>>;
    }>>;
    readonly revision: Schema.String;
    readonly reference: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
      readonly kind: Schema.Literal<"loose">;
      readonly path: Schema.String;
    }>, Schema.Struct<{
      readonly kind: Schema.Literal<"package">;
      readonly packageName: Schema.String;
      readonly path: Schema.String;
    }>]>>;
    readonly snapshot: Schema.optionalKey<Schema.Struct<{
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
    }>>;
  }>;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"missing">;
  readonly path: Schema.String;
}>, Schema.Struct<{
  readonly status: Schema.Literal<"failed">;
  readonly path: Schema.String;
  readonly message: Schema.String;
  readonly detailsText: Schema.optionalKey<Schema.String>;
}>]>;
type ScriptFileResolution = typeof ScriptFileResolutionSchema.Type;
declare const ScriptInputValuesSchema: Schema.$Record<Schema.String, Schema.Union<readonly [Schema.String, Schema.Number, Schema.Boolean, Schema.$Array<Schema.String>]>>;
type ScriptInputValues = typeof ScriptInputValuesSchema.Type;
type ScriptInputValidationResult = {
  readonly status: "ok";
  readonly values: ScriptInputValues;
} | {
  readonly fieldKeys: readonly string[];
  readonly status: "missing-required";
  readonly values: ScriptInputValues;
};
declare const defaultScriptInputValue: (field: ScriptInputField) => ScriptInputValue | undefined;
declare const normalizeScriptInputValues: (definition: ScriptInputsDefinition, rawValues?: Record<string, unknown>) => ScriptInputValues;
declare const findMissingRequiredScriptInputs: (definition: ScriptInputsDefinition, values: ScriptInputValues) => readonly string[];
declare const validateScriptInputValues: (definition: ScriptInputsDefinition, values: Record<string, unknown>) => ScriptInputValidationResult;
export { ScriptBooleanInputFieldSchema, ScriptFile, ScriptFileReference, ScriptFileReferenceSchema, ScriptFileResolution, ScriptFileResolutionSchema, ScriptFileSchema, ScriptInputField, ScriptInputFieldSchema, ScriptInputType, ScriptInputTypeSchema, ScriptInputValidationResult, ScriptInputValue, ScriptInputValueSchema, ScriptInputValues, ScriptInputValuesSchema, ScriptInputsDefinition, ScriptInputsDefinitionSchema, ScriptMultiSelectInputFieldSchema, ScriptNumberInputFieldSchema, ScriptOpenFileResult, ScriptOpenFileResultSchema, ScriptSelectFileResult, ScriptSelectFileResultSchema, ScriptSelectInputFieldSchema, ScriptStringInputFieldSchema, defaultScriptInputValue, findMissingRequiredScriptInputs, normalizeScriptInputValues, validateScriptInputValues };