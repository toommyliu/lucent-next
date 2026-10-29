// @ts-check

import { farmEldersBlood } from "@lucent/dailies";

export default function* run() {
  return yield* farmEldersBlood();
}
