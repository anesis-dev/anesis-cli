mod common;

use anesis::addons::catalog::{CatalogAddon, CatalogAddonConfig, addon_pick_items};
use common::fixture::{Fixture, build};

fn addon(version: &str, star_count: i64) -> CatalogAddon {
  CatalogAddon {
    addon_id: "docker-compose".to_string(),
    name: "Docker Compose".to_string(),
    version: version.to_string(),
    config: CatalogAddonConfig {
      description: "Container orchestration".to_string(),
    },
    star_count,
  }
}

#[test]
fn to_pick_item_with_no_version_or_stars_has_no_meta() {
  let item = addon("", 0).to_pick_item();
  assert_eq!(item.meta, "");
}

#[test]
fn to_pick_item_shows_the_version_when_present() {
  let item = addon("1.2.3", 0).to_pick_item();
  assert_eq!(item.meta, " · v1.2.3");
}

#[test]
fn to_pick_item_shows_star_count_when_positive() {
  let item = addon("", 42).to_pick_item();
  assert_eq!(item.meta, " · 42★");
}

#[test]
fn to_pick_item_combines_version_and_stars() {
  let item = addon("1.2.3", 42).to_pick_item();
  assert_eq!(item.meta, " · v1.2.3 · 42★");
}

#[test]
fn to_pick_item_zero_stars_is_not_shown() {
  let item = addon("1.0.0", 0).to_pick_item();
  assert_eq!(item.meta, " · v1.0.0");
}

#[test]
fn to_pick_item_copies_id_name_and_description() {
  let item = addon("1.0.0", 0).to_pick_item();
  assert_eq!(item.id, "docker-compose");
  assert_eq!(item.name, "Docker Compose");
  assert_eq!(item.description, "Container orchestration");
}

#[test]
fn to_pick_item_haystack_is_lowercased_id_name_and_description() {
  let item = CatalogAddon {
    addon_id: "Docker-Compose".to_string(),
    name: "DOCKER".to_string(),
    version: String::new(),
    config: CatalogAddonConfig {
      description: "Orchestration".to_string(),
    },
    star_count: 0,
  }
  .to_pick_item();
  assert_eq!(item.haystack, "docker-compose docker orchestration");
}

#[tokio::test]
async fn addon_pick_items_installed_only_reads_the_cache_without_any_network_call() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  fx.install_addon(
    "docker-compose",
    &build::addon_manifest("docker-compose", "1.0.0"),
  );

  let items = addon_pick_items(&ctx, true).await.unwrap();
  assert_eq!(items.len(), 1);
  assert_eq!(items[0].id, "docker-compose");
}

#[tokio::test]
async fn addon_pick_items_installed_only_is_empty_with_no_cache() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();

  let items = addon_pick_items(&ctx, true).await.unwrap();
  assert!(items.is_empty());
}
