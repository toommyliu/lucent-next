import { describe, expect, it } from "@effect/vitest";
import * as Schema from "effect/Schema";

import valid from "../test-fixtures/combat-profiles/valid.json";
import invalid from "../test-fixtures/combat-profiles/invalid.json";
import normalization from "../test-fixtures/combat-profiles/normalization.json";

import {
  CombatProfileLibrarySchema,
  normalizeCombatProfileLibrary,
  serializeCombatProfileLibrary,
} from "./combatProfiles";

const FixtureSchema = Schema.Struct({
  name: Schema.String,
  input: Schema.Unknown,
  expected: Schema.Unknown,
});
const corpus = {
  valid: Schema.decodeUnknownSync(Schema.Array(FixtureSchema))(valid),
  invalid: Schema.decodeUnknownSync(
    Schema.Array(Schema.Struct({ name: Schema.String, input: Schema.Unknown })),
  )(invalid),
  normalization: Schema.decodeUnknownSync(Schema.Array(FixtureSchema))(normalization),
};

const decode = Schema.decodeUnknownSync(CombatProfileLibrarySchema);
const encode = Schema.encodeSync(CombatProfileLibrarySchema);

describe("combat profile protocol fixtures", () => {
  for (const fixture of corpus.valid) {
    it(`roundtrips ${fixture.name}`, () => {
      const decoded = decode(fixture.input);
      expect(decoded).toEqual(fixture.expected);
      expect(encode(decoded)).toEqual(fixture.expected);
      expect(decode(encode(decoded))).toEqual(fixture.expected);
    });
  }

  for (const fixture of corpus.invalid) {
    it(`rejects ${fixture.name}`, () => {
      expect(() => decode(fixture.input)).toThrow();
    });
  }

  for (const fixture of corpus.normalization) {
    it(`defaults ${fixture.name}`, () => {
      const normalized = normalizeCombatProfileLibrary(fixture.input);
      expect(normalized).toEqual(fixture.expected);
      expect(encode(decode(normalized))).toEqual(fixture.expected);
      expect(serializeCombatProfileLibrary(normalized)).toEqual(fixture.expected);
    });
  }

  for (const fixture of corpus.valid.filter((fixture) =>
    fixture.name.startsWith("release-"),
  )) {
    it(`preserves saved ${fixture.name}`, () => {
      expect(serializeCombatProfileLibrary(decode(fixture.input))).toEqual(
        fixture.expected,
      );
    });
  }
});
