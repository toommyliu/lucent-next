import type { DeepReadonly } from "@lucent/game";
import type * as Protocol from "@lucent/game/protocol";
import type { ItemSnapshot, MonsterQuery, PlayerSnapshot } from "@lucent/game";
import type { ArmyLoopTauntStrategy, ArmySessionPayload } from "@lucent/schemas/army";
import type { CombatProfileDefinition, SkillSlot } from "@lucent/schemas/combatProfiles";
import type * as Duration from "effect/Duration";
import type * as Effect from "effect/Effect";

export type { SkillSlot } from "@lucent/schemas/combatProfiles";

export interface ScriptExecutionError extends Error {
  readonly _tag: "ScriptExecutionError";
}

export interface ScriptStopSignal extends Error {
  readonly _tag: "ScriptStopSignal";
  readonly reason?: string;
}

export interface AutoReloginState {
  readonly attemptsRemaining?: number;
  readonly attempting: boolean;
  readonly captured: boolean;
  readonly delayMs: number;
  readonly enabled: boolean;
  readonly lastError?: string;
  readonly server?: string;
  readonly username?: string;
  readonly waitingDelay: boolean;
}

export type AutoZoneSupportedMap = Protocol.AutoZoneMap;

export interface AutoZoneState {
  readonly enabled: boolean;
  readonly map: AutoZoneSupportedMap | undefined;
}

export interface ArmyEquipSetOptions {
  readonly resolveItems?: boolean;
}

export interface ArmyError extends Error {
  _tag: unknown;
}

export type ArmySession = ArmySessionPayload;

export interface CellPositionOptions {
  /** Destination cell. */
  readonly cell?: string;
  /** Destination pad. */
  readonly pad?: string;
}

export interface CombatKillOptions {
  readonly profile?: CombatProfileDefinition;
  /**
   * Delay between skill attempts.
   * @defaultValue 150
   */
  readonly skillInterval?: Duration.Input;
  /**
   * Skills to cycle while fighting.
   * @defaultValue [1, 2, 3, 4]
   */
  readonly skills?: readonly SkillSlot[];
  readonly targetPriority?: readonly MonsterQuery[];
}

export interface ArmyRunStepOptions {
  readonly timeout?: Duration.Input;
}

/**
 * Ordered target priorities shared by every participant in the Army.
 *
 * The first group with a living target is selected. When all of its targets
 * die, the next eligible group takes over; a respawn preempts lower groups and
 * restarts that target's rotation from its first assigned player.
 */
export type ArmyLoopTauntPlan = readonly ArmyLoopTauntPriorityGroup[];

/** Stops a Loop Taunt run early; map and script lifecycle changes stop it automatically. */
export interface ArmyLoopTauntHandle {
  /** Stops this run. Repeated calls are safe. */
  stop(): Effect.Effect<void>;
}

export interface ArmyLoopTauntError extends Error {
  _tag: unknown;
}

export interface ConnectOutcome {
  readonly message: string;
  readonly retryable: boolean;
  readonly serverName?: string;
  readonly status:
    | "blocked"
    | "connected"
    | "connection-error"
    | "connection-failed"
    | "full"
    | "not-found"
    | "not-ready"
    | "timeout";
}

export interface BankOpenOptions {
  /**
   * Whether to reload bank items before opening the bank.
   * @defaultValue false
   */
  readonly force?: boolean;
}

export interface HuntOptions {
  /**
   * Whether to prefer the cell containing the most matches.
   * @defaultValue false
   */
  readonly preferMostMatches?: boolean;
}

export interface SkillUseOptions {
  /**
   * Whether to force the skill-use attempt without approaching the target.
   * @defaultValue false
   */
  readonly force?: boolean;
  /** The monster to use the skill on. */
  readonly target?: MonsterQuery;
  /**
   * Whether to wait for the skill to become ready.
   * @defaultValue false
   */
  readonly waitUntilReady?: boolean;
}

