import fs from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import * as NodeRuntime from "@effect/platform-node/NodeRuntime";
import * as NodeServices from "@effect/platform-node/NodeServices";
import * as Console from "effect/Console";
import * as Data from "effect/Data";
import * as Effect from "effect/Effect";
import * as FileSystem from "effect/FileSystem";
import * as Option from "effect/Option";
import { Command, Flag } from "effect/cli";

const DEFAULT_REPO_ROOT = fileURLToPath(new URL("../", import.meta.url));

class BridgeGenerationError extends Data.TaggedError("BridgeGenerationError")<{
  readonly message: string;
  readonly cause?: unknown;
}> {}

type GenerateOptions = {
  repoRoot?: string;
  check?: boolean;
  swfDtsPath?: string;
};

const BRIDGE_TS_RETURN_TYPE_METADATA = "BridgeTsReturnType";
const BRIDGE_TS_PARAM_TYPE_METADATA = "BridgeTsParamType";
const BRIDGE_TS_TYPES_ALIAS = "FlashTypes";
const BRIDGE_TS_TYPES_MODULE = "./Types";

type BridgeParameter = {
  name: string;
  type: string;
  tsType: string | null;
  optional: boolean;
  rest: boolean;
};

type Metadata = {
  name: string;
  arg: string | null;
  line: number;
};

type ParsedMethod = {
  className: string;
  classNamespace: string | null;
  classFqn: string;
  name: string;
  parameters: BridgeParameter[];
  returnType: string;
  isStatic: boolean;
  metadata: Metadata[];
  line: number;
  filePath: string;
};

type BridgeEntry = {
  name: string;
  className: string;
  classFqn: string;
  methodName: string;
  parameters: BridgeParameter[];
  returnType: string;
  tsReturnType: string | null;
  filePath: string;
  line: number;
};

type BridgeModel = {
  exports: BridgeEntry[];
  events: BridgeEntry[];
};

type As3Source = {
  filePath: string;
  source: string;
};

