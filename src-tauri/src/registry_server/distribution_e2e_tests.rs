//! End-to-end integration tests for Chunk 1.7 (Real Downloadable Agent
//! Distribution).
//!
//! These tests exercise the **full installer path** for a downloadable
//! third-party agent against a real HTTP fixture:
//!
//! 1. A small `axum` server (the **upstream binary host**, simulating
//!    GitHub Releases / S3) serves the agent binary at a stable URL.
//! 2. A second `axum` server (the **agent registry**, started via
//!    [`crate::registry_server::server::start`]-like logic) serves the
//!    manifest and `307`-redirects `/agents/{name}/download` to the
//!    upstream host.
//! 3. The installer fetches the manifest, follows the redirect via
//!    [`reqwest`], verifies the SHA-256, and writes a non-empty
//!    `agent.bin`.
//!
//! These tests fail loudly if any of the trust gates regress
//! (mandatory `sha256`, redirect contract, hash verification).

#![cfg(test)]

use axum::{
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;

use crate::package_manager::{
    parse_manifest, AgentManifest, InstalledAgent, InstallerError, PackageInstaller,
};
use crate::registry_server::http_registry::HttpRegistry;

/// Spawn the **upstream binary host** at a free port. Returns
/// `(port, JoinHandle)`. Serves a single binary at `/binaries/{name}`.
async fn spawn_upstream_host(
    binaries: HashMap<String, Vec<u8>>,
) -> (u16, tokio::task::JoinHandle<()>) {
    #[derive(Clone)]
    struct UpstreamState {
        binaries: Arc<HashMap<String, Vec<u8>>>,
    }

    async fn serve(
        State(state): State<UpstreamState>,
        AxumPath(name): AxumPath<String>,
    ) -> Result<Response, StatusCode> {
        let bytes = state
            .binaries
            .get(&name)
            .cloned()
            .ok_or(StatusCode::NOT_FOUND)?;
        Ok((
            [(header::CONTENT_TYPE, "application/octet-stream")],
            bytes,
        )
            .into_response())
    }

    let app = Router::new()
        .route("/binaries/{name}", get(serve))
        .with_state(UpstreamState {
            binaries: Arc::new(binaries),
        });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (port, handle)
}

/// Spawn an **agent registry server** seeded with a caller-supplied
/// manifest map. Mirrors [`crate::registry_server::server::start`] but
/// lets the test inject a manifest whose `Binary { url }` points at the
/// upstream-host fixture.
async fn spawn_registry_server(
    manifests: HashMap<String, AgentManifest>,
) -> (u16, tokio::task::JoinHandle<()>) {
    #[derive(Clone)]
    struct RegState {
        agents: Arc<HashMap<String, AgentManifest>>,
    }

    async fn list(State(s): State<RegState>) -> Json<Vec<AgentManifest>> {
        Json(s.agents.values().cloned().collect())
    }
    async fn get_one(
        State(s): State<RegState>,
        AxumPath(name): AxumPath<String>,
    ) -> Result<Json<AgentManifest>, StatusCode> {
        s.agents
            .get(&name)
            .cloned()
            .map(Json)
            .ok_or(StatusCode::NOT_FOUND)
    }
    async fn download(
        State(s): State<RegState>,
        AxumPath(name): AxumPath<String>,
    ) -> Result<Response, StatusCode> {
        let m = s.agents.get(&name).ok_or(StatusCode::NOT_FOUND)?;
        match &m.install_method {
            crate::package_manager::InstallMethod::Binary { url }
            | crate::package_manager::InstallMethod::Wasm { url } => {
                Ok(Redirect::temporary(url).into_response())
            }
            _ => Ok((
                [(header::CONTENT_TYPE, "application/octet-stream")],
                Vec::<u8>::new(),
            )
                .into_response()),
        }
    }
    async fn search(
        State(s): State<RegState>,
        axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
    ) -> Json<Vec<AgentManifest>> {
        let needle = q.get("q").cloned().unwrap_or_default().to_lowercase();
        let out: Vec<AgentManifest> = s
            .agents
            .values()
            .filter(|m| {
                m.name.to_lowercase().contains(&needle)
                    || m.description.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        Json(out)
    }

    let app = Router::new()
        .route("/agents", get(list))
        .route("/agents/{name}", get(get_one))
        .route("/agents/{name}/download", get(download))
        .route("/search", get(search))
        .with_state(RegState {
            agents: Arc::new(manifests),
        });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (port, handle)
}

/// Compute the SHA-256 hex digest. Mirrors the installer's hashing.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let bytes = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[tokio::test]
async fn end_to_end_install_downloads_real_binary_via_redirect() {
    // ── Arrange: a real binary, hosted upstream ─────────────────────────
    let binary: Vec<u8> = b"\x7fELF terransoul-fixture-binary v1.2.3".to_vec();
    let sha = sha256_hex(&binary);

    let mut binaries = HashMap::new();
    binaries.insert("third-party-agent".to_string(), binary.clone());
    let (upstream_port, _upstream) = spawn_upstream_host(binaries).await;

    // Manifest points at the upstream host's URL.
    let manifest_json = format!(
        r#"{{
            "name": "third-party-agent",
            "version": "1.2.3",
            "description": "Third-party downloadable agent (test fixture)",
            "system_requirements": {{}},
            "install_method": {{
                "type": "binary",
                "url": "http://127.0.0.1:{upstream_port}/binaries/third-party-agent"
            }},
            "capabilities": ["chat"],
            "ipc_protocol_version": 1,
            "sha256": "{sha}",
            "license": "MIT",
            "author": "Test Suite"
        }}"#
    );
    let manifest = parse_manifest(&manifest_json).unwrap();

    let mut manifests = HashMap::new();
    manifests.insert("third-party-agent".to_string(), manifest);
    let (reg_port, _registry) = spawn_registry_server(manifests).await;

    // ── Act: install via the real HttpRegistry → installer path ──────
    let tmp = TempDir::new().unwrap();
    let agents_dir = tmp.path().join("agents");
    let mut installer = PackageInstaller::new_in(agents_dir.clone());
    let registry = HttpRegistry::new(reg_port);

    let installed: InstalledAgent = installer
        .install("third-party-agent", &registry)
        .await
        .expect("install must succeed when binary + sha256 are valid");

    // ── Assert: manifest + non-empty binary written to disk ──────────
    assert_eq!(installed.name, "third-party-agent");
    assert_eq!(installed.version, "1.2.3");

    let agent_dir = agents_dir.join("third-party-agent");
    let manifest_path = agent_dir.join("manifest.json");
    let binary_path = agent_dir.join("agent.bin");

    assert!(manifest_path.exists(), "manifest must be written");
    assert!(binary_path.exists(), "agent.bin must be written");

    let written = std::fs::read(&binary_path).expect("read agent.bin");
    assert!(!written.is_empty(), "agent.bin must be non-empty");
    assert_eq!(written, binary, "written bytes must match upstream payload");
    assert_eq!(
        sha256_hex(&written),
        sha,
        "written bytes must match declared sha256"
    );
}

#[tokio::test]
async fn end_to_end_install_rejects_tampered_binary() {
    // The registry advertises one hash; the upstream host serves
    // *different* bytes — the installer must refuse to write.
    let real_binary: Vec<u8> = b"genuine".to_vec();
    let _real_sha = sha256_hex(&real_binary);
    let tampered: Vec<u8> = b"tampered-bytes".to_vec();

    let mut binaries = HashMap::new();
    binaries.insert("tamper-test".to_string(), tampered);
    let (upstream_port, _upstream) = spawn_upstream_host(binaries).await;

    // Manifest declares the *real* SHA but upstream serves tampered bytes.
    let advertised_sha = sha256_hex(&real_binary);
    let manifest_json = format!(
        r#"{{
            "name": "tamper-test",
            "version": "1.0.0",
            "description": "Tampered binary fixture",
            "system_requirements": {{}},
            "install_method": {{
                "type": "binary",
                "url": "http://127.0.0.1:{upstream_port}/binaries/tamper-test"
            }},
            "capabilities": ["chat"],
            "ipc_protocol_version": 1,
            "sha256": "{advertised_sha}"
        }}"#
    );
    let manifest = parse_manifest(&manifest_json).unwrap();

    let mut manifests = HashMap::new();
    manifests.insert("tamper-test".to_string(), manifest);
    let (reg_port, _registry) = spawn_registry_server(manifests).await;

    let tmp = TempDir::new().unwrap();
    let agents_dir = tmp.path().join("agents");
    let mut installer = PackageInstaller::new_in(agents_dir.clone());
    let registry = HttpRegistry::new(reg_port);

    let err = installer
        .install("tamper-test", &registry)
        .await
        .expect_err("tampered binary must be rejected");
    assert!(
        matches!(err, InstallerError::HashMismatch { .. }),
        "expected HashMismatch, got {err:?}"
    );

    // Disk side-effects must not leak.
    let agent_dir = agents_dir.join("tamper-test");
    assert!(
        !agent_dir.exists(),
        "no agent directory should be created on hash mismatch"
    );
}

