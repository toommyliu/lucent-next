use std::{env, fs, process::Command};

#[test]
fn check_accepts_crlf_and_rejects_changed_declarations_without_writing() {
    let path = env::temp_dir().join(format!("lucent-protocol-export-{}.ts", std::process::id()));
    let generate = Command::new(env!("CARGO_BIN_EXE_export-types"))
        .arg(&path)
        .output()
        .unwrap();
    assert!(generate.status.success());
    let crlf = fs::read_to_string(&path).unwrap().replace('\n', "\r\n");
    fs::write(&path, &crlf).unwrap();

    let check = || {
        Command::new(env!("CARGO_BIN_EXE_export-types"))
            .arg("--check")
            .arg(&path)
            .output()
            .unwrap()
    };
    let result = check();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), crlf);

    let stale = format!("{crlf}export type ExtraContract = string;\r\n");
    fs::write(&path, &stale).unwrap();
    assert!(!check().status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), stale);
    fs::remove_file(path).unwrap();
}

#[test]
fn enhancement_labels_allow_undefined_without_changing_json() {
    use lucent_protocol::item::{Enhancement, EnhancementSnapshot};
    use serde_json::json;
    use ts_rs::{Config, TS};

    let declaration = EnhancementSnapshot::decl(&Config::new());
    assert!(
        declaration.contains("name?: string | undefined"),
        "{declaration}"
    );
    assert!(
        declaration.contains("special?: string | undefined"),
        "{declaration}"
    );

    let mut snapshot = EnhancementSnapshot {
        data: Enhancement::default(),
        name: None,
        special: None,
    };
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), json!({}));
    snapshot.name = Some("Fighter".into());
    snapshot.special = Some("Dauntless".into());
    assert_eq!(
        serde_json::to_value(&snapshot).unwrap(),
        json!({ "name": "Fighter", "special": "Dauntless" })
    );
}