export interface EquipOptions {
  /**
   * Whether to wear wearable equipment after equipping it.
   * @defaultValue true
   */
  readonly wear?: boolean;
}

export type ClientPacketEncoding = PacketEncoding | "xml";

export type ServerPacketEncoding = PacketEncoding;

export interface RestOptions {
  /** Whether to wait until both HP and MP are full. */
  readonly waitUntilFull?: boolean;
}

export interface WalkToOptions {
  /** Movement speed override. */
  readonly speed?: number;
}

export interface CompleteQuestOptions {
  /** Number of turn-ins to complete. Must be finite. Omit to use the maximum currently possible. */
  readonly turnIns?: number;
  /** Preferred reward item. Omit when no specific reward is requested. */
  readonly rewardItemId?: number;
}

export type GameAction = 'acceptQuest' | 'buyItem' | 'equipItem' | 'equipLoadout' | 'getMapItem' | 'loadEnhShop' | 'loadHairShop' | 'loadShop' | 'rest' | 'sellItem' | 'tfer' | 'tryQuestComplete' | 'unequipItem' | 'wearItem' | 'wearLoadout';

export interface WaitOptions {
  /**
   * Polling interval.
   * @defaultValue "100 millis"
   */
  readonly interval?: Duration.Input;
  /**
   * Maximum wait duration.
   * @defaultValue undefined
   */
  readonly timeout?: Duration.Input;
}

/**
 * A set of target assignments evaluated at the same priority.
 *
 * Living assignments in the selected group run concurrently.
 */
export interface ArmyLoopTauntPriorityGroup {
  /** Target rotations that may run together while this group is selected. */
  readonly assignments: readonly ArmyLoopTauntAssignment[];
}

export type ScriptCallbackResult<A = unknown> =
  | Effect.Effect<A, unknown>
  | ScriptGenerator<A>;

/**
 * Runs `trigger` only after the subscription is live. A `true` result means the
 * command was sent and a matching response should be awaited; `false` closes
 * the scoped subscription immediately and returns `null`.
 */
export interface TriggeredWaitOptions<E = never, R = never> extends Pick<
  WaitOptions,
  "timeout"
> {
  readonly trigger?: Effect.Effect<boolean, E, R>;
}

export type PacketDirection = Protocol.PacketDirection;

export interface PacketSelector {
  readonly command?: string;
  readonly direction?: PacketDirection;
  readonly encoding?: PacketEncoding;
  readonly predicate?: (packet: FlashPacket) => boolean;
}

export type PacketForDirection<D extends PacketDirection> = Extract<
  FlashPacket,
  { readonly direction: D }
>;

export type Packet = FlashPacket;

export type PacketEncoding = Protocol.PacketEncoding;

/** Assigns one target to the Army players allowed to keep it taunted. */
export interface ArmyLoopTauntAssignment {
  /** One-based player numbers from the active Army roster. */
  readonly players: readonly number[];
  /** Skips the selected player's attempt when the callback returns true. */
  readonly skipWhen?: ArmyLoopTauntSkipWhen;
  /** The event that starts each taunt attempt. */
  readonly strategy: ArmyLoopTauntStrategy;
  /** The monster this assignment may taunt. */
  readonly target: MonsterQuery;
}

export type ScriptGenerator<A = unknown> = Generator<
  Effect.Effect<any, any, never>,
  A,
  any
>;

