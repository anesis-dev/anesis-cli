use anesis::templates::{
  AnesisTemplate, AnesisTemplateAuthor, AnesisTemplateMetadata, AnesisTemplateRepository,
};

#[test]
fn anesis_template_deserializes_from_json() {
  let json = r#"{
    "name": "react-vite",
    "version": "1.0.0",
    "anesisVersion": "0.9.0",
    "repository": {"url": "https://github.com/anesis-dev/react-vite"},
    "metadata": {"displayName": "React + Vite", "description": "React with Vite bundler"}
  }"#;

  let t: AnesisTemplate = serde_json::from_str(json).unwrap();
  assert_eq!(t.name, "react-vite");
  assert_eq!(t.version, "1.0.0");
  assert_eq!(t.anesis_version, "0.9.0");
  assert_eq!(t.repository.url, "https://github.com/anesis-dev/react-vite");
  assert_eq!(t.metadata.display_name, "React + Vite");
  assert_eq!(t.metadata.description, "React with Vite bundler");
}

#[test]
fn anesis_template_missing_new_fields_still_deserializes_with_defaults() {
  let json = r#"{
    "name": "react-vite",
    "version": "1.0.0",
    "anesisVersion": "0.9.0",
    "repository": {"url": "https://github.com/anesis-dev/react-vite"},
    "metadata": {"displayName": "React + Vite", "description": "React with Vite bundler"}
  }"#;

  let t: AnesisTemplate = serde_json::from_str(json).unwrap();
  assert_eq!(t.author.name, "");
  assert_eq!(t.author.github, "");
  assert_eq!(t.specialization, "");
  assert_eq!(t.scope, "");
  assert!(t.technologies.is_empty());
  assert!(t.languages.is_empty());
  assert_eq!(t.template_type, "");
  assert!(t.metadata.tags.is_empty());
}

#[test]
fn anesis_template_deserializes_all_schema_required_fields() {
  let json = r#"{
    "name": "react-vite",
    "version": "1.0.0",
    "anesisVersion": "0.9.0",
    "author": {"name": "Anesis", "github": "anesis-dev"},
    "repository": {"url": "https://github.com/anesis-dev/react-vite"},
    "specialization": "frontend",
    "scope": "web",
    "technologies": ["react", "vite"],
    "languages": ["typescript"],
    "type": "base",
    "metadata": {
      "displayName": "React + Vite",
      "description": "React with Vite bundler",
      "tags": ["react", "vite", "spa"]
    }
  }"#;

  let t: AnesisTemplate = serde_json::from_str(json).unwrap();
  assert_eq!(t.author.name, "Anesis");
  assert_eq!(t.author.github, "anesis-dev");
  assert_eq!(t.specialization, "frontend");
  assert_eq!(t.scope, "web");
  assert_eq!(t.technologies, vec!["react", "vite"]);
  assert_eq!(t.languages, vec!["typescript"]);
  assert_eq!(t.template_type, "base");
  assert_eq!(t.metadata.tags, vec!["react", "vite", "spa"]);
}

fn full_template() -> AnesisTemplate {
  AnesisTemplate {
    name: "next-app".to_string(),
    version: "2.0.0".to_string(),
    anesis_version: "0.8.0".to_string(),
    author: AnesisTemplateAuthor {
      name: "Anesis".to_string(),
      github: "anesis-dev".to_string(),
    },
    repository: AnesisTemplateRepository {
      url: "https://github.com/example/next-app".to_string(),
    },
    specialization: "fullstack".to_string(),
    scope: "web".to_string(),
    technologies: vec!["next".to_string(), "react".to_string()],
    languages: vec!["typescript".to_string()],
    template_type: "base".to_string(),
    metadata: AnesisTemplateMetadata {
      display_name: "Next.js App".to_string(),
      description: "Next.js application template".to_string(),
      tags: vec!["next".to_string(), "fullstack".to_string()],
    },
    inputs: vec![],
    exclude: vec![],
  }
}

#[test]
fn anesis_template_serializes_with_camel_case_keys() {
  let json = serde_json::to_string(&full_template()).unwrap();
  assert!(
    json.contains("\"anesisVersion\""),
    "should use anesisVersion key"
  );
  assert!(
    json.contains("\"displayName\""),
    "should use displayName key"
  );
  assert!(json.contains("\"next-app\""));
  assert!(
    !json.contains("anesis_version"),
    "should not use snake_case key"
  );
  assert!(
    !json.contains("display_name"),
    "should not use snake_case key"
  );
  assert!(
    !json.contains("official"),
    "should not serialize removed official key"
  );
  assert!(
    json.contains("\"type\":\"base\""),
    "template_type must serialize as the bare `type` key, not template_type: {json}"
  );
  assert!(
    !json.contains("template_type"),
    "should not use the Rust field name for the reserved `type` keyword"
  );
}

#[test]
fn anesis_template_json_round_trip_preserves_all_fields() {
  let original = full_template();

  let json = serde_json::to_string(&original).unwrap();
  let restored: AnesisTemplate = serde_json::from_str(&json).unwrap();

  assert_eq!(restored.name, original.name);
  assert_eq!(restored.version, original.version);
  assert_eq!(restored.anesis_version, original.anesis_version);
  assert_eq!(restored.author.name, original.author.name);
  assert_eq!(restored.author.github, original.author.github);
  assert_eq!(restored.repository.url, original.repository.url);
  assert_eq!(restored.specialization, original.specialization);
  assert_eq!(restored.scope, original.scope);
  assert_eq!(restored.technologies, original.technologies);
  assert_eq!(restored.languages, original.languages);
  assert_eq!(restored.template_type, original.template_type);
  assert_eq!(
    restored.metadata.display_name,
    original.metadata.display_name
  );
  assert_eq!(restored.metadata.description, original.metadata.description);
  assert_eq!(restored.metadata.tags, original.metadata.tags);
}

#[test]
fn repository_serializes_and_deserializes() {
  let repo = AnesisTemplateRepository {
    url: "https://github.com/owner/repo".to_string(),
  };
  let json = serde_json::to_string(&repo).unwrap();
  let back: AnesisTemplateRepository = serde_json::from_str(&json).unwrap();
  assert_eq!(back.url, repo.url);
}

#[test]
fn metadata_deserializes_display_name_camel_case() {
  let json = r#"{"displayName":"My Template","description":"desc"}"#;
  let meta: AnesisTemplateMetadata = serde_json::from_str(json).unwrap();
  assert_eq!(meta.display_name, "My Template");
  assert_eq!(meta.description, "desc");
  assert!(meta.tags.is_empty());
}

#[test]
fn metadata_deserializes_tags() {
  let json = r#"{"displayName":"My Template","description":"desc","tags":["a","b"]}"#;
  let meta: AnesisTemplateMetadata = serde_json::from_str(json).unwrap();
  assert_eq!(meta.tags, vec!["a", "b"]);
}
