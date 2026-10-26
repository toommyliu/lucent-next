import * as Schema from "effect/Schema";
type ArmyConfigRaw = Record<string, unknown>;
/**
 * Equipment is applied in this order: `safeClass`, `safePot`, `class`,
 * `safePot`, `weapon`, `cape`, `helm`, `armor`, `pet`, each entry in `pots`,
 * then `scroll`.
 */
interface ArmyEquipSet {
  readonly armor?: string;
  readonly cape?: string;
  readonly class?: string;
  readonly helm?: string;
  readonly pet?: string;
  readonly pots?: readonly string[];
  readonly safeClass?: string;
  readonly safePot?: string;
  readonly scroll?: string;
  readonly weapon?: string;
}
interface ArmySetConfig {
  readonly default?: ArmyEquipSet;
  readonly players: Readonly<Record<string, ArmyEquipSet>>;
}
interface ArmyConfigCore {
  readonly items: Readonly<Record<string, string>>;
  readonly players: readonly string[];
  readonly room: number;
  readonly sets: Readonly<Record<string, ArmySetConfig>>;
}
interface ArmyConfigPayload extends ArmyConfigCore {
  readonly configName: string;
  readonly raw: ArmyConfigRaw;
}
interface ArmySessionPayload extends ArmyConfigPayload {
  readonly playerName: string;
  readonly playerNumber: number;
  readonly role: "leader" | "member";
  readonly sessionId: string;
}
interface ArmyStartPayload {
  readonly configName: string;
  readonly playerName: string;
}
interface ArmySyncPayload {
  readonly label?: string;
  readonly sessionId: string;
  readonly step: number;
  readonly timeoutMs?: number;
}
interface ArmyProgressPayload extends ArmySyncPayload {
  readonly complete: boolean;
}
interface ArmyProgressResult {
  readonly complete: boolean;
  readonly completedPlayers: readonly string[];
  readonly pendingPlayers: readonly string[];
}
interface ArmyFailPayload {
  readonly reason: string;
  readonly sessionId: string;
}
interface ArmySessionEndedPayload {
  readonly reason: string;
  readonly sessionId: string;
}
/** Selects the encounter event that starts each taunt attempt. */
type ArmyLoopTauntStrategy = {
  /** Cast after the target's Focus aura disappears. */
  readonly type: "focus";
} | {
  /** Case-insensitive text contained in the target's animation or aura message. */
  readonly message: string;
  readonly type: "message";
};
interface ArmyLoopTauntResolvedTarget {
  readonly focusActive: boolean;
  readonly lifeRevision: number;
  readonly monsterMapId: number;
  readonly state: "alive" | "dead";
}
interface ArmyLoopTauntRegistrationAssignment {
  readonly assignmentId: number;
  readonly players: readonly number[];
  readonly strategy: ArmyLoopTauntStrategy;
  readonly target: ArmyLoopTauntResolvedTarget;
}
interface ArmyLoopTauntRegistrationPriorityGroup {
  readonly assignments: readonly ArmyLoopTauntRegistrationAssignment[];
}
interface ArmyLoopTauntMapIdentity {
  readonly id: number;
  readonly name: string;
  readonly roomNumber: number;
}
interface ArmyLoopTauntRegisterPayload {
  readonly map: ArmyLoopTauntMapIdentity;
  readonly priorityGroups: readonly ArmyLoopTauntRegistrationPriorityGroup[];
  readonly sessionId: string;
}
interface ArmyLoopTauntRegisterResult {
  readonly runId: string;
}
interface ArmyLoopTauntRunPayload {
  readonly runId: string;
  readonly sessionId: string;
}
interface ArmyLoopTauntLeavePayload extends ArmyLoopTauntRunPayload {
  readonly reason?: string;
}
type ArmyLoopTauntTerminalResult = {
  readonly status: "completed";
} | {
  readonly reason: string;
  readonly status: "failed";
};
type ArmyLoopTauntReport = {
  readonly alive: boolean;
  readonly cooldownMs: number | null;
  readonly type: "participant-state";
  readonly usable: boolean;
} | {
  readonly assignmentId: number;
  readonly focusActive?: boolean;
  readonly lifeRevision: number;
  readonly monsterMapId: number;
  readonly state: "alive" | "dead";
  readonly type: "target-state";
} | {
  readonly active: boolean;
  readonly assignmentId: number;
  readonly lifeRevision: number;
  readonly monsterMapId: number;
  readonly type: "focus-state";
} | {
  readonly assignmentId: number;
  readonly lifeRevision: number;
  readonly message: string;
  readonly monsterMapId: number;
  readonly type: "message";
} | {
  readonly commandId: number;
  readonly cooldownMs?: number | null;
  readonly outcome: "confirmed" | "target-unavailable" | "cast-failed" | "not-ready" | "skipped";
  readonly type: "command-result";
} | {
  readonly reason: string;
  readonly type: "failed";
};
interface ArmyLoopTauntReportPayload extends ArmyLoopTauntRunPayload {
  readonly report: ArmyLoopTauntReport;
}
type ArmyLoopTauntCommand = {
  readonly assignmentId: number;
  readonly expiresAt: number;
  readonly lifeRevision: number;
  readonly monsterMapId: number;
  readonly type: "taunt";
};
interface ArmyLoopTauntCommandPayload extends ArmyLoopTauntRunPayload {
  readonly command: ArmyLoopTauntCommand;
  readonly commandId: number;
}
declare const ArmyConfigRawSchema: Schema.$Record<Schema.String, Schema.Unknown>;
declare const ArmyEquipSetSchema: Schema.Struct<{
  readonly armor: Schema.optionalKey<Schema.String>;
  readonly cape: Schema.optionalKey<Schema.String>;
  readonly class: Schema.optionalKey<Schema.String>;
  readonly helm: Schema.optionalKey<Schema.String>;
  readonly pet: Schema.optionalKey<Schema.String>;
  readonly pots: Schema.optionalKey<Schema.$Array<Schema.String>>;
  readonly safeClass: Schema.optionalKey<Schema.String>;
  readonly safePot: Schema.optionalKey<Schema.String>;
  readonly scroll: Schema.optionalKey<Schema.String>;
  readonly weapon: Schema.optionalKey<Schema.String>;
}>;
declare const ArmySetConfigSchema: Schema.Struct<{
  readonly default: Schema.optionalKey<Schema.Struct<{
    readonly armor: Schema.optionalKey<Schema.String>;
    readonly cape: Schema.optionalKey<Schema.String>;
    readonly class: Schema.optionalKey<Schema.String>;
    readonly helm: Schema.optionalKey<Schema.String>;
    readonly pet: Schema.optionalKey<Schema.String>;
    readonly pots: Schema.optionalKey<Schema.$Array<Schema.String>>;
    readonly safeClass: Schema.optionalKey<Schema.String>;
    readonly safePot: Schema.optionalKey<Schema.String>;
    readonly scroll: Schema.optionalKey<Schema.String>;
    readonly weapon: Schema.optionalKey<Schema.String>;
  }>>;
  readonly players: Schema.$Record<Schema.String, Schema.Struct<{
    readonly armor: Schema.optionalKey<Schema.String>;
    readonly cape: Schema.optionalKey<Schema.String>;
    readonly class: Schema.optionalKey<Schema.String>;
    readonly helm: Schema.optionalKey<Schema.String>;
    readonly pet: Schema.optionalKey<Schema.String>;
    readonly pots: Schema.optionalKey<Schema.$Array<Schema.String>>;
    readonly safeClass: Schema.optionalKey<Schema.String>;
    readonly safePot: Schema.optionalKey<Schema.String>;
    readonly scroll: Schema.optionalKey<Schema.String>;
    readonly weapon: Schema.optionalKey<Schema.String>;
  }>>;
}>;
declare const ArmySessionPayloadSchema: Schema.Struct<{
  readonly playerName: Schema.String;
  readonly playerNumber: Schema.Int;
  readonly role: Schema.Literals<readonly ["leader", "member"]>;
  readonly sessionId: Schema.String;
  readonly configName: Schema.String;
  readonly items: Schema.$Record<Schema.String, Schema.String>;
  readonly players: Schema.$Array<Schema.String>;
  readonly raw: Schema.$Record<Schema.String, Schema.Unknown>;
  readonly room: Schema.Int;
  readonly sets: Schema.$Record<Schema.String, Schema.Struct<{
    readonly default: Schema.optionalKey<Schema.Struct<{
      readonly armor: Schema.optionalKey<Schema.String>;
      readonly cape: Schema.optionalKey<Schema.String>;
      readonly class: Schema.optionalKey<Schema.String>;
      readonly helm: Schema.optionalKey<Schema.String>;
      readonly pet: Schema.optionalKey<Schema.String>;
      readonly pots: Schema.optionalKey<Schema.$Array<Schema.String>>;
      readonly safeClass: Schema.optionalKey<Schema.String>;
      readonly safePot: Schema.optionalKey<Schema.String>;
      readonly scroll: Schema.optionalKey<Schema.String>;
      readonly weapon: Schema.optionalKey<Schema.String>;
    }>>;
    readonly players: Schema.$Record<Schema.String, Schema.Struct<{
      readonly armor: Schema.optionalKey<Schema.String>;
      readonly cape: Schema.optionalKey<Schema.String>;
      readonly class: Schema.optionalKey<Schema.String>;
      readonly helm: Schema.optionalKey<Schema.String>;
      readonly pet: Schema.optionalKey<Schema.String>;
      readonly pots: Schema.optionalKey<Schema.$Array<Schema.String>>;
      readonly safeClass: Schema.optionalKey<Schema.String>;
      readonly safePot: Schema.optionalKey<Schema.String>;
      readonly scroll: Schema.optionalKey<Schema.String>;
      readonly weapon: Schema.optionalKey<Schema.String>;
    }>>;
  }>>;
}>;
declare const ArmySessionEndedPayloadSchema: Schema.Struct<{
  readonly reason: Schema.String;
  readonly sessionId: Schema.String;
}>;
declare const ArmyLoopTauntStrategySchema: Schema.Union<readonly [Schema.Struct<{
  readonly type: Schema.Literal<"focus">;
}>, Schema.Struct<{
  readonly message: Schema.String;
  readonly type: Schema.Literal<"message">;
}>]>;
declare const ArmyLoopTauntResolvedTargetSchema: Schema.Struct<{
  readonly focusActive: Schema.Boolean;
  readonly lifeRevision: Schema.Int;
  readonly monsterMapId: Schema.Int;
  readonly state: Schema.Literals<readonly ["alive", "dead"]>;
}>;
declare const ArmyLoopTauntRegistrationAssignmentSchema: Schema.Struct<{
  readonly assignmentId: Schema.Int;
  readonly players: Schema.$Array<Schema.Int>;
  readonly strategy: Schema.Union<readonly [Schema.Struct<{
    readonly type: Schema.Literal<"focus">;
  }>, Schema.Struct<{
    readonly message: Schema.String;
    readonly type: Schema.Literal<"message">;
  }>]>;
  readonly target: Schema.Struct<{
    readonly focusActive: Schema.Boolean;
    readonly lifeRevision: Schema.Int;
    readonly monsterMapId: Schema.Int;
    readonly state: Schema.Literals<readonly ["alive", "dead"]>;
  }>;
}>;
declare const ArmyLoopTauntRegistrationPriorityGroupSchema: Schema.Struct<{
  readonly assignments: Schema.$Array<Schema.Struct<{
    readonly assignmentId: Schema.Int;
    readonly players: Schema.$Array<Schema.Int>;
    readonly strategy: Schema.Union<readonly [Schema.Struct<{
      readonly type: Schema.Literal<"focus">;
    }>, Schema.Struct<{
      readonly message: Schema.String;
      readonly type: Schema.Literal<"message">;
    }>]>;
    readonly target: Schema.Struct<{
      readonly focusActive: Schema.Boolean;
      readonly lifeRevision: Schema.Int;
      readonly monsterMapId: Schema.Int;
      readonly state: Schema.Literals<readonly ["alive", "dead"]>;
    }>;
  }>>;
}>;
declare const ArmyLoopTauntMapIdentitySchema: Schema.Struct<{
  readonly id: Schema.Int;
  readonly name: Schema.String;
  readonly roomNumber: Schema.Int;
}>;
declare const ArmyLoopTauntRegisterPayloadSchema: Schema.Struct<{
  readonly map: Schema.Struct<{
    readonly id: Schema.Int;
    readonly name: Schema.String;
    readonly roomNumber: Schema.Int;
  }>;
  readonly priorityGroups: Schema.$Array<Schema.Struct<{
    readonly assignments: Schema.$Array<Schema.Struct<{
      readonly assignmentId: Schema.Int;
      readonly players: Schema.$Array<Schema.Int>;
      readonly strategy: Schema.Union<readonly [Schema.Struct<{
        readonly type: Schema.Literal<"focus">;
      }>, Schema.Struct<{
        readonly message: Schema.String;
        readonly type: Schema.Literal<"message">;
      }>]>;
      readonly target: Schema.Struct<{
        readonly focusActive: Schema.Boolean;
        readonly lifeRevision: Schema.Int;
        readonly monsterMapId: Schema.Int;
        readonly state: Schema.Literals<readonly ["alive", "dead"]>;
      }>;
    }>>;
  }>>;
  readonly sessionId: Schema.String;
}>;
declare const ArmyLoopTauntRegisterResultSchema: Schema.Struct<{
  readonly runId: Schema.String;
}>;
declare const ArmyLoopTauntRunPayloadSchema: Schema.Struct<{
  readonly runId: Schema.String;
  readonly sessionId: Schema.String;
}>;
declare const ArmyLoopTauntLeavePayloadSchema: Schema.Struct<{
  readonly reason: Schema.optionalKey<Schema.String>;
  readonly runId: Schema.String;
  readonly sessionId: Schema.String;
}>;
declare const ArmyLoopTauntTerminalResultSchema: Schema.Union<readonly [Schema.Struct<{
  readonly status: Schema.Literal<"completed">;
}>, Schema.Struct<{
  readonly reason: Schema.String;
  readonly status: Schema.Literal<"failed">;
}>]>;
declare const ArmyLoopTauntReportSchema: Schema.Union<readonly [Schema.Struct<{
  readonly alive: Schema.Boolean;
  readonly cooldownMs: Schema.NullOr<Schema.Finite>;
  readonly type: Schema.Literal<"participant-state">;
  readonly usable: Schema.Boolean;
}>, Schema.Struct<{
  readonly assignmentId: Schema.Int;
  readonly focusActive: Schema.optionalKey<Schema.Boolean>;
  readonly lifeRevision: Schema.Int;
  readonly monsterMapId: Schema.Int;
  readonly state: Schema.Literals<readonly ["alive", "dead"]>;
  readonly type: Schema.Literal<"target-state">;
}>, Schema.Struct<{
  readonly active: Schema.Boolean;
  readonly assignmentId: Schema.Int;
  readonly lifeRevision: Schema.Int;
  readonly monsterMapId: Schema.Int;
  readonly type: Schema.Literal<"focus-state">;
}>, Schema.Struct<{
  readonly assignmentId: Schema.Int;
  readonly lifeRevision: Schema.Int;
  readonly message: Schema.String;
  readonly monsterMapId: Schema.Int;
  readonly type: Schema.Literal<"message">;
}>, Schema.Struct<{
  readonly commandId: Schema.Int;
  readonly cooldownMs: Schema.optionalKey<Schema.NullOr<Schema.Finite>>;
  readonly outcome: Schema.Literals<readonly ["confirmed", "target-unavailable", "cast-failed", "not-ready", "skipped"]>;
  readonly type: Schema.Literal<"command-result">;
}>, Schema.Struct<{
  readonly reason: Schema.String;
  readonly type: Schema.Literal<"failed">;
}>]>;
declare const ArmyLoopTauntReportPayloadSchema: Schema.Struct<{
  readonly report: Schema.Union<readonly [Schema.Struct<{
    readonly alive: Schema.Boolean;
    readonly cooldownMs: Schema.NullOr<Schema.Finite>;
    readonly type: Schema.Literal<"participant-state">;
    readonly usable: Schema.Boolean;
  }>, Schema.Struct<{
    readonly assignmentId: Schema.Int;
    readonly focusActive: Schema.optionalKey<Schema.Boolean>;
    readonly lifeRevision: Schema.Int;
    readonly monsterMapId: Schema.Int;
    readonly state: Schema.Literals<readonly ["alive", "dead"]>;
    readonly type: Schema.Literal<"target-state">;
  }>, Schema.Struct<{
    readonly active: Schema.Boolean;
    readonly assignmentId: Schema.Int;
    readonly lifeRevision: Schema.Int;
    readonly monsterMapId: Schema.Int;
    readonly type: Schema.Literal<"focus-state">;
  }>, Schema.Struct<{
    readonly assignmentId: Schema.Int;
    readonly lifeRevision: Schema.Int;
    readonly message: Schema.String;
    readonly monsterMapId: Schema.Int;
    readonly type: Schema.Literal<"message">;
  }>, Schema.Struct<{
    readonly commandId: Schema.Int;
    readonly cooldownMs: Schema.optionalKey<Schema.NullOr<Schema.Finite>>;
    readonly outcome: Schema.Literals<readonly ["confirmed", "target-unavailable", "cast-failed", "not-ready", "skipped"]>;
    readonly type: Schema.Literal<"command-result">;
  }>, Schema.Struct<{
    readonly reason: Schema.String;
    readonly type: Schema.Literal<"failed">;
  }>]>;
  readonly runId: Schema.String;
  readonly sessionId: Schema.String;
}>;
declare const ArmyLoopTauntCommandSchema: Schema.Struct<{
  readonly assignmentId: Schema.Int;
  readonly expiresAt: Schema.Finite;
  readonly lifeRevision: Schema.Int;
  readonly monsterMapId: Schema.Int;
  readonly type: Schema.Literal<"taunt">;
}>;
declare const ArmyLoopTauntCommandPayloadSchema: Schema.Struct<{
  readonly command: Schema.Struct<{
    readonly assignmentId: Schema.Int;
    readonly expiresAt: Schema.Finite;
    readonly lifeRevision: Schema.Int;
    readonly monsterMapId: Schema.Int;
    readonly type: Schema.Literal<"taunt">;
  }>;
  readonly commandId: Schema.Int;
  readonly runId: Schema.String;
  readonly sessionId: Schema.String;
}>;
declare const normalizeArmyConfigName: (fileName: string) => string;
declare const isValidArmyConfigName: (configName: string) => boolean;
declare const assertValidArmyConfigName: (fileName: string) => string;
declare const normalizeArmyPlayerKey: (playerName: string) => string;
declare const assertArmyConfigRaw: (value: unknown) => ArmyConfigRaw;
declare const parseArmyConfigCore: (value: unknown) => ArmyConfigCore;
declare const normalizeArmyConfig: (configName: string, value: unknown) => ArmyConfigPayload;
declare const resolveArmyEquipSet: (session: Pick<ArmySessionPayload, "playerNumber" | "sets">, setName: string) => ArmyEquipSet | undefined;
declare const resolveArmyItemAlias: (config: Pick<ArmyConfigPayload, "items">, item: string | undefined) => string | undefined;
export { ArmyConfigCore, ArmyConfigPayload, ArmyConfigRaw, ArmyConfigRawSchema, ArmyEquipSet, ArmyEquipSetSchema, ArmyFailPayload, ArmyLoopTauntCommand, ArmyLoopTauntCommandPayload, ArmyLoopTauntCommandPayloadSchema, ArmyLoopTauntCommandSchema, ArmyLoopTauntLeavePayload, ArmyLoopTauntLeavePayloadSchema, ArmyLoopTauntMapIdentity, ArmyLoopTauntMapIdentitySchema, ArmyLoopTauntRegisterPayload, ArmyLoopTauntRegisterPayloadSchema, ArmyLoopTauntRegisterResult, ArmyLoopTauntRegisterResultSchema, ArmyLoopTauntRegistrationAssignment, ArmyLoopTauntRegistrationAssignmentSchema, ArmyLoopTauntRegistrationPriorityGroup, ArmyLoopTauntRegistrationPriorityGroupSchema, ArmyLoopTauntReport, ArmyLoopTauntReportPayload, ArmyLoopTauntReportPayloadSchema, ArmyLoopTauntReportSchema, ArmyLoopTauntResolvedTarget, ArmyLoopTauntResolvedTargetSchema, ArmyLoopTauntRunPayload, ArmyLoopTauntRunPayloadSchema, ArmyLoopTauntStrategy, ArmyLoopTauntStrategySchema, ArmyLoopTauntTerminalResult, ArmyLoopTauntTerminalResultSchema, ArmyProgressPayload, ArmyProgressResult, ArmySessionEndedPayload, ArmySessionEndedPayloadSchema, ArmySessionPayload, ArmySessionPayloadSchema, ArmySetConfig, ArmySetConfigSchema, ArmyStartPayload, ArmySyncPayload, assertArmyConfigRaw, assertValidArmyConfigName, isValidArmyConfigName, normalizeArmyConfig, normalizeArmyConfigName, normalizeArmyPlayerKey, parseArmyConfigCore, resolveArmyEquipSet, resolveArmyItemAlias };