use anesis::utils::validate::{
  ensure_destination_available, is_valid_github_repo_url, require_https_url, validate_project_name,
  validate_template_name,
};

#[test]
fn project_name_dot_is_valid() {
  assert!(validate_project_name(".").is_ok());
}

#[test]
fn project_name_normal() {
  assert!(validate_project_name("my-project").is_ok());
  assert!(validate_project_name("MyProject123").is_ok());
  assert!(validate_project_name("my_project.v2").is_ok());
}

#[test]
fn project_name_empty_is_err() {
  assert!(validate_project_name("").is_err());
}

#[test]
fn project_name_too_long_is_err() {
  let long = "a".repeat(256);
  assert!(validate_project_name(&long).is_err());
}

#[test]
fn project_name_invalid_chars() {
  for name in ["my project", "my/project", "my@project", "my!project"] {
    assert!(
      validate_project_name(name).is_err(),
      "{name} should be invalid"
    );
  }
}

#[test]
fn project_name_starts_with_dot() {
  assert!(validate_project_name(".hidden").is_err());
}

#[test]
fn project_name_ends_with_dot() {
  assert!(validate_project_name("project.").is_err());
}

#[test]
fn project_name_ends_with_space() {
  assert!(validate_project_name("project ").is_err());
}

#[test]
fn project_name_reserved_windows() {
  for name in ["CON", "con", "NUL", "nul", "COM1", "LPT9"] {
    assert!(
      validate_project_name(name).is_err(),
      "{name} should be reserved"
    );
  }
}

#[test]
fn project_name_reserved_windows_with_extension() {
  for name in ["CON.txt", "con.tar.gz", "NUL.md", "com1.json"] {
    assert!(
      validate_project_name(name).is_err(),
      "{name} should be reserved, since Windows treats any extension on a \
       device name as still referring to the device"
    );
  }
}

#[test]
fn github_url_valid() {
  assert!(is_valid_github_repo_url("https://github.com/owner/repo").is_ok());
  assert!(is_valid_github_repo_url("https://github.com/anesis-dev/anesis").is_ok());
}

#[test]
fn github_url_not_github_domain() {
  assert!(is_valid_github_repo_url("https://gitlab.com/owner/repo").is_err());
  assert!(is_valid_github_repo_url("https://example.com/owner/repo").is_err());
}

#[test]
fn github_url_rejects_non_https_schemes() {
  assert!(is_valid_github_repo_url("http://github.com/owner/repo").is_err());
  assert!(is_valid_github_repo_url("ftp://github.com/owner/repo").is_err());
  assert!(is_valid_github_repo_url("javascript:alert(1)").is_err());
}

#[test]
fn github_url_invalid_format() {
  assert!(is_valid_github_repo_url("not-a-url").is_err());
  assert!(is_valid_github_repo_url("").is_err());
}

#[test]
fn github_url_no_repo_path() {
  assert!(is_valid_github_repo_url("https://github.com/owner").is_err());
  assert!(is_valid_github_repo_url("https://github.com/").is_err());
}

#[test]
fn template_name_valid() {
  for name in ["react-vite-ts", "NextJS", "my_template", "template123"] {
    assert!(
      validate_template_name(name).is_ok(),
      "{name} should be valid"
    );
  }
}

#[test]
fn template_name_invalid() {
  for name in ["my template", "my.template", "my/template", "my@template"] {
    assert!(
      validate_template_name(name).is_err(),
      "{name} should be invalid"
    );
  }
}

#[test]
fn template_name_empty_is_err() {
  assert!(validate_template_name("").is_err());
}

#[test]
fn https_url_is_accepted() {
  assert!(require_https_url("https://example.com/archive.tar.gz", "archive_url").is_ok());
}

#[test]
fn http_url_is_rejected() {
  let err = require_https_url("http://example.com/archive.tar.gz", "archive_url")
    .expect_err("plaintext http must be rejected");
  assert!(err.to_string().contains("https"));
}

#[test]
fn file_scheme_url_is_rejected() {
  assert!(require_https_url("file:///etc/passwd", "archive_url").is_err());
}

#[test]
fn malformed_url_is_rejected() {
  assert!(require_https_url("not-a-url", "archive_url").is_err());
  assert!(require_https_url("", "archive_url").is_err());
}

#[test]
fn destination_free_name_is_available_with_or_without_overwrite() {
  let dir = assert_fs::TempDir::new().unwrap();
  let name = dir.path().join("fresh").to_string_lossy().into_owned();
  assert!(ensure_destination_available(&name, false).is_ok());
  assert!(ensure_destination_available(&name, true).is_ok());
}

#[test]
fn destination_existing_directory_requires_overwrite() {
  let dir = assert_fs::TempDir::new().unwrap();
  let name = dir.path().to_string_lossy().into_owned();
  let err = ensure_destination_available(&name, false).unwrap_err();
  assert!(err.to_string().contains("--overwrite"), "{err}");
  assert!(ensure_destination_available(&name, true).is_ok());
}

#[test]
fn destination_existing_file_is_rejected_even_with_overwrite() {
  let dir = assert_fs::TempDir::new().unwrap();
  let file = dir.path().join("f");
  std::fs::write(&file, "x").unwrap();
  let name = file.to_string_lossy().into_owned();
  assert!(ensure_destination_available(&name, true).is_err());
}

#[test]
fn destination_dot_is_always_available() {
  assert!(ensure_destination_available(".", false).is_ok());
}
