// Re-exports of test utilities from src modules
// This centralizes all _for_tests functions in one place

// Auth utilities
pub use oxide_cli::auth::login::{generate_state_token_for_tests, write_auth_file_for_tests};

// Archive utilities
pub use oxide_cli::utils::archive::strip_archive_path_for_tests;

// Cleanup utilities
pub use oxide_cli::utils::cleanup::cleanup_incomplete_template_for_tests;

// Template utilities
pub use oxide_cli::templates::install::classify_install_state_for_tests as template_classify_install_state_for_tests;

// Addon utilities
pub use oxide_cli::addons::runner::{should_fallback_to_cached_manifest_for_tests, rerun_prompt_message_for_tests};
pub use oxide_cli::addons::install::classify_install_state_for_tests as addon_classify_install_state_for_tests;

// Upgrade utilities
pub use oxide_cli::upgrade::{
  normalize_version_tag_for_tests,
  parse_version_for_tests,
  is_newer_version_for_tests,
  is_cache_fresh_for_tests,
  asset_filename_for_tests,
  release_asset_url_for_tests,
};