export type ProjectionEvent =
  | {
      /** A game session starts. */
      readonly type: "login";
    }
  | {
      /** A game session ends. */
      readonly type: "logout";
    }
  | {
      /** A player goes AFK or comes back. */
      readonly type: "player-afk";
      readonly afk: ProjectionEventOf<"player-afk">["afk"];
      readonly entityId: ProjectionEventOf<"player-afk">["entityId"];
      readonly username: ProjectionEventOf<"player-afk">["username"];
    }
  | {
      /** A map finishes loading. */
      readonly type: "join-map";
      readonly map: {
        readonly id: ProjectionEventOf<"join-map">["map"]["id"];
        readonly name: ProjectionEventOf<"join-map">["map"]["name"];
        readonly roomNumber: ProjectionEventOf<"join-map">["map"]["roomNumber"];
      };
    }
  | {
      /** An item drops. */
      readonly type: "item-drop";
      readonly item: ItemSnapshot;
    }
  | {
      /** A quest turn-in succeeds. */
      readonly type: "quest-complete";
      readonly questId: ProjectionEventOf<"quest-complete">["questId"];
    }
  | {
      /** A monster dies. */
      readonly type: "monster-death";
      readonly monsterMapId: ProjectionEventOf<"monster-death">["monsterMapId"];
    }
  | {
      /** A monster respawns. */
      readonly type: "monster-respawn";
      readonly monsterMapId: ProjectionEventOf<"monster-respawn">["monsterMapId"];
    }
  | {
      /** A player dies. */
      readonly type: "player-death";
      readonly entityId: ProjectionEventOf<"player-death">["entityId"];
      readonly username: ProjectionEventOf<"player-death">["username"];
    }
  | {
      /** A player joins or leaves the current map. */
      readonly type: "players-changed";
    }
  | {
      /** An aura first appears on a player or monster. */
      readonly type: "aura-added";
      /** The aura duration in seconds, when available. */
      readonly duration?: Exclude<ProjectionEventOf<"aura-added">["duration"], undefined>;
      readonly icon?: Exclude<ProjectionEventOf<"aura-added">["icon"], undefined>;
      readonly kind: ProjectionEventOf<"aura-added">["kind"];
      readonly name: ProjectionEventOf<"aura-added">["name"];
      readonly stack: ProjectionEventOf<"aura-added">["stack"];
      /** The applying entity's map-scoped ID, when known. */
      readonly sourceId?: Exclude<ProjectionEventOf<"aura-added">["sourceId"], undefined>;
      readonly sourceType?: Exclude<ProjectionEventOf<"aura-added">["sourceType"], undefined>;
      /** The affected entity's map-scoped ID. */
      readonly targetId: ProjectionEventOf<"aura-added">["targetId"];
      readonly targetType: ProjectionEventOf<"aura-added">["targetType"];
    }
  | {
      /** An existing aura changes stack, timer, or metadata. */
      readonly type: "aura-updated";
      /** The aura duration in seconds, when available. */
      readonly duration?: Exclude<ProjectionEventOf<"aura-updated">["duration"], undefined>;
      readonly icon?: Exclude<ProjectionEventOf<"aura-updated">["icon"], undefined>;
      readonly kind: ProjectionEventOf<"aura-updated">["kind"];
      readonly name: ProjectionEventOf<"aura-updated">["name"];
      readonly stack: ProjectionEventOf<"aura-updated">["stack"];
      /** The applying entity's map-scoped ID, when known. */
      readonly sourceId?: Exclude<ProjectionEventOf<"aura-updated">["sourceId"], undefined>;
      readonly sourceType?: Exclude<ProjectionEventOf<"aura-updated">["sourceType"], undefined>;
      /** The affected entity's map-scoped ID. */
      readonly targetId: ProjectionEventOf<"aura-updated">["targetId"];
      readonly targetType: ProjectionEventOf<"aura-updated">["targetType"];
    }
  | {
      /** An aura is removed from a player or monster. */
      readonly type: "aura-removed";
      /** The aura duration in seconds, when available. */
      readonly duration?: Exclude<ProjectionEventOf<"aura-removed">["duration"], undefined>;
      readonly icon?: Exclude<ProjectionEventOf<"aura-removed">["icon"], undefined>;
      readonly kind: ProjectionEventOf<"aura-removed">["kind"];
      readonly name: ProjectionEventOf<"aura-removed">["name"];
      /** The applying entity's map-scoped ID, when known. */
      readonly sourceId?: Exclude<ProjectionEventOf<"aura-removed">["sourceId"], undefined>;
      readonly sourceType?: Exclude<ProjectionEventOf<"aura-removed">["sourceType"], undefined>;
      /** The affected entity's map-scoped ID. */
      readonly targetId: ProjectionEventOf<"aura-removed">["targetId"];
      readonly targetType: ProjectionEventOf<"aura-removed">["targetType"];
    }
  | {
      /** A monster's counter attack is applied, refreshed, or announced in a combat message. */
      readonly type: "counter-attack-start";
      /** The applied aura duration or announced window in milliseconds, when known. */
      readonly durationMs?: Exclude<ProjectionEventOf<"counter-attack-start">["durationMs"], undefined>;
      /** The monster's map-scoped ID. */
      readonly monsterMapId: ProjectionEventOf<"counter-attack-start">["monsterMapId"];
      /** Where the trigger was detected. */
      readonly source: ProjectionEventOf<"counter-attack-start">["source"];
      /** A stable identifier for the recognized trigger. */
      readonly triggerId: ProjectionEventOf<"counter-attack-start">["triggerId"];
      /** The aura name or combat message that matched the trigger. */
      readonly triggerText: ProjectionEventOf<"counter-attack-start">["triggerText"];
    }
  | {
      /** A monster's active counter-attack aura is removed, including server state clears. */
      readonly type: "counter-attack-end";
      /** The monster's map-scoped ID. */
      readonly monsterMapId: ProjectionEventOf<"counter-attack-end">["monsterMapId"];
      /** Where the trigger was detected. */
      readonly source: ProjectionEventOf<"counter-attack-end">["source"];
      /** A stable identifier for the recognized trigger. */
      readonly triggerId: ProjectionEventOf<"counter-attack-end">["triggerId"];
      /** The aura name or combat message that matched the trigger. */
      readonly triggerText: ProjectionEventOf<"counter-attack-end">["triggerText"];
    }
  | ({
      /** A player's location changes. */
      readonly type: "player-location";
      readonly cell: ProjectionEventOf<"player-location">["cell"];
      readonly entityId: ProjectionEventOf<"player-location">["entityId"];
      readonly pad: ProjectionEventOf<"player-location">["pad"];
      /** The latest known coordinates for the player. */
      readonly position: {
        readonly x: ProjectionEventOf<"player-location">["position"]["x"];
        readonly y: ProjectionEventOf<"player-location">["position"]["y"];
      };
      readonly username: ProjectionEventOf<"player-location">["username"];
    } & (
      | {
          /** The destination reported by a complete walk update. */
          readonly destination: {
            readonly x: Extract<ProjectionEventOf<"player-location">, { kind: "walk" }>["destination"]["x"];
            readonly y: Extract<ProjectionEventOf<"player-location">, { kind: "walk" }>["destination"]["y"];
          };
          readonly kind: "walk";
        }
      | {
          /** `position` reports coordinates; `cell` reports only cell/pad. */
          readonly kind: "cell" | "position";
        }
    ))
  | {
      /** A yellow combat message appears. */
      readonly type: "update-message";
      /** The animation's `animStr`, when present. */
      readonly animation?: Exclude<ProjectionEventOf<"update-message">["animation"], undefined>;
      readonly message: ProjectionEventOf<"update-message">["message"];
      /** The related monster's map-scoped ID, when the message names one. */
      readonly monsterMapId?: Exclude<ProjectionEventOf<"update-message">["monsterMapId"], undefined>;
      readonly source: ProjectionEventOf<"update-message">["source"];
    }
  | {
      /** The current map's encounter zone changes. */
      readonly type: "zone";
      readonly map: ProjectionEventOf<"zone">["map"];
      readonly zone: ProjectionEventOf<"zone">["zone"];
    };

