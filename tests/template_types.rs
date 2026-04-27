use oxide_cli::templates::{OxideTemplate, OxideTemplateMetadata, OxideTemplateRepository};

// ── JSON serialization / deserialization ──────────────────────────────────────

#[test]
fn oxide_template_deserializes_from_json() {
  let json = r#"{
    "name": "react-vite",
    "version": "1.0.0",
    "oxideVersion": "0.9.0",
    "official": true,
    "repository": {"url": "https://github.com/oxide-cli/react-vite"},
    "metadata": {"displayName": "React + Vite", "description": "React with Vite bundler"}
  }"#;

  let t: OxideTemplate = serde_json::from_str(json).unwrap();
  assert_eq!(t.name, "react-vite");
  assert_eq!(t.version, "1.0.0");
  assert_eq!(t.oxide_version, "0.9.0");
  assert!(t.official);
  assert_eq!(t.repository.url, "https://github.com/oxide-cli/react-vite");
  assert_eq!(t.metadata.display_name, "React + Vite");
  assert_eq!(t.metadata.description, "React with Vite bundler");
}

#[test]
fn oxide_template_serializes_with_camel_case_keys() {
  let template = OxideTemplate {
    name: "next-app".to_string(),
    version: "2.0.0".to_string(),
    oxide_version: "0.8.0".to_string(),
    official: false,
    repository: OxideTemplateRepository {
      url: "https://github.com/example/next-app".to_string(),
    },
    metadata: OxideTemplateMetadata {
      display_name: "Next.js App".to_string(),
      description: "Next.js application template".to_string(),
    },
  };

  let json = serde_json::to_string(&template).unwrap();
  // The rename attributes use camelCase for oxide_version and display_name.
  assert!(json.contains("\"oxideVersion\""), "should use oxideVersion key");
  assert!(json.contains("\"displayName\""), "should use displayName key");
  assert!(json.contains("\"next-app\""));
  assert!(!json.contains("oxide_version"), "should not use snake_case key");
  assert!(!json.contains("display_name"), "should not use snake_case key");
}

#[test]
fn oxide_template_json_round_trip_preserves_all_fields() {
  let original = OxideTemplate {
    name: "svelte-kit".to_string(),
    version: "3.1.0".to_string(),
    oxide_version: "0.9.0".to_string(),
    official: true,
    repository: OxideTemplateRepository {
      url: "https://github.com/oxide-cli/svelte-kit".to_string(),
    },
    metadata: OxideTemplateMetadata {
      display_name: "SvelteKit".to_string(),
      description: "SvelteKit fullstack template".to_string(),
    },
  };

  let json = serde_json::to_string(&original).unwrap();
  let restored: OxideTemplate = serde_json::from_str(&json).unwrap();

  assert_eq!(restored.name, original.name);
  assert_eq!(restored.version, original.version);
  assert_eq!(restored.oxide_version, original.oxide_version);
  assert_eq!(restored.official, original.official);
  assert_eq!(restored.repository.url, original.repository.url);
  assert_eq!(restored.metadata.display_name, original.metadata.display_name);
  assert_eq!(restored.metadata.description, original.metadata.description);
}

#[test]
fn oxide_template_official_false_round_trips() {
  let json = r#"{
    "name": "community-template",
    "version": "0.1.0",
    "oxideVersion": "0.9.0",
    "official": false,
    "repository": {"url": "https://github.com/someone/template"},
    "metadata": {"displayName": "Community", "description": "A community template"}
  }"#;

  let t: OxideTemplate = serde_json::from_str(json).unwrap();
  assert!(!t.official);
}

// ── OxideTemplateRepository ───────────────────────────────────────────────────

#[test]
fn repository_serializes_and_deserializes() {
  let repo = OxideTemplateRepository {
    url: "https://github.com/owner/repo".to_string(),
  };
  let json = serde_json::to_string(&repo).unwrap();
  let back: OxideTemplateRepository = serde_json::from_str(&json).unwrap();
  assert_eq!(back.url, repo.url);
}

// ── OxideTemplateMetadata ─────────────────────────────────────────────────────

#[test]
fn metadata_deserializes_display_name_camel_case() {
  let json = r#"{"displayName":"My Template","description":"desc"}"#;
  let meta: OxideTemplateMetadata = serde_json::from_str(json).unwrap();
  assert_eq!(meta.display_name, "My Template");
  assert_eq!(meta.description, "desc");
}
