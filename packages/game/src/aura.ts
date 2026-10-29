import { LiveModel } from "./model";

export type AuraKind = "active" | "passive";

export interface AuraQueryOptions {
  readonly kind?: AuraKind;
}

export interface Aura {
  readonly category: string | undefined;
  readonly duration: number;
  readonly expiresAt: number | undefined;
  readonly icon: string | undefined;
  readonly kind: AuraKind;
  readonly name: string;
  readonly persistent: boolean;
  readonly stack: number;
  readonly value: number | string | undefined;
  toJSON(): AuraSnapshot;
}

export interface AuraData {
  category?: string;
  duration: number;
  expiresAt?: number;
  icon?: string;
  kind: AuraKind;
  name: string;
  persistent: boolean;
  stack: number;
  value?: number | string;
}

export type AuraSnapshot = Readonly<AuraData>;

export class LiveAura extends LiveModel<AuraData> implements Aura {
  get category(): string | undefined {
    return this.modelData.category;
  }
  /** Full duration in seconds, or `0` when the aura has no timer. */
  get duration(): number {
    return this.modelData.duration;
  }
  /** Expiry time in epoch milliseconds, or `undefined` when the aura has no timer. The aura stays until the server removes it, even after this time. */
  get expiresAt(): number | undefined {
    return this.modelData.expiresAt;
  }
  get icon(): string | undefined {
    return this.modelData.icon;
  }
  get kind(): AuraKind {
    return this.modelData.kind;
  }
  get name(): string {
    return this.modelData.name;
  }
  get persistent(): boolean {
    return this.modelData.persistent;
  }
  get stack(): number {
    return this.modelData.stack;
  }
  get value(): number | string | undefined {
    return this.modelData.value;
  }
  toJSON(): AuraSnapshot {
    return this.snapshot();
  }
}

export const toLiveAura = (snapshot: AuraSnapshot): LiveAura =>
  new LiveAura({ ...snapshot });
