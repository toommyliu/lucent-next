import type * as Protocol from "../../game/src/protocol";
import type { AssertEqual } from "../../game/src/protocol-contract";
import type { DeepReadonly } from "../../game/src/model";
import type * as Profile from "./combatProfiles";

type Assert<T extends true> = T;

export type CombatProfileProtocolContracts = [
  Assert<AssertEqual<Profile.SkillSlot, Protocol.SkillSlot>>,
  Assert<AssertEqual<Profile.CombatProfileCooldownMode, Protocol.CombatProfileCooldownMode>>,
  Assert<AssertEqual<Profile.CombatProfileThresholdUnit, Protocol.CombatProfileThresholdUnit>>,
  Assert<AssertEqual<Profile.CombatProfileComparison, Protocol.CombatProfileComparison>>,
  Assert<AssertEqual<Profile.CombatProfileStatCondition, DeepReadonly<Protocol.CombatProfileStatCondition>>>,
  Assert<AssertEqual<Profile.CombatProfileAuraCondition, DeepReadonly<Protocol.CombatProfileAuraCondition>>>,
  Assert<AssertEqual<Profile.CombatProfileCondition, DeepReadonly<Protocol.CombatProfileCondition>>>,
  Assert<AssertEqual<Profile.CombatProfileStep, DeepReadonly<Protocol.CombatProfileStep>>>,
  Assert<AssertEqual<Profile.CombatProfileMessageTriggerSource, Protocol.CombatProfileMessageTriggerSource>>,
  Assert<AssertEqual<Profile.CombatProfileMessageTrigger, DeepReadonly<Protocol.CombatProfileMessageTrigger>>>,
  Assert<AssertEqual<Profile.CombatProfile, DeepReadonly<Protocol.CombatProfile>>>,
  Assert<AssertEqual<Profile.CombatProfileLibrary, DeepReadonly<Protocol.CombatProfileLibrary>>>,
];
