fn main() {
    check_migration_version_conflicts();
    embed_windows_manifest();

    let brain_proto = "proto/terransoul/brain.v1.proto";
    let phone_proto = "proto/terransoul/phone_control.v1.proto";
    println!("cargo:rerun-if-changed={brain_proto}");
    println!("cargo:rerun-if-changed={phone_proto}");
    if let Ok(protoc) = protoc_bin_vendored::protoc_bin_path() {
        std::env::set_var("PROTOC", protoc);
    }
    tonic_prost_build::compile_protos(brain_proto).expect("compile brain.v1.proto");
    tonic_prost_build::compile_protos(phone_proto).expect("compile phone_control.v1.proto");
    let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
    let attrs = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attrs).expect("run Tauri build script")
}

fn embed_windows_manifest() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }

    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let manifest_path = out_dir.join("windows-app-manifest.xml");
    std::fs::write(&manifest_path, WINDOWS_APP_MANIFEST).expect("write Windows app manifest");

    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
    println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
    println!("cargo:rerun-if-changed=build.rs");
}

const WINDOWS_APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3" manifestVersion="1.0">
  <assemblyIdentity name="TerranSoul.App" type="win32" version="0.1.0.0" />
    <dependency>
        <dependentAssembly>
            <assemblyIdentity
                type="win32"
                name="Microsoft.Windows.Common-Controls"
                version="6.0.0.0"
                processorArchitecture="*"
                publicKeyToken="6595b64144ccf1df"
                language="*"
            />
        </dependentAssembly>
    </dependency>
    <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
        <application>
            <maxversiontested Id="10.0.18362.1" />
            <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}" />
            <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}" />
            <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}" />
            <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
        </application>
    </compatibility>
    <asmv3:application>
        <asmv3:windowsSettings>
            <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
            <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">permonitorv2</dpiAwareness>
            <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
            <printerDriverIsolation xmlns="http://schemas.microsoft.com/SMI/2011/WindowsSettings">true</printerDriverIsolation>
        </asmv3:windowsSettings>
    </asmv3:application>
    <asmv3:trustInfo>
        <asmv3:security>
            <asmv3:requestedPrivileges>
                <asmv3:requestedExecutionLevel level="asInvoker" uiAccess="false" />
            </asmv3:requestedPrivileges>
        </asmv3:security>
    </asmv3:trustInfo>
</assembly>
"#;

// ─── Migration version conflict detection ───────────────────────────────────

/// Scans `mcp-data/shared/migrations/` for duplicate version numbers.
///
/// If two .sql files share the same 3-digit version prefix (e.g. two people
/// merged branches each adding `008_xxx.sql`), the build is rejected with a
/// clear error message explaining how to resolve the conflict.
fn check_migration_version_conflicts() {
    use std::collections::HashMap;
    use std::path::Path;

    let migrations_dir = Path::new("../mcp-data/shared/migrations");
    println!("cargo:rerun-if-changed=../mcp-data/shared/migrations");

    let entries = match std::fs::read_dir(migrations_dir) {
        Ok(e) => e,
        // Directory missing is fine (fresh clone, CI without checkout)
        Err(_) => return,
    };

    let mut version_map: HashMap<u32, Vec<String>> = HashMap::new();

    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.ends_with(".sql") {
            continue;
        }
        if let Some((version, _name)) = parse_migration_filename(&fname) {
            version_map.entry(version).or_default().push(fname);
        }
    }

    let mut conflicts: Vec<(u32, Vec<String>)> = version_map
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();
    conflicts.sort_by_key(|(v, _)| *v);

    if !conflicts.is_empty() {
        let mut msg = String::from(
            "\n\n╔══════════════════════════════════════════════════════════════╗\n\
               ║  MIGRATION VERSION CONFLICT DETECTED                        ║\n\
               ╠══════════════════════════════════════════════════════════════╣\n\
               ║                                                              ║\n\
               ║  Two or more migration files share the same version number.  ║\n\
               ║  This usually happens when parallel branches each added a    ║\n\
               ║  new migration and were merged without renumbering.          ║\n\
               ║                                                              ║\n\
               ║  TO RESOLVE: renumber the conflicting files so each version  ║\n\
               ║  is unique, then update the compiled_migrations() list in    ║\n\
               ║  src-tauri/src/memory/seed_migrations.rs to match.           ║\n\
               ║                                                              ║\n\
               ╚══════════════════════════════════════════════════════════════╝\n\n",
        );

        for (version, files) in &conflicts {
            msg.push_str(&format!(
                "  Version {version:03} has {} files:\n",
                files.len()
            ));
            for f in files {
                msg.push_str(&format!("    - {f}\n"));
            }
            msg.push('\n');
        }

        msg.push_str(
            "  Action required: rename the duplicate(s) to the next available version\n\
             and update seed_migrations.rs accordingly.\n",
        );

        panic!("{msg}");
    }

    // Also verify versions are sequential with no gaps (warn only)
    let mut versions: Vec<u32> = std::fs::read_dir(migrations_dir)
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let fname = e.file_name().to_string_lossy().to_string();
            parse_migration_filename(&fname).map(|(v, _)| v)
        })
        .collect();
    versions.sort();

    for window in versions.windows(2) {
        let (prev, next) = (window[0], window[1]);
        if next != prev + 1 {
            println!(
                "cargo:warning=Migration version gap: v{prev:03} → v{next:03}. \
                 Expected v{:03}. This is OK after conflict resolution but \
                 may indicate a missing migration.",
                prev + 1
            );
        }
    }
}

/// Parse `001_initial_seed.sql` → `(1, "initial_seed")`.
fn parse_migration_filename(name: &str) -> Option<(u32, String)> {
    let stem = name.strip_suffix(".sql")?;
    let underscore = stem.find('_')?;
    let version: u32 = stem[..underscore].parse().ok()?;
    let label = stem[underscore + 1..].to_string();
    Some((version, label))
}
