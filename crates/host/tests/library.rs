use lucent_host::library::{Package, load_script};
use lucent_protocol::script::ModulePath;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "lucent-library-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn write(&self, path: &str, source: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    fn package(&self, dir: &str, name: &str) {
        self.write(
            &format!("{dir}/package.json"),
            &format!(r#"{{"name":"{name}","main":"./index.js","lucent":{{}}}}"#),
        );
        self.write(&format!("{dir}/index.js"), "export const value = 42;");
        self.write(
            &format!("{dir}/scripts/run.js"),
            "export default function* () {}",
        );
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn a_daily_loads_by_file_and_by_manifest_name() {
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for spec in [
        "script-packages/lucent-dailies/scripts/farm-elders-blood.js",
        "@lucent/dailies/scripts/farm-elders-blood.js",
    ] {
        let script = load_script(spec, &cwd).unwrap();
        assert_eq!(
            script.entry().to_string(),
            "@lucent/dailies/scripts/farm-elders-blood.js"
        );
        assert_eq!(
            script
                .resolve(script.entry(), "@lucent/dailies")
                .unwrap()
                .to_string(),
            "@lucent/dailies/index.js"
        );
        assert_eq!(
            script
                .resolve(script.entry(), "@lucent/consumables")
                .unwrap()
                .to_string(),
            "@lucent/consumables/index.js"
        );
    }
}

#[test]
fn loose_files_include_the_shared_packages_without_reading_siblings() {
    let ws = Workspace::new();
    ws.write("package.json", r#"{"name":"workspace"}"#);
    ws.write("loop.js", "export default function* () {}");
    ws.write("sibling.js", "export const sibling = true;");
    let script = load_script("loop.js", &ws.0).unwrap();
    assert_eq!(script.entry().to_string(), "loop.js");
    assert!(script.resolve(script.entry(), "@lucent/dailies").is_none());
    assert!(script.resolve(script.entry(), "./sibling.js").is_none());
    ws.package("script-packages/unrelated-folder", "@lucent/dailies");
    let script = load_script("loop.js", &ws.0).unwrap();
    assert_eq!(
        script
            .resolve(script.entry(), "@lucent/dailies")
            .unwrap()
            .to_string(),
        "@lucent/dailies/index.js"
    );
    let script = load_script("@lucent/dailies/scripts/run.js", &ws.0).unwrap();
    assert_eq!(script.entry().to_string(), "@lucent/dailies/scripts/run.js");
    ws.package("script-packages/unscoped", "tools");
    assert_eq!(
        load_script("tools/scripts/run.js", &ws.0)
            .unwrap()
            .entry()
            .to_string(),
        "tools/scripts/run.js"
    );
}

#[test]
fn packages_load_modules_and_skip_dependencies_and_hidden_entries() {
    let ws = Workspace::new();
    ws.package("pkg", "@me/pkg");
    ws.write("pkg/lib/a.mjs", "export const a = 1;");
    ws.write("pkg/node_modules/x/index.js", "excluded");
    ws.write("pkg/.hidden/index.js", "excluded");
    let package = Package::read(&ws.0.join("pkg")).unwrap();
    assert_eq!(package.main.to_string(), "@me/pkg/index.js");
    let script = package.script("scripts/run.js").unwrap();
    assert_eq!(
        script.source(&ModulePath::in_package("@me/pkg", "lib/a.mjs").unwrap()),
        Some("export const a = 1;")
    );
    for path in ["node_modules/x/index.js", ".hidden/index.js"] {
        assert!(
            script
                .source(&ModulePath::in_package("@me/pkg", path).unwrap())
                .is_none()
        );
    }
    assert_eq!(
        load_script("pkg/scripts/run.js", &ws.0)
            .unwrap()
            .entry()
            .to_string(),
        "@me/pkg/scripts/run.js"
    );
}

#[test]
fn duplicate_names_and_missing_specs_name_the_input_and_search_root() {
    let ws = Workspace::new();
    ws.package("script-packages/a", "@me/pkg");
    ws.package("script-packages/b", "@me/pkg");
    let error = load_script("@me/pkg/scripts/run.js", &ws.0)
        .unwrap_err()
        .to_string();
    assert!(error.contains("duplicate package name @me/pkg"), "{error}");
    fs::remove_dir_all(ws.0.join("script-packages/b")).unwrap();
    for spec in [
        "@missing/pkg/run.js",
        "missing.js",
        "@me/pkg/scripts/missing.js",
    ] {
        let error = load_script(spec, &ws.0).unwrap_err().to_string();
        assert!(error.contains(spec), "{error}");
        assert!(
            error.contains(ws.0.join("script-packages").to_str().unwrap()),
            "{error}"
        );
    }
}

#[test]
fn malformed_manifests_report_the_manifest_and_reason() {
    let ws = Workspace::new();
    ws.write("package.json", r#"{"name":"@me/pkg","lucent":{}}"#);
    let error = Package::read(&ws.0).unwrap_err().to_string();
    assert!(
        error.ends_with("is not a Lucent package manifest: \"main\" is missing"),
        "{error}"
    );
}
