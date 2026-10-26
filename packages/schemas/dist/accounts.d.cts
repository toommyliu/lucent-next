import * as Schema from "effect/Schema";
declare const ManagedAccountSchema: Schema.Struct<{
  readonly label: Schema.String;
  readonly username: Schema.String;
  readonly password: Schema.String;
}>;
type ManagedAccount = typeof ManagedAccountSchema.Type;
declare const ManagedAccountDraftSchema: Schema.Struct<{
  readonly label: Schema.optionalKey<Schema.String>;
  readonly username: Schema.String;
  readonly password: Schema.String;
}>;
type ManagedAccountDraft = typeof ManagedAccountDraftSchema.Type;
declare const ManagedAccountPatchSchema: Schema.Struct<{
  readonly label: Schema.optionalKey<Schema.String>;
  readonly username: Schema.optionalKey<Schema.String>;
  readonly password: Schema.optionalKey<Schema.String>;
}>;
type ManagedAccountPatch = typeof ManagedAccountPatchSchema.Type;
declare const ManagedAccountGroupsSchema: Schema.$Record<Schema.String, Schema.$Array<Schema.String>>;
type ManagedAccountGroups = typeof ManagedAccountGroupsSchema.Type;
declare const ManagedAccountGroupDraftSchema: Schema.Struct<{
  readonly name: Schema.String;
  readonly usernames: Schema.$Array<Schema.String>;
}>;
type ManagedAccountGroupDraft = typeof ManagedAccountGroupDraftSchema.Type;
declare const ManagedAccountGroupPatchSchema: Schema.Struct<{
  readonly name: Schema.optionalKey<Schema.String>;
  readonly usernames: Schema.optionalKey<Schema.$Array<Schema.String>>;
}>;
type ManagedAccountGroupPatch = typeof ManagedAccountGroupPatchSchema.Type;
declare const AccountScriptReferenceSchema: Schema.Struct<{
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
type AccountScriptReference = typeof AccountScriptReferenceSchema.Type;
declare const AccountSessionConnectionSchema: Schema.Union<readonly [Schema.Struct<{
  readonly state: Schema.Literal<"offline">;
  readonly lastUsername: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"connecting">;
  readonly lastUsername: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"online">;
  readonly username: Schema.String;
}>]>;
type AccountSessionConnection = typeof AccountSessionConnectionSchema.Type;
declare const AccountSessionLoginSchema: Schema.Union<readonly [Schema.Struct<{
  readonly state: Schema.Literal<"idle">;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"waiting-for-game">;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"authenticating">;
  readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
  readonly message: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"waiting-for-server">;
  readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
  readonly message: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"selecting-server">;
  readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
  readonly message: Schema.optionalKey<Schema.String>;
  readonly server: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"waiting-for-player">;
  readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
  readonly message: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"failed">;
  readonly message: Schema.String;
}>]>;
type AccountSessionLogin = typeof AccountSessionLoginSchema.Type;
declare const AccountSessionScriptSchema: Schema.Union<readonly [Schema.Struct<{
  readonly state: Schema.Literal<"idle">;
  readonly name: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"starting">;
  readonly name: Schema.String;
  readonly message: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"running">;
  readonly name: Schema.String;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"stopped">;
  readonly name: Schema.optionalKey<Schema.String>;
  readonly message: Schema.optionalKey<Schema.String>;
}>, Schema.Struct<{
  readonly state: Schema.Literal<"failed">;
  readonly name: Schema.optionalKey<Schema.String>;
  readonly message: Schema.String;
}>]>;
type AccountSessionScript = typeof AccountSessionScriptSchema.Type;
/** Separate runtime facts reconciled by the renderer's session reducer. */
declare const AccountSessionRuntimeSchema: Schema.Struct<{
  readonly connection: Schema.Union<readonly [Schema.Struct<{
    readonly state: Schema.Literal<"offline">;
    readonly lastUsername: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"connecting">;
    readonly lastUsername: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"online">;
    readonly username: Schema.String;
  }>]>;
  readonly login: Schema.Union<readonly [Schema.Struct<{
    readonly state: Schema.Literal<"idle">;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"waiting-for-game">;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"authenticating">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"waiting-for-server">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"selecting-server">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
    readonly server: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"waiting-for-player">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"failed">;
    readonly message: Schema.String;
  }>]>;
  readonly script: Schema.Union<readonly [Schema.Struct<{
    readonly state: Schema.Literal<"idle">;
    readonly name: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"starting">;
    readonly name: Schema.String;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"running">;
    readonly name: Schema.String;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"stopped">;
    readonly name: Schema.optionalKey<Schema.String>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"failed">;
    readonly name: Schema.optionalKey<Schema.String>;
    readonly message: Schema.String;
  }>]>;
}>;
type AccountSessionRuntime = typeof AccountSessionRuntimeSchema.Type;
declare const AccountGameServerSchema: Schema.Struct<{
  readonly name: Schema.String;
  readonly language: Schema.String;
  readonly online: Schema.Boolean;
  readonly upgrade: Schema.Boolean;
  readonly playerCount: Schema.Number;
  readonly maxPlayers: Schema.Number;
}>;
type AccountGameServer = typeof AccountGameServerSchema.Type;
declare const AccountGameServersResultSchema: Schema.Struct<{
  readonly servers: Schema.$Array<Schema.Struct<{
    readonly name: Schema.String;
    readonly language: Schema.String;
    readonly online: Schema.Boolean;
    readonly upgrade: Schema.Boolean;
    readonly playerCount: Schema.Number;
    readonly maxPlayers: Schema.Number;
  }>>;
  readonly refreshAvailableAt: Schema.Number;
}>;
type AccountGameServersResult = typeof AccountGameServersResultSchema.Type;
declare const AccountGameServerPingSchema: Schema.Union<readonly [Schema.Struct<{
  readonly serverName: Schema.String;
  readonly status: Schema.Literal<"ok">;
  readonly latencyMs: Schema.Number;
}>, Schema.Struct<{
  readonly serverName: Schema.String;
  readonly status: Schema.Literals<readonly ["offline", "timeout", "unreachable"]>;
}>]>;
type AccountGameServerPing = typeof AccountGameServerPingSchema.Type;
declare const AccountGameServerPingsResultSchema: Schema.Struct<{
  readonly pings: Schema.$Array<Schema.Union<readonly [Schema.Struct<{
    readonly serverName: Schema.String;
    readonly status: Schema.Literal<"ok">;
    readonly latencyMs: Schema.Number;
  }>, Schema.Struct<{
    readonly serverName: Schema.String;
    readonly status: Schema.Literals<readonly ["offline", "timeout", "unreachable"]>;
  }>]>>;
}>;
type AccountGameServerPingsResult = typeof AccountGameServerPingsResultSchema.Type;
declare const AccountSessionLaunchSchema: Schema.Struct<{
  readonly requestedAt: Schema.Number;
  readonly scriptName: Schema.optionalKey<Schema.String>;
  readonly server: Schema.optionalKey<Schema.String>;
  readonly username: Schema.String;
}>;
type AccountSessionLaunch = typeof AccountSessionLaunchSchema.Type;
declare const AccountGameSessionSchema: Schema.Struct<{
  readonly gameWindowId: Schema.Number;
  readonly gameWindowGroupId: Schema.optionalKey<Schema.Number>;
  readonly rendererGeneration: Schema.Int;
  readonly revision: Schema.Int;
  readonly launch: Schema.optionalKey<Schema.Struct<{
    readonly requestedAt: Schema.Number;
    readonly scriptName: Schema.optionalKey<Schema.String>;
    readonly server: Schema.optionalKey<Schema.String>;
    readonly username: Schema.String;
  }>>;
  readonly connection: Schema.Union<readonly [Schema.Struct<{
    readonly state: Schema.Literal<"offline">;
    readonly lastUsername: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"connecting">;
    readonly lastUsername: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"online">;
    readonly username: Schema.String;
  }>]>;
  readonly login: Schema.Union<readonly [Schema.Struct<{
    readonly state: Schema.Literal<"idle">;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"waiting-for-game">;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"authenticating">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"waiting-for-server">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"selecting-server">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
    readonly server: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"waiting-for-player">;
    readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"failed">;
    readonly message: Schema.String;
  }>]>;
  readonly script: Schema.Union<readonly [Schema.Struct<{
    readonly state: Schema.Literal<"idle">;
    readonly name: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"starting">;
    readonly name: Schema.String;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"running">;
    readonly name: Schema.String;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"stopped">;
    readonly name: Schema.optionalKey<Schema.String>;
    readonly message: Schema.optionalKey<Schema.String>;
  }>, Schema.Struct<{
    readonly state: Schema.Literal<"failed">;
    readonly name: Schema.optionalKey<Schema.String>;
    readonly message: Schema.String;
  }>]>;
  readonly updatedAt: Schema.Number;
}>;
type AccountGameSession = typeof AccountGameSessionSchema.Type;
declare const RecentlyClosedGameSessionSchema: Schema.Struct<{
  readonly id: Schema.Int;
  readonly closedAt: Schema.Number;
  readonly username: Schema.String;
  readonly server: Schema.optionalKey<Schema.String>;
}>;
type RecentlyClosedGameSession = typeof RecentlyClosedGameSessionSchema.Type;
declare const AccountManagerStateSchema: Schema.Struct<{
  readonly accounts: Schema.$Array<Schema.Struct<{
    readonly label: Schema.String;
    readonly username: Schema.String;
    readonly password: Schema.String;
  }>>;
  readonly groups: Schema.$Record<Schema.String, Schema.$Array<Schema.String>>;
  readonly sessions: Schema.$Array<Schema.Struct<{
    readonly gameWindowId: Schema.Number;
    readonly gameWindowGroupId: Schema.optionalKey<Schema.Number>;
    readonly rendererGeneration: Schema.Int;
    readonly revision: Schema.Int;
    readonly launch: Schema.optionalKey<Schema.Struct<{
      readonly requestedAt: Schema.Number;
      readonly scriptName: Schema.optionalKey<Schema.String>;
      readonly server: Schema.optionalKey<Schema.String>;
      readonly username: Schema.String;
    }>>;
    readonly connection: Schema.Union<readonly [Schema.Struct<{
      readonly state: Schema.Literal<"offline">;
      readonly lastUsername: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"connecting">;
      readonly lastUsername: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"online">;
      readonly username: Schema.String;
    }>]>;
    readonly login: Schema.Union<readonly [Schema.Struct<{
      readonly state: Schema.Literal<"idle">;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"waiting-for-game">;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"authenticating">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"waiting-for-server">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"selecting-server">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
      readonly server: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"waiting-for-player">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"failed">;
      readonly message: Schema.String;
    }>]>;
    readonly script: Schema.Union<readonly [Schema.Struct<{
      readonly state: Schema.Literal<"idle">;
      readonly name: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"starting">;
      readonly name: Schema.String;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"running">;
      readonly name: Schema.String;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"stopped">;
      readonly name: Schema.optionalKey<Schema.String>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"failed">;
      readonly name: Schema.optionalKey<Schema.String>;
      readonly message: Schema.String;
    }>]>;
    readonly updatedAt: Schema.Number;
  }>>;
  readonly storagePath: Schema.String;
}>;
type AccountManagerState = typeof AccountManagerStateSchema.Type;
declare const AccountLaunchWindowTargetSchema: Schema.Union<readonly [Schema.Struct<{
  readonly kind: Schema.Literal<"new">;
}>, Schema.Struct<{
  readonly kind: Schema.Literal<"same-as-game">;
  readonly gameWindowId: Schema.Number;
}>]>;
type AccountLaunchWindowTarget = typeof AccountLaunchWindowTargetSchema.Type;
declare const AccountLaunchRequestSchema: Schema.Struct<{
  readonly username: Schema.String;
  readonly script: Schema.optionalKey<Schema.NullOr<Schema.Struct<{
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
  }>>>;
  readonly server: Schema.optionalKey<Schema.String>;
  readonly gameViewLayout: Schema.optionalKey<Schema.Literal<"grid">>;
  readonly windowTarget: Schema.optionalKey<Schema.Union<readonly [Schema.Struct<{
    readonly kind: Schema.Literal<"new">;
  }>, Schema.Struct<{
    readonly kind: Schema.Literal<"same-as-game">;
    readonly gameWindowId: Schema.Number;
  }>]>>;
}>;
type AccountLaunchRequest = typeof AccountLaunchRequestSchema.Type;
declare const AccountLaunchResultSchema: Schema.Struct<{
  readonly gameWindowId: Schema.Number;
}>;
type AccountLaunchResult = typeof AccountLaunchResultSchema.Type;
declare const AccountGameWindowTargetRequestSchema: Schema.Struct<{
  readonly gameWindowId: Schema.Number;
}>;
type AccountGameWindowTargetRequest = typeof AccountGameWindowTargetRequestSchema.Type;
declare const AccountGameLaunchPayloadSchema: Schema.Struct<{
  readonly account: Schema.Struct<{
    readonly label: Schema.String;
    readonly username: Schema.String;
    readonly password: Schema.String;
  }>;
  readonly script: Schema.optionalKey<Schema.Struct<{
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
  }>>;
  readonly server: Schema.optionalKey<Schema.String>;
  readonly gameWindowId: Schema.Number;
  readonly requestedAt: Schema.Number;
}>;
type AccountGameLaunchPayload = typeof AccountGameLaunchPayloadSchema.Type;
/** A full renderer snapshot ordered within one renderer document generation. */
declare const AccountSessionReportSchema: Schema.Struct<{
  readonly rendererGeneration: Schema.Int;
  readonly revision: Schema.Int;
  readonly runtime: Schema.Struct<{
    readonly connection: Schema.Union<readonly [Schema.Struct<{
      readonly state: Schema.Literal<"offline">;
      readonly lastUsername: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"connecting">;
      readonly lastUsername: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"online">;
      readonly username: Schema.String;
    }>]>;
    readonly login: Schema.Union<readonly [Schema.Struct<{
      readonly state: Schema.Literal<"idle">;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"waiting-for-game">;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"authenticating">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"waiting-for-server">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"selecting-server">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
      readonly server: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"waiting-for-player">;
      readonly attemptsRemaining: Schema.optionalKey<Schema.Number>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"failed">;
      readonly message: Schema.String;
    }>]>;
    readonly script: Schema.Union<readonly [Schema.Struct<{
      readonly state: Schema.Literal<"idle">;
      readonly name: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"starting">;
      readonly name: Schema.String;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"running">;
      readonly name: Schema.String;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"stopped">;
      readonly name: Schema.optionalKey<Schema.String>;
      readonly message: Schema.optionalKey<Schema.String>;
    }>, Schema.Struct<{
      readonly state: Schema.Literal<"failed">;
      readonly name: Schema.optionalKey<Schema.String>;
      readonly message: Schema.String;
    }>]>;
  }>;
}>;
type AccountSessionReport = typeof AccountSessionReportSchema.Type;
interface AccountManagerStorage {
  readonly accounts: readonly ManagedAccount[];
  readonly groups: ManagedAccountGroups;
}
declare const normalizeStoredAccount: (account: ManagedAccount) => ManagedAccount;
declare const dedupeAccountsByUsername: (accounts: readonly ManagedAccount[]) => readonly ManagedAccount[];
declare const normalizeAccountManagerStorage: (value: unknown) => AccountManagerStorage;
declare const renameGroupMemberUsername: (groups: ManagedAccountGroups, currentUsername: string, nextUsername: string) => ManagedAccountGroups;
declare const removeGroupMemberUsername: (groups: ManagedAccountGroups, accountUsername: string) => ManagedAccountGroups;
declare const serializeAccountManagerStorage: (storage: AccountManagerStorage) => AccountManagerStorage;
declare const emptyAccountManagerStorage: () => AccountManagerStorage;
export { AccountGameLaunchPayload, AccountGameLaunchPayloadSchema, AccountGameServer, AccountGameServerPing, AccountGameServerPingSchema, AccountGameServerPingsResult, AccountGameServerPingsResultSchema, AccountGameServerSchema, AccountGameServersResult, AccountGameServersResultSchema, AccountGameSession, AccountGameSessionSchema, AccountGameWindowTargetRequest, AccountGameWindowTargetRequestSchema, AccountLaunchRequest, AccountLaunchRequestSchema, AccountLaunchResult, AccountLaunchResultSchema, AccountLaunchWindowTarget, AccountLaunchWindowTargetSchema, AccountManagerState, AccountManagerStateSchema, AccountManagerStorage, AccountScriptReference, AccountScriptReferenceSchema, AccountSessionConnection, AccountSessionConnectionSchema, AccountSessionLaunch, AccountSessionLaunchSchema, AccountSessionLogin, AccountSessionLoginSchema, AccountSessionReport, AccountSessionReportSchema, AccountSessionRuntime, AccountSessionRuntimeSchema, AccountSessionScript, AccountSessionScriptSchema, ManagedAccount, ManagedAccountDraft, ManagedAccountDraftSchema, ManagedAccountGroupDraft, ManagedAccountGroupDraftSchema, ManagedAccountGroupPatch, ManagedAccountGroupPatchSchema, ManagedAccountGroups, ManagedAccountGroupsSchema, ManagedAccountPatch, ManagedAccountPatchSchema, ManagedAccountSchema, RecentlyClosedGameSession, RecentlyClosedGameSessionSchema, dedupeAccountsByUsername, emptyAccountManagerStorage, normalizeAccountManagerStorage, normalizeStoredAccount, removeGroupMemberUsername, renameGroupMemberUsername, serializeAccountManagerStorage };