/**
 * An event-shaped partial selector. `type` chooses the event variant, and every
 * other field is an exact-match constraint on a scalar field of that variant.
 *
 * Omitting a field leaves it unconstrained. With exact optional property types,
 * explicitly passing `undefined` is invalid; untyped callers that do so safely
 * fail to match instead of selecting events where the field happens to be
 * absent.
 */
export type ProjectionEventSelector =
  | {
      readonly type: "login" | "logout";
    }
  | {
      readonly afk?: boolean;
      readonly entityId?: number;
      readonly type: "player-afk";
      readonly username?: string;
    }
  | {
      readonly type: "join-map";
    }
  | {
      readonly type: "item-drop";
    }
  | {
      readonly questId?: number;
      readonly type: "quest-complete";
    }
  | {
      readonly monsterMapId?: number;
      readonly type: "monster-death" | "monster-respawn";
    }
  | {
      readonly entityId?: number;
      readonly type: "player-death";
      readonly username?: string;
    }
  | {
      readonly entityId?: number;
      readonly kind?: "cell" | "position" | "walk";
      readonly type: "player-location";
      readonly username?: string;
    }
  | {
      readonly type: "players-changed";
    }
  | {
      readonly duration?: number;
      readonly icon?: string;
      readonly kind?: "active" | "passive";
      readonly name?: string;
      readonly sourceId?: number;
      readonly sourceType?: "monster" | "player";
      readonly targetId?: number;
      readonly targetType?: "monster" | "player";
      readonly type: "aura-added" | "aura-updated" | "aura-removed";
    }
  | {
      readonly durationMs?: number;
      readonly monsterMapId?: number;
      readonly source?: "aura" | "message";
      readonly triggerId?: string;
      readonly triggerText?: string;
      readonly type: "counter-attack-start";
    }
  | {
      readonly monsterMapId?: number;
      readonly source?: "aura" | "message";
      readonly triggerId?: string;
      readonly triggerText?: string;
      readonly type: "counter-attack-end";
    }
  | {
      readonly animation?: string;
      readonly message?: string;
      readonly monsterMapId?: number;
      readonly source?: "animation" | "aura";
      readonly type: "update-message";
    }
  | {
      readonly map?: string;
      readonly type: "zone";
      readonly zone?: string;
    };

