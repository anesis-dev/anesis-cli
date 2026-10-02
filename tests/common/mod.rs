#![allow(unused_imports)]

pub mod fixture;

pub use anesis::auth::login::{generate_state_token_for_tests, write_auth_file_for_tests};
pub use anesis::auth::token::get_auth_user_with_token_override_for_tests;
pub use anesis::auth::token::is_token_expired_for_tests;

pub use anesis::utils::archive::{
  download_capped_with_progress_for_tests, strip_archive_path_for_tests,
};

pub use anesis::utils::http::{MAX_TOTAL_PAGES_FOR_TESTS, check_total_pages_for_tests};

pub use anesis::utils::cleanup::{prune_empty_parents_for_tests, run_cleanup};

pub use anesis::utils::picker::{match_score_for_tests, smart_match_for_tests, tokenize_for_tests};

pub use anesis::utils::ui::caps::{caps_for_tests, detect_unicode_for_tests};

pub use anesis::doctor::{
  check_addon_cache_for_tests, check_home_writable_for_tests, check_project_consistency_for_tests,
  check_template_cache_for_tests,
};

pub use anesis::mcp::push_inputs_for_tests;

pub use anesis::templates::install::classify_install_state_for_tests as template_classify_install_state_for_tests;

pub use anesis::addons::install::classify_install_state_for_tests as addon_classify_install_state_for_tests;
pub use anesis::addons::runner::is_newer_for_tests;
pub use anesis::addons::runner::prune_empty_dirs_for_tests;
pub use anesis::addons::runner::rerun_prompt_message_for_tests;
pub use anesis::addons::runner::step_label_for_tests;
pub use anesis::addons::runner::undo_conflicts_for_tests;
pub use anesis::addons::steps::packages::detect_pm_for_tests as addon_detect_pm_for_tests;
pub use anesis::addons::steps::packages::missing_pm_error_for_tests as addon_missing_pm_error_for_tests;

pub use anesis::upgrade::{
  asset_filename_for_tests, is_cache_fresh_for_tests, is_newer_version_for_tests,
  normalize_version_tag_for_tests, parse_version_for_tests, parse_version_prerelease_for_tests,
  release_asset_url_for_tests, release_checksums_url_for_tests, upgrade_success_message_for_tests,
  verify_asset_checksum_for_tests,
};
