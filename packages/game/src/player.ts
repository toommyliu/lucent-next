import { LiveEntity, type Entity } from "./entity";
import type { DeepReadonly } from "./model";
import type * as Protocol from "./protocol";

export interface PlayerSelectorByUsername {
  readonly username: string;
}

export type PlayerSelector = PlayerSelectorByUsername;
export type PlayerQuery = PlayerSelector | string;

export const toPlayerSelector = (query: PlayerQuery): PlayerSelector => ({
  username: typeof query === "string" ? query.trim() : query.username.trim(),
});

export type BoostType = "classPoints" | "exp" | "gold" | "rep";

export type Position = Readonly<Protocol.Point>;

export interface Player extends Entity {
  readonly afk: boolean;
  readonly entityId: number;
  readonly entityType: string;
  readonly level: number;
  readonly pad: string;
  readonly position: Position;
  readonly username: string;
}

export type PlayerData = Protocol.PlayerData;

export type PlayerSnapshot = DeepReadonly<Protocol.PlayerSnapshot>;

export class LivePlayer extends LiveEntity<PlayerData> implements Player {
  get afk(): boolean {
    return this.modelData.afk;
  }
  get entityId(): number {
    return this.modelData.entityId;
  }
  get entityType(): string {
    return this.modelData.entityType;
  }
  get level(): number {
    return this.modelData.level;
  }
  get pad(): string {
    return this.modelData.pad;
  }
  get position(): Position {
    return this.modelData.position;
  }
  get username(): string {
    return this.modelData.username;
  }

  toJSON(): PlayerSnapshot {
    return {
      ...this.modelData,
      alive: this.alive,
      auras: this.auras.map((aura) => aura.toJSON()),
      dead: this.dead,
      hpPercent: this.hpPercent,
      idle: this.idle,
      inCombat: this.inCombat,
      mpPercent: this.mpPercent,
      position: { ...this.position },
    };
  }
}
