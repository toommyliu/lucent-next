import * as Schema from "effect/Schema";

import { PositiveWireInt } from "../Coercion";

const MonsterDropsPayload = Schema.Struct({
  MonMapID: PositiveWireInt,
  items: Schema.Record(Schema.String, Schema.Unknown),
});

export const decodeMonsterDrops =
  Schema.decodeUnknownOption(MonsterDropsPayload);
