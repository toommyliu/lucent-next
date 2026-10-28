import { LiveModel } from "./model";
import type { DeepReadonly } from "./model";
import type * as Protocol from "./protocol";

export type QuestItem = Readonly<Protocol.QuestItem>;

export type QuestReward = Readonly<Protocol.QuestReward>;

export type QuestCadence = Protocol.QuestCadence;

export interface Quest {
  readonly cadence: QuestCadence;
  readonly id: number;
  readonly name: string;
  readonly once: boolean;
  readonly requirements: readonly QuestItem[];
  readonly rewards: readonly QuestReward[];
  toJSON(): QuestSnapshot;
}

export type QuestData = Omit<Protocol.QuestData, "requirements" | "rewards"> & {
  requirements: readonly QuestItem[];
  rewards: readonly QuestReward[];
};

export type QuestSnapshot = DeepReadonly<Protocol.QuestData>;

export class LiveQuest extends LiveModel<QuestData> implements Quest {
  /**
   * How often the quest can be completed.
   */
  get cadence(): QuestCadence {
    return this.modelData.cadence;
  }
  get id(): number {
    return this.modelData.id;
  }
  get name(): string {
    return this.modelData.name;
  }
  get once(): boolean {
    return this.modelData.once;
  }
  get requirements(): readonly QuestItem[] {
    return this.modelData.requirements;
  }
  get rewards(): readonly QuestReward[] {
    return this.modelData.rewards;
  }
  toJSON(): QuestSnapshot {
    return {
      ...this.modelData,
      requirements: this.requirements.map((item) => ({ ...item })),
      rewards: this.rewards.map((item) => ({ ...item })),
    };
  }
}
