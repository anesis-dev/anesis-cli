use anyhow::{Result, anyhow};
use semver::{Version, VersionReq};

pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

fn running_version() -> Result<Version> {
  let full = Version::parse(env!("CARGO_PKG_VERSION"))
    .map_err(|e| anyhow!("anesis has an unparseable own version: {e}"))?;
  Ok(Version::new(full.major, full.minor, full.patch))
}

pub fn check_schema_version(kind: &str, id: &str, declared: &str) -> Result<()> {
  let declared_trimmed = declared.trim();

  let Ok(version) = declared_trimmed.parse::<u64>() else {
    return Err(anyhow!(
      "{kind} '{id}' declares schema_version '{declared_trimmed}', which is not a version number.\n\
       Expected a whole number such as \"{SUPPORTED_SCHEMA_VERSION}\"."
    ));
  };

  if version > SUPPORTED_SCHEMA_VERSION {
    return Err(anyhow!(
      "{kind} '{id}' needs manifest schema version {version}, but anesis {} understands up to {SUPPORTED_SCHEMA_VERSION}.\n\
       An unrecognised manifest shape can silently drop fields, so this is refused rather than applied best-effort.\n\
       Upgrade with `anesis upgrade` and try again.",
      env!("CARGO_PKG_VERSION")
    ));
  }

  Ok(())
}

pub fn check_anesis_version(template_name: &str, requirement: &str) -> Result<()> {
  let requirement = requirement.trim();
  if requirement.is_empty() {
    return Ok(());
  }

  let Ok(req) = VersionReq::parse(requirement) else {
    eprintln!(
      "warning: template '{template_name}' declares anesisVersion '{requirement}', \
       which is not a valid semver range — skipping the compatibility check."
    );
    return Ok(());
  };

  let current = running_version()?;
  if !req.matches(&current) {
    return Err(anyhow!(
      "template '{template_name}' requires anesis {requirement}, but this is anesis {current}.\n\
       Upgrade with `anesis upgrade` and try again."
    ));
  }

  Ok(())
}

#[doc(hidden)]
pub fn running_version_for_tests() -> Result<Version> {
  running_version()
}
