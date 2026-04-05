//! End-to-end tests for the `generate` command.
//!
//! These tests load fixture `.sdkforge` session files and exercise the
//! full generate pipeline: load session → extract API model → emit crate.

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
    assert!(output.struct_count > 0, "should generate structs");
    assert_eq!(output.method_count, 3, "should generate 3 methods");

    // Verify all files exist.
    let crate_dir = temp.path().join("petstore-sdk");
    assert!(crate_dir.join("Cargo.toml").exists());
    assert!(crate_dir.join("src/lib.rs").exists());
    assert!(crate_dir.join("src/types.rs").exists());
    assert!(crate_dir.join("src/client.rs").exists());
    assert!(crate_dir.join("src/endpoints.rs").exists());
    assert!(crate_dir.join("src/error.rs").exists());

    // Verify Cargo.toml uses rquest.
    let cargo_toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("rquest"), "should use rquest");
    assert!(
        cargo_toml.contains("name = \"petstore-sdk\""),
        "should have correct crate name"
    );

    // Verify types.rs has Pet struct.
    let types_src = std::fs::read_to_string(crate_dir.join("src/types.rs")).unwrap();
    assert!(
        types_src.contains("pub struct Pet"),
        "should generate Pet struct from Pets resource"
    );
    assert!(
        types_src.contains("pub struct CreatePetRequest"),
        "should generate CreatePetRequest for POST"
    );

    // Verify client.rs has auth.
    let client_src = std::fs::read_to_string(crate_dir.join("src/client.rs")).unwrap();
    assert!(
        client_src.contains("pub struct BearerAuth"),
        "should have BearerAuth from detected auth pattern"
    );
    assert!(
        client_src.contains("pub trait AuthStrategy"),
        "should have AuthStrategy trait"
    );

    // Verify endpoints.rs has methods.
    let endpoints_src = std::fs::read_to_string(crate_dir.join("src/endpoints.rs")).unwrap();
    assert!(
        endpoints_src.contains("async fn list_pets"),
        "should have list_pets"
    );
    assert!(
        endpoints_src.contains("async fn get_pet"),
        "should have get_pet"
    );
    assert!(
        endpoints_src.contains("async fn create_pet"),
        "should have create_pet"
    );

    // Verify error.rs uses rquest.
    let error_src = std::fs::read_to_string(crate_dir.join("src/error.rs")).unwrap();
    assert!(
        error_src.contains("rquest::Error"),
        "errors should reference rquest"
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
    let client_src = std::fs::read_to_string(crate_dir.join("src/client.rs")).unwrap();
    assert!(
        !client_src.contains("BearerAuth"),
        "no-auth SDK should not have BearerAuth"
    );
    assert!(
        client_src.contains("NoAuth"),
        "should still have NoAuth struct"
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
#[ignore = "requires network to download rquest from crates.io"]
fn test_generate_petstore_cargo_check_passes() {
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
        .arg("check")
        .current_dir(temp.path().join("petstore-check"))
        .status()
        .expect("failed to run cargo check");

    assert!(status.success(), "generated crate should pass cargo check");
}
