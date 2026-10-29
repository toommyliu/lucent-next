import { LiveModel, normalizeGameText } from "./model";
import { EntityState } from "./protocol";
import type {
  Aura,
  AuraKind,
  AuraQueryOptions,
  AuraSnapshot,
  LiveAura,
} from "./aura";

export { EntityState } from "./protocol";

const percent = (value: number, maximum: number): number =>
  maximum <= 0 ? 0 : (value / maximum) * 100;

export const isEntityState = (value: unknown): value is EntityState =>
  value === EntityState.Dead ||
  value === EntityState.Idle ||
  value === EntityState.InCombat;

export interface Entity {
  readonly alive: boolean;
  readonly auras: readonly Aura[];
  readonly cell: string;
  readonly dead: boolean;
  readonly hp: number;
  readonly hpPercent: number;
  readonly idle: boolean;
  readonly inCombat: boolean;
  readonly maxHp: number;
  readonly maxMp: number;
  readonly mp: number;
  readonly mpPercent: number;
  readonly state: EntityState;
  getAura(name: string, options?: AuraQueryOptions): Aura | null;
  hasAura(name: string, options?: AuraQueryOptions): boolean;
  isInCell(cell: string): boolean;
  toJSON(): EntitySnapshot;
}

export interface EntityData {
  cell: string;
  hp: number;
  maxHp: number;
  maxMp: number;
  mp: number;
  state: EntityState;
}

export type EntitySnapshot = Readonly<EntityData> & {
  readonly alive: boolean;
  readonly auras: readonly AuraSnapshot[];
  readonly dead: boolean;
  readonly hpPercent: number;
  readonly idle: boolean;
  readonly inCombat: boolean;
  readonly mpPercent: number;
};

export abstract class LiveEntity<State extends EntityData>
  extends LiveModel<State>
  implements Entity
{
  readonly #auras = new Map<string, LiveAura>();

  constructor(data: State, auras: readonly LiveAura[] = []) {
    super(data);
    for (const aura of auras) {
      this.#auras.set(this.auraKey(aura.kind, aura.name), aura);
    }
  }

  get alive(): boolean {
    return this.hp > 0 && this.state !== EntityState.Dead;
  }

  get auras(): readonly LiveAura[] {
    return Array.from(this.#auras.values());
  }

  get cell(): string {
    return this.modelData.cell;
  }

  get dead(): boolean {
    return !this.alive;
  }

  get hp(): number {
    return this.modelData.hp;
  }

  get hpPercent(): number {
    return percent(this.hp, this.maxHp);
  }

  get idle(): boolean {
    return this.state === EntityState.Idle;
  }

  get inCombat(): boolean {
    return this.state === EntityState.InCombat;
  }

  get maxHp(): number {
    return this.modelData.maxHp;
  }

  get maxMp(): number {
    return this.modelData.maxMp;
  }

  get mp(): number {
    return this.modelData.mp;
  }

  get mpPercent(): number {
    return percent(this.mp, this.maxMp);
  }

  get state(): EntityState {
    return this.modelData.state;
  }

  getAura(name: string, options?: AuraQueryOptions): LiveAura | null {
    const normalizedName = normalizeGameText(name);
    for (const aura of this.#auras.values()) {
      if (
        normalizeGameText(aura.name) === normalizedName &&
        (options?.kind === undefined || aura.kind === options.kind)
      ) {
        return aura;
      }
    }
    return null;
  }

  hasAura(name: string, options?: AuraQueryOptions): boolean {
    return this.getAura(name, options) !== null;
  }

  private auraKey(kind: AuraKind, name: string): string {
    return `${kind}:${name}`;
  }

  isInCell(cell: string): boolean {
    return normalizeGameText(this.cell) === normalizeGameText(cell);
  }

  abstract toJSON(): EntitySnapshot;
}
