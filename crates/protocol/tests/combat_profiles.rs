use lucent_protocol::combat_profile::CombatProfileLibrary;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    input: Value,
    expected: Value,
}

#[derive(Deserialize)]
struct InvalidFixture {
    name: String,
    input: Value,
}

#[test]
fn profile_documents_roundtrip_to_canonical_json() {
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../packages/schemas/test-fixtures/combat-profiles/valid.json"
    ))
    .unwrap();
    for fixture in fixtures {
        let library: CombatProfileLibrary = serde_json::from_value(fixture.input)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.name));
        assert_eq!(
            serde_json::to_value(&library).unwrap(),
            fixture.expected,
            "{}",
            fixture.name
        );
    }
}

#[test]
fn invalid_profile_documents_are_rejected() {
    let fixtures: Vec<InvalidFixture> = serde_json::from_str(include_str!(
        "../../../packages/schemas/test-fixtures/combat-profiles/invalid.json"
    ))
    .unwrap();
    for fixture in fixtures {
        assert!(
            serde_json::from_value::<CombatProfileLibrary>(fixture.input).is_err(),
            "{}",
            fixture.name
        );
    }
}

#[test]
fn normalized_profile_documents_roundtrip_without_further_defaults() {
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../packages/schemas/test-fixtures/combat-profiles/normalization.json"
    ))
    .unwrap();
    for fixture in fixtures {
        let library: CombatProfileLibrary =
            serde_json::from_value(fixture.expected.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&library).unwrap(),
            fixture.expected,
            "{}",
            fixture.name
        );
    }
}