export type FlashPacket = ClientPacket | ServerPacket | ExtensionPacket;

/** Returns true when the selected player should yield to the next participant. */
export type ArmyLoopTauntSkipWhen = (
  context: ArmyLoopTauntSkipContext,
) =>
  | boolean
  | Effect.Effect<boolean, unknown>
  | Generator<Effect.Effect<any, any, never>, boolean, any>;

export type ClientPacket = { readonly [Key in "command" | "direction" | "params" | "raw" | "encoding"]: DeepReadonly<Extract<Protocol.Packet, { direction: "client" }>[Key]> };

export type ServerPacket = { readonly [Key in "command" | "data" | "direction" | "raw" | "encoding"]: DeepReadonly<Extract<Protocol.Packet, { direction: "server" }>[Key]> };

export type ExtensionPacket = { readonly [Key in "command" | "data" | "direction" | "raw" | "encoding"]: DeepReadonly<Extract<Protocol.Packet, { direction: "extension" }>[Key]> };

/** Projected state available when deciding whether to yield a taunt attempt. */
export interface ArmyLoopTauntSkipContext {
  /** Snapshots for the players assigned to this target. */
  readonly participants: readonly ArmyLoopTauntParticipantSnapshot[];
  /** The assigned participant selected for this attempt. */
  readonly self: ArmyLoopTauntParticipantSnapshot;
}

/** Identifies an assigned Army player and its point-in-time projected state. */
export interface ArmyLoopTauntParticipantSnapshot {
  /** One-based position in the active Army roster. */
  readonly playerNumber: number;
  /** Point-in-time player state captured immediately before this attempt. */
  readonly player: PlayerSnapshot;
}

type ProjectionEventOf<Type extends Protocol.ProjectionEvent["type"]> = Extract<Protocol.ProjectionEvent, { type: Type }>;
