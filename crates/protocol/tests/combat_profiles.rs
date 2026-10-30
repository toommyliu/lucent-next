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

#[test]
fn normalization_repairs_shared_documents() {
    use lucent_protocol::combat_profile::normalize_combat_profile_library;
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../packages/schemas/test-fixtures/combat-profiles/normalization.json"
    ))
    .unwrap();
    for fixture in fixtures {
        let actual = normalize_combat_profile_library(&fixture.input).unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            fixture.expected,
            "{}",
            fixture.name
        );
    }
}

#[test]
fn normalization_matches_typescript_for_all_valid_documents() {
    use lucent_protocol::combat_profile::normalize_combat_profile_library;
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("fixtures/combat_profile_normalization.json")).unwrap();
    let originals: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../packages/schemas/test-fixtures/combat-profiles/valid.json"
    ))
    .unwrap();
    assert_eq!(fixtures.len(), originals.len());
    for (fixture, original) in fixtures.into_iter().zip(originals) {
        assert_eq!(fixture.input, original.input);
        let actual = normalize_combat_profile_library(&fixture.input).unwrap();
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            fixture.expected,
            "{}",
            fixture.name
        );
        assert_eq!(
            normalize_combat_profile_library(&serde_json::to_value(&actual).unwrap()).unwrap(),
            actual
        );
    }
}

#[test]
fn normalization_preserves_reserved_ids_and_utf16_boundaries() {
    use lucent_protocol::combat_profile::{
        normalize_combat_profile, normalize_combat_profile_library,
    };
    let id = format!("{}😀", "a".repeat(77));
    let library = normalize_combat_profile_library(&serde_json::json!({"profiles": [
        {"id": "dupe"}, {"id": "dupe"}, {"id": "dupe-2"}, {"id": id}, {"id": id}
    ]}))
    .unwrap();
    assert_eq!(
        library
            .profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "generic-base",
            "dupe",
            "dupe-3",
            "dupe-2",
            &id,
            &format!("{}-2", "a".repeat(77))
        ]
    );
    assert!(normalize_combat_profile_library(&serde_json::json!({"version": 2})).is_err());
    let profile = normalize_combat_profile(
        &serde_json::json!({"label": format!("{}😀", "a".repeat(79)), "steps": [{"skill": 9.8}, {"skill": "2"}]}),
    );
    assert_eq!(profile.label, "a".repeat(79));
    assert_eq!(profile.steps.len(), 1);
    assert_eq!(
        profile.steps[0].skill,
        lucent_protocol::combat_profile::SkillSlot::Consumable
    );
}