function parseMetadata(line: string, lineNumber: number): Metadata | null {
  const match = line.trim().match(/^\[(\w+)(?:\("([^"]*)"\))?\]$/);
  if (!match) {
    return null;
  }

  const name = match[1];
  if (!name) {
    return null;
  }

  return {
    name,
    arg: match[2] ?? null,
    line: lineNumber,
  };
}

function findMetadata(metadata: Metadata[], name: string): Metadata | null {
  return metadata.find((entry) => entry.name === name) ?? null;
}

function removeBlockComments(value: string): string {
  return value.replace(/\/\*[\s\S]*?\*\//g, "");
}

function splitTopLevel(text: string, separator: string): string[] {
  const parts: string[] = [];
  let current = "";
  let depthParen = 0;
  let depthBrace = 0;
  let depthBracket = 0;
  let quote: "'" | '"' | null = null;

  for (let i = 0; i < text.length; i += 1) {
    const char = text[i];

    if (quote) {
      current += char;
      if (char === quote && text[i - 1] !== "\\") {
        quote = null;
      }
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      current += char;
      continue;
    }

    switch (char) {
      case "(": depthParen += 1; break;
      case ")": depthParen -= 1; break;
      case "{": depthBrace += 1; break;
      case "}": depthBrace -= 1; break;
      case "[": depthBracket += 1; break;
      case "]": depthBracket -= 1; break;
    }

    if (
      char === separator &&
      depthParen === 0 &&
      depthBrace === 0 &&
      depthBracket === 0
    ) {
      parts.push(current.trim());
      current = "";
      continue;
    }

    current += char;
  }

  if (current.trim().length > 0) {
    parts.push(current.trim());
  }

  return parts;
}

function parseParameterList(rawParams: string): BridgeParameter[] {
  const cleanedParams = removeBlockComments(rawParams).trim();
  if (!cleanedParams) {
    return [];
  }

  const chunks = splitTopLevel(cleanedParams, ",");

  return chunks
    .filter(Boolean)
    .map((chunk) => {
      if (chunk.startsWith("...")) {
        const restName = chunk.slice(3).trim();
        return {
          name: restName,
          type: "*",
          tsType: null,
          optional: false,
          rest: true,
        } satisfies BridgeParameter;
      }

      const match = chunk.match(
        /^([A-Za-z_][A-Za-z0-9_]*)\s*(?::\s*([^=]+?))?\s*(?:=\s*(.+))?$/,
      );

      if (!match) {
        throw new Error(`Unable to parse parameter declaration: "${chunk}"`);
      }

      const [, name, type, defaultValue] = match;
      if (!name) {
        throw new Error(`Unable to parse parameter declaration: "${chunk}"`);
      }

      return {
        name,
        type: (type ?? "*").trim(),
        tsType: null,
        optional: defaultValue != null,
        rest: false,
      } satisfies BridgeParameter;
    });
}

function validateTsTypeMetadataValue(
  metadata: Metadata,
  filePath: string,
): string {
  const type = metadata.arg?.trim();
  if (!type) {
    throw new Error(
      `${metadata.name} metadata requires a non-empty value in ${filePath}:${metadata.line}`,
    );
  }

  if (type.includes("import(")) {
    throw new Error(
      `${metadata.name} in ${filePath}:${metadata.line} should not use import() expressions. Use ${BRIDGE_TS_TYPES_ALIAS}.TypeName (for example: ${BRIDGE_TS_TYPES_ALIAS}.TargetInfo | null).`,
    );
  }

  return type;
}

function resolveTsParameterTypes(method: ParsedMethod): Map<string, string> {
  const overrides = new Map<string, string>();

  for (const metadata of method.metadata) {
    if (metadata.name !== BRIDGE_TS_PARAM_TYPE_METADATA) {
      continue;
    }

    const value = validateTsTypeMetadataValue(
      metadata,
      method.filePath,
    );
    const separatorIndex = value.indexOf(":");
    if (separatorIndex <= 0) {
      throw new Error(
        `${BRIDGE_TS_PARAM_TYPE_METADATA} metadata must use "parameterName: Type" in ${method.filePath}:${metadata.line}`,
      );
    }

    const parameterName = value.slice(0, separatorIndex).trim();
    const tsType = value.slice(separatorIndex + 1).trim();
    if (!parameterName || !tsType) {
      throw new Error(
        `${BRIDGE_TS_PARAM_TYPE_METADATA} metadata must use "parameterName: Type" in ${method.filePath}:${metadata.line}`,
      );
    }

    if (!method.parameters.some((parameter) => parameter.name === parameterName)) {
      throw new Error(
        `${BRIDGE_TS_PARAM_TYPE_METADATA} references unknown parameter "${parameterName}" in ${method.filePath}:${metadata.line}`,
      );
    }

    if (overrides.has(parameterName)) {
      throw new Error(
        `Duplicate ${BRIDGE_TS_PARAM_TYPE_METADATA} metadata for parameter "${parameterName}" in ${method.filePath}:${metadata.line}`,
      );
    }

    overrides.set(parameterName, tsType);
  }

  return overrides;
}

function applyTsParameterTypes(method: ParsedMethod): BridgeParameter[] {
  const overrides = resolveTsParameterTypes(method);
  if (overrides.size === 0) {
    return method.parameters;
  }

  return method.parameters.map((parameter) => ({
    ...parameter,
    tsType: overrides.get(parameter.name) ?? parameter.tsType,
  }));
}

function parseAs3File(filePath: string, source: string): ParsedMethod[] {
  const packageMatch = source.match(/package\s+([A-Za-z0-9_.]+)\s*\{/m);
  const packageName = packageMatch?.[1] ?? "";

  const lines = source.split(/\r?\n/);
  const methods: ParsedMethod[] = [];

  let pendingMetadata: Metadata[] = [];
  let className: string | null = null;
  let classNamespace: string | null = null;

  lines.forEach((line, index) => {
    const metadata = parseMetadata(line, index + 1);
    if (metadata) {
      pendingMetadata.push(metadata);
      return;
    }

    const classMatch = line.match(
      /^\s*public\s+class\s+([A-Za-z_][A-Za-z0-9_]*)/,
    );
    if (classMatch) {
      const matchedClassName = classMatch[1];
      if (!matchedClassName) {
        throw new Error(
          `Unable to parse class declaration in ${filePath}:${index + 1}`,
        );
      }

      className = matchedClassName;
      const namespaceMetadata = findMetadata(pendingMetadata, "BridgeNamespace");
      classNamespace = namespaceMetadata?.arg ?? null;
      pendingMetadata = [];
      return;
    }

    const methodMatch = line.match(
      /^\s*public\s+(static\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*(?::\s*([A-Za-z0-9_*]+))?/,
    );

    if (methodMatch) {
      if (!className) {
        throw new Error(
          `Found method before class declaration in ${filePath}:${index + 1}`,
        );
      }

      const methodMetadata = pendingMetadata;
      pendingMetadata = [];

      const methodName = methodMatch[2];
      if (!methodName) {
        throw new Error(
          `Unable to parse method declaration in ${filePath}:${index + 1}`,
        );
      }
      const methodParameters = parseParameterList(methodMatch[3] ?? "");
      const returnType = methodMatch[4] ?? "void";

      methods.push({
        className,
        classNamespace,
        classFqn: `${packageName}.${className}`,
        name: methodName,
        parameters: methodParameters,
        returnType,
        isStatic: Boolean(methodMatch[1]),
        metadata: methodMetadata,
        line: index + 1,
        filePath,
      });

      return;
    }

    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//")) {
      return;
    }

    pendingMetadata = [];
  });

  return methods;
}

function parseBridgeModel(sources: As3Source[]): BridgeModel {
  const exports: BridgeEntry[] = [];
  const events: BridgeEntry[] = [];

  for (const { filePath, source } of sources) {
    for (const method of parseAs3File(filePath, source)) {
      const exportMetadata = findMetadata(method.metadata, "BridgeExport");
      const ignoreMetadata = findMetadata(method.metadata, "BridgeIgnore");
      const eventMetadata = findMetadata(method.metadata, "BridgeEvent");

      const returnMetadata = findMetadata(method.metadata, BRIDGE_TS_RETURN_TYPE_METADATA);
      const tsReturnType = returnMetadata
        ? validateTsTypeMetadataValue(returnMetadata, method.filePath)
        : null;
      const entry = {
        className: method.className,
        classFqn: method.classFqn,
        methodName: method.name,
        parameters: applyTsParameterTypes(method),
        returnType: method.returnType,
        tsReturnType,
        filePath: method.filePath,
        line: method.line,
      };

      if (eventMetadata) {
        if (!eventMetadata.arg) {
          throw new Error(
            `BridgeEvent metadata requires a value in ${method.filePath}:${method.line}`,
          );
        }

        events.push({ ...entry, name: eventMetadata.arg.trim() });
      }

      if (!method.isStatic) {
        if (exportMetadata || ignoreMetadata) {
          throw new Error(
            `BridgeExport/BridgeIgnore can only be used on static methods: ${method.filePath}:${method.line}`,
          );
        }

        continue;
      }

      if (!method.classNamespace) {
        if (exportMetadata || ignoreMetadata) {
          throw new Error(
            `Class missing BridgeNamespace for exported method in ${method.filePath}:${method.line}`,
          );
        }
        continue;
      }

      if (exportMetadata && ignoreMetadata) {
        throw new Error(
          `Method cannot have both BridgeExport and BridgeIgnore: ${method.filePath}:${method.line}`,
        );
      }

      if (!exportMetadata && !ignoreMetadata) {
        throw new Error(
          `Missing BridgeExport/BridgeIgnore metadata for static method ${method.name} in ${method.filePath}:${method.line}`,
        );
      }

      if (ignoreMetadata) {
        continue;
      }

      const exportedMethodName = exportMetadata?.arg?.trim() || method.name;
      exports.push({ ...entry, name: `${method.classNamespace}.${exportedMethodName}` });
    }
  }

  for (const [kind, entries] of [["export", exports], ["event", events]] as const) {
    const byName = new Map<string, BridgeEntry>();
    for (const entry of entries) {
      const existing = byName.get(entry.name);
      if (existing) {
        throw new Error([
          `Duplicate bridge ${kind} name "${entry.name}"`,
          `- ${existing.filePath}:${existing.line}`,
          `- ${entry.filePath}:${entry.line}`,
        ].join("\n"));
      }
      byName.set(entry.name, entry);
    }
    entries.sort((a, b) => a.name.localeCompare(b.name));
  }

  return { exports, events };
}

function mapAs3Type(type: string): string {
  const normalized = type.trim();

  switch (normalized) {
    case "void":
      return "void";
    case "String":
      return "string";
    case "Boolean":
      return "boolean";
    case "int":
    case "uint":
    case "Number":
      return "number";
    case "Array":
      return "unknown[]";
    case "Object":
      return "Record<string, unknown>";
    case "Function":
      return "(...args: unknown[]) => unknown";
    case "*":
      return "unknown";
    default:
      return "unknown";
  }
}

function formatFunctionParameter(parameter: BridgeParameter): string {
  if (parameter.rest) {
    const baseType = parameter.tsType ?? mapAs3Type(parameter.type);
    const restType = baseType.endsWith("[]") ? baseType : `${baseType}[]`;
    return `...${parameter.name}: ${restType}`;
  }

  const suffix = parameter.optional ? "?" : "";
  return `${parameter.name}${suffix}: ${parameter.tsType ?? mapAs3Type(parameter.type)}`;
}

function formatFunctionProperty(
  name: string,
  parameters: BridgeParameter[],
  returnType: string,
  indent: string,
  optional = false,
  quoteName = true,
): string[] {
  const canUseIdentifier = /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name);
  const renderedName = quoteName || !canUseIdentifier ? JSON.stringify(name) : name;
  const propertyName = `${renderedName}${optional ? "?" : ""}`;
  const renderedParameters = parameters.map(formatFunctionParameter);
  const singleLine = `${indent}${propertyName}: (${renderedParameters.join(", ")}) => ${returnType};`;
  if (singleLine.length <= 80 || parameters.length === 0) {
    return [singleLine];
  }

  const parameterIndent = `${indent}  `;
  return [
    `${indent}${propertyName}: (`,
    ...renderedParameters.map((parameter) => `${parameterIndent}${parameter},`),
    `${indent}) => ${returnType};`,
  ];
}

function resolveRenderedReturnType(
  as3ReturnType: string,
  tsReturnType: string | null,
): string {
  if (tsReturnType !== null) {
    return tsReturnType;
  }

  if (as3ReturnType.trim() === "Object") {
    return "Record<string, unknown> | null";
  }

  return mapAs3Type(as3ReturnType);
}

function shouldImportFlashTypes(model: BridgeModel): boolean {
  const usesAlias = (tsReturnType: string | null) =>
    tsReturnType?.includes(`${BRIDGE_TS_TYPES_ALIAS}.`) ?? false;
  return [...model.exports, ...model.events].some((entry) =>
    usesAlias(entry.tsReturnType) || entry.parameters.some((parameter) => usesAlias(parameter.tsType)),
  );
}

function renderAs3Registry(model: BridgeModel): string {
  const importSet = new Set<string>();
  for (const entry of model.exports) {
    importSet.add(entry.classFqn);
  }

  const imports = [...importSet].sort((a, b) => a.localeCompare(b));

  const lines: string[] = [
    "// AUTO-GENERATED FILE. DO NOT EDIT.",
    "package lucent.generated",
    "{",
    "  import lucent.Externalizer;",
  ];

  for (const importLine of imports) {
    lines.push(`  import ${importLine};`);
  }

  lines.push(
    "",
    "  public class BridgeRegistryGenerated",
    "  {",
    "    public static function register(external:Externalizer):void",
    "    {",
  );

  for (const entry of model.exports) {
    lines.push(
      `      external.externalize("${entry.name}", ${entry.className}.${entry.methodName});`,
    );
  }

  lines.push("    }", "  }", "}", "");
  return lines.join("\n");
}

function renderWindowSwfDts(model: BridgeModel): string {
  const lines: string[] = [
    "// AUTO-GENERATED FILE. DO NOT EDIT.",
    "",
  ];

  if (shouldImportFlashTypes(model)) {
    lines.push(
      `import type * as ${BRIDGE_TS_TYPES_ALIAS} from "${BRIDGE_TS_TYPES_MODULE}";`,
      "",
    );
  }
  else {
    lines.push("export {};", "");
  }

  lines.push("declare global {", "  interface Window {", "    swf: {");

  for (const entry of model.exports) {
    lines.push(
      ...formatFunctionProperty(
        entry.name,
        entry.parameters,
        resolveRenderedReturnType(entry.returnType, entry.tsReturnType),
        "      ",
      ),
    );
  }

  lines.push("    };");

  for (const entry of model.events) {
    lines.push(
      ...formatFunctionProperty(
        entry.name,
        entry.parameters,
        resolveRenderedReturnType(entry.returnType, entry.tsReturnType),
        "    ",
        true,
        false,
      ),
    );
  }

  lines.push("  }", "}", "");

  return lines.join("\n");
}

const writeOrCheckFile = Effect.fn("writeOrCheckFile")(function* (
  filePath: string,
  content: string,
  check: boolean,
  repoRoot: string,
) {
  const fs = yield* FileSystem.FileSystem;
  const existing = yield* fs.readFileString(filePath).pipe(
    Effect.catchIf(
      (error) => !check && error.reason._tag === "NotFound",
      () => Effect.succeed(null),
    ),
    Effect.mapError((cause) => check
      ? new BridgeGenerationError({
        message: `Unable to read generated file: ${relative(repoRoot, filePath)}`,
        cause,
      })
      : cause),
  );

  if (existing === content) {
    return;
  }
  if (check) {
    return yield* new BridgeGenerationError({
      message: `Generated file is out of date: ${relative(repoRoot, filePath)}`,
    });
  }

  yield* fs.makeDirectory(dirname(filePath), { recursive: true });
  yield* fs.writeFileString(filePath, content);
});

export const generateBridgeArtifacts = Effect.fn("generateBridgeArtifacts")(function* (
  options: GenerateOptions = {},
) {
  const fileSystem = yield* FileSystem.FileSystem;
  const repoRoot = resolve(options.repoRoot ?? DEFAULT_REPO_ROOT);
  const as3Root = join(repoRoot, "as3", "src", "lucent");
  const as3OutputPath = join(as3Root, "generated", "BridgeRegistryGenerated.as");
  const swfDtsOutputPath = options.swfDtsPath ? resolve(options.swfDtsPath) : null;
  const entries = yield* Effect.tryPromise({
    try: () => fs.readdir(as3Root, { recursive: true, withFileTypes: true }),
    catch: (cause) => new BridgeGenerationError({
      message: `Unable to list AS3 sources: ${relative(repoRoot, as3Root)}`,
      cause,
    }),
  });
  const files = entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".as"))
    .map((entry) => join(entry.parentPath, entry.name))
    .filter((filePath) => filePath !== as3OutputPath)
    .sort((a, b) => a.localeCompare(b));
  const sources = yield* Effect.forEach(files, (filePath) =>
    fileSystem.readFileString(filePath).pipe(
      Effect.map((source) => ({ filePath: relative(repoRoot, filePath), source })),
    ),
  );
  const model = yield* Effect.try({
    try: () => parseBridgeModel(sources),
    catch: (cause) => new BridgeGenerationError({
      message: cause instanceof Error ? cause.message : String(cause),
      cause,
    }),
  });
  const as3Registry = renderAs3Registry(model);
  const swfDts = renderWindowSwfDts(model);
  const check = options.check ?? false;

  yield* writeOrCheckFile(as3OutputPath, as3Registry, check, repoRoot);
  if (swfDtsOutputPath !== null) {
    yield* writeOrCheckFile(swfDtsOutputPath, swfDts, check, repoRoot);
  }

  return { model, as3Registry, swfDts, as3OutputPath, swfDtsOutputPath };
});

const command = Command.make("gen-as3-bridge", {
  check: Flag.Boolean("check").pipe(
    Flag.withDescription("Check generated artifacts without writing files"),
    Flag.withDefault(false),
  ),
  repoRoot: Flag.Directory("repo-root", { mustExist: true }).pipe(
    Flag.withDescription("Repository root containing as3/src/lucent"),
    Flag.withDefault(DEFAULT_REPO_ROOT),
  ),
  swfDtsPath: Flag.String("swf-dts").pipe(
    Flag.withDescription("Also generate Window.swf and event declarations at this path"),
    Flag.optional,
  ),
}, ({ check, repoRoot, swfDtsPath }) => Effect.gen(function* () {
  const result = yield* generateBridgeArtifacts({
    check,
    repoRoot,
    ...(Option.isSome(swfDtsPath) ? { swfDtsPath: swfDtsPath.value } : {}),
  });
  if (check) {
    yield* Console.log("Bridge artifacts are up to date.");
    return;
  }
  yield* Console.log([
    `Generated ${relative(repoRoot, result.as3OutputPath)}`,
    ...(result.swfDtsOutputPath ? [`Generated ${relative(repoRoot, result.swfDtsOutputPath)}`] : []),
    `Exports: ${result.model.exports.length}`,
    `Events: ${result.model.events.length}`,
  ].join("\n"));
})).pipe(
  Command.withDescription("Generate bridge registrations and TypeScript declarations from AS3 metadata"),
);

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  Command.run(command, { version: "1.0.0" }).pipe(
    Effect.catchTag("BridgeGenerationError", (error) => Console.error(error.message).pipe(
      Effect.andThen(Effect.sync(() => { process.exitCode = 1; })),
    )),
    Effect.provide(NodeServices.layer),
    NodeRuntime.runMain,
  );
}
