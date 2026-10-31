import { describe, expect, it } from "vitest";
import order from "../test-fixtures/environment/order.json";
import fixtures from "../test-fixtures/environment/normalization.json";
import { clearEnvironmentState, normalizeEnvironmentState, type EnvironmentState } from "./environment";
describe("environment normalization corpus", () => {
  for (const fixture of fixtures) it(fixture.name, () => {
    const input = fixture.input as EnvironmentState;
    expect(fixture.clear ? clearEnvironmentState(input) : normalizeEnvironmentState(input)).toEqual(fixture.expected);
  });
});

it("name ordering corpus matches Node", () => {
  for (const [left, right, expected] of order as [string, string, number][]) {
    expect(Math.sign(left.localeCompare(right))).toBe(expected);
  }
});
