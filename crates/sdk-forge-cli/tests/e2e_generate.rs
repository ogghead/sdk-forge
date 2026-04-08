//! End-to-end tests for the `generate` command.
//!
//! These tests load fixture `.sdkforge` session files and exercise the
//! full generate pipeline: load session → extract API model → emit
//! Wasm component crate (WIT spec + Rust implementation).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_docs_in_private_items
)]

mod common;

use sdk_forge_codegen::emitter::{EmitConfig, emit};
use sdk_forge_session::io::load_session;

// ── Fixture validation ─────────────────────────────────────────────

#[test]
fn test_petstore_analyzed_fixture_loads() {
    let session = load_session(&common::petstore_analyzed()).unwrap();

    assert_eq!(
        session.exchanges.len(),
        3,
        "petstore fixture should have 3 exchanges"
    );
    assert!(
        session.api_model.is_some(),
        "analyzed fixture must have an api_model"
    );

    let model = session.api_model.as_ref().unwrap();
    assert_eq!(model.base_url, "https://petstore.example.com");
    assert!(model.auth.is_some(), "petstore should have auth");
    assert_eq!(model.resources.len(), 1, "should have 1 resource group");
    assert_eq!(
        model.resources[0].endpoints.len(),
        3,
        "Pets group should have 3 endpoints"
    );
}

#[test]
fn test_petstore_unanalyzed_fixture_loads() {
    let session = load_session(&common::petstore_unanalyzed()).unwrap();

    assert!(
        session.api_model.is_none(),
        "unanalyzed fixture must not have api_model"
    );
    assert_eq!(session.exchanges.len(), 1);
}

#[test]
fn test_minimal_noauth_fixture_loads() {
    let session = load_session(&common::minimal_noauth()).unwrap();

    let model = session.api_model.as_ref().unwrap();
    assert!(model.auth.is_none(), "no-auth fixture should have no auth");
    assert_eq!(model.resources.len(), 1);
}

// ── Generate pipeline ──────────────────────────────────────────────

#[test]
fn test_generate_petstore_emits_complete_crate() {
    let session = load_session(&common::petstore_analyzed()).unwrap();
    let model = session.api_model.as_ref().unwrap();

    let temp = tempfile::tempdir().unwrap();
    let config = EmitConfig {
        crate_name: "petstore-sdk".to_owned(),
        output_dir: temp.path().to_path_buf(),
        templates_dir: common::templates_dir(),
    };

    let output = emit(model, &config).unwrap();

    assert_eq!(output.file_count, 6, "should write 6 files");
    assert!(output.struct_count > 0, "should generate records");
    assert_eq!(output.method_count, 3, "should generate 3 methods");

    // Verify all files exist.
    let crate_dir = temp.path().join("petstore-sdk");
    assert!(crate_dir.join("Cargo.toml").exists());
    assert!(crate_dir.join("wit/world.wit").exists());
    assert!(crate_dir.join("src/lib.rs").exists());
    assert!(crate_dir.join("src/types.rs").exists());
    assert!(crate_dir.join("src/http.rs").exists());
    assert!(crate_dir.join("src/error.rs").exists());

    // Verify Cargo.toml uses wit-bindgen and cdylib.
    let cargo_toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("wit-bindgen"), "should use wit-bindgen");
    assert!(cargo_toml.contains("cdylib"), "should be a cdylib");
    assert!(
        cargo_toml.contains("name = \"petstore-sdk\""),
        "should have correct crate name"
    );

    // Verify world.wit has WIT records and interfaces.
    let world_wit = std::fs::read_to_string(crate_dir.join("wit/world.wit")).unwrap();
    assert!(
        world_wit.contains("package petstore-sdk:api"),
        "should have package declaration"
    );
    assert!(
        world_wit.contains("record pet"),
        "should have pet record (singularized from Pets resource)"
    );
    assert!(
        world_wit.contains("record create-pet-request"),
        "should have create-pet-request for POST"
    );
    assert!(
        world_wit.contains("interface pets"),
        "should have pets interface"
    );
    assert!(
        world_wit.contains("list-pets"),
        "should have list-pets function"
    );
    assert!(
        world_wit.contains("get-pet"),
        "should have get-pet function"
    );
    assert!(
        world_wit.contains("create-pet"),
        "should have create-pet function"
    );
    assert!(
        world_wit.contains("wasi:http/outgoing-handler"),
        "should import WASI HTTP"
    );
    assert!(
        world_wit.contains("record bearer-auth"),
        "should have bearer-auth record from detected auth pattern"
    );

    // Verify types.rs has Rust struct definitions.
    let types_src = std::fs::read_to_string(crate_dir.join("src/types.rs")).unwrap();
    assert!(
        types_src.contains("pub struct Pet"),
        "should generate Pet struct from Pets resource"
    );
    assert!(
        types_src.contains("pub struct CreatePetRequest"),
        "should generate CreatePetRequest for POST"
    );

    // Verify http.rs has WASI HTTP helpers.
    let http_src = std::fs::read_to_string(crate_dir.join("src/http.rs")).unwrap();
    assert!(
        http_src.contains("send_request"),
        "should have send_request helper"
    );
    assert!(
        http_src.contains("apply_auth"),
        "should have apply_auth for bearer auth"
    );
}

#[test]
fn test_generate_noauth_model_omits_bearer() {
    let session = load_session(&common::minimal_noauth()).unwrap();
    let model = session.api_model.as_ref().unwrap();

    let temp = tempfile::tempdir().unwrap();
    let config = EmitConfig {
        crate_name: "noauth-sdk".to_owned(),
        output_dir: temp.path().to_path_buf(),
        templates_dir: common::templates_dir(),
    };

    let output = emit(model, &config).unwrap();
    assert_eq!(output.method_count, 1);

    let crate_dir = temp.path().join("noauth-sdk");
    let world_wit = std::fs::read_to_string(crate_dir.join("wit/world.wit")).unwrap();
    assert!(
        !world_wit.contains("bearer-auth"),
        "no-auth SDK should not have bearer-auth record"
    );
}

#[test]
fn test_generate_rejects_session_without_api_model() {
    let session = load_session(&common::petstore_unanalyzed()).unwrap();
    assert!(
        session.api_model.is_none(),
        "fixture precondition: no api_model"
    );
}

#[test]
#[ignore = "requires cargo-component to be installed"]
fn test_generate_petstore_cargo_component_check_passes() {
    let session = load_session(&common::petstore_analyzed()).unwrap();
    let model = session.api_model.as_ref().unwrap();

    let temp = tempfile::tempdir().unwrap();
    let config = EmitConfig {
        crate_name: "petstore-check".to_owned(),
        output_dir: temp.path().to_path_buf(),
        templates_dir: common::templates_dir(),
    };

    emit(model, &config).unwrap();

    let status = std::process::Command::new("cargo")
        .args(["component", "check"])
        .current_dir(temp.path().join("petstore-check"))
        .status()
        .expect("failed to run cargo component check");

    assert!(
        status.success(),
        "generated crate should pass cargo component check"
    );
}