#[tokio::test]
async fn end_to_end_install_rejects_manifest_missing_sha256() {
    // A registry that advertises a downloadable agent without `sha256`
    // must be rejected by the installer before any download occurs.
    let manifest_json = r#"{
        "name": "no-hash-agent",
        "version": "1.0.0",
        "description": "Missing sha256 fixture",
        "system_requirements": {},
        "install_method": {
            "type": "binary",
            "url": "http://127.0.0.1:1/should-never-be-reached"
        },
        "capabilities": ["chat"],
        "ipc_protocol_version": 1
    }"#;
    let manifest = parse_manifest(manifest_json).unwrap();

    let mut manifests = HashMap::new();
    manifests.insert("no-hash-agent".to_string(), manifest);
    let (reg_port, _registry) = spawn_registry_server(manifests).await;

    let tmp = TempDir::new().unwrap();
    let agents_dir = tmp.path().join("agents");
    let mut installer = PackageInstaller::new_in(agents_dir.clone());
    let registry = HttpRegistry::new(reg_port);

    let err = installer
        .install("no-hash-agent", &registry)
        .await
        .expect_err("missing sha256 must be rejected");
    assert!(
        matches!(err, InstallerError::MissingSha256(ref n) if n == "no-hash-agent"),
        "expected MissingSha256, got {err:?}"
    );
}
