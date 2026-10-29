// @ts-check

import * as script from "lucent/script";
import * as s from "lucent/schema";
import { spinWheelOfDoom } from "@lucent/dailies";

const bankRewardsSchema = s.boolean().default(false);

export default function* run() {
  const bankRewards = bankRewardsSchema.parse(
    yield* script.inputs.get("bankRewards"),
  );
  const result = yield* spinWheelOfDoom({ bankRewards });
  // optionally:
  // yield* script.log(result);
  return result;
}


/** @satisfies {import("lucent/script").ScriptInputsDefinition} */
export const inputs = {
  id: "lucent-dailies-spin-wheel-of-doom",
  fields: [
    {
      key: "bankRewards",
      type: "boolean",
      label: "Bank rewards",
      description: "Deposit wheel rewards after each successful spin.",
      default: false,
    },
  ],
};
