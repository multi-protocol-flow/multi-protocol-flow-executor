//! Minimal semver parsing and `min_host_version` gating.
//!
//! A deliberately small `major.minor.patch` parser with optional prerelease
//! (`-alpha.1`) and build (`+build.5`) suffixes. Build metadata is discarded
//! at parse time (semver precedence ignores it). Prerelease ordering follows
//! the semver subset the host needs: numeric identifiers compare numerically,
//! alphanumeric identifiers compare ASCII-lexically, numeric < alphanumeric,
//! a shorter identifier list is lower than a longer one with the same prefix,
//! and a release (no prerelease) outranks any prerelease of the same core
//! version.
//!
//! Gating must never panic or crash the host: parse failures are typed
//! errors, and the gating entry point treats unparseable versions as
//! **incompatible** with a warning. No external semver crate is used
//! (workspace-hack regeneration is M0-only).

use std::cmp::Ordering;

/// One prerelease identifier (e.g. `alpha`, `rc`, or `1` in `-rc.1`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrereleaseId {
    /// Numeric identifier (e.g. `1` in `-alpha.1`); compares numerically.
    Numeric(u64),
    /// Alphanumeric identifier (e.g. `alpha`, `beta`, `rc`); compares ASCII-lexically.
    Alpha(String),
}

/// A parsed semver version; build metadata is discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// Major component.
    pub major: u64,
    /// Minor component.
    pub minor: u64,
    /// Patch component.
    pub patch: u64,
    /// Prerelease identifiers; empty means a final release.
    pub prerelease: Vec<PrereleaseId>,
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        for (a, b) in [
            (self.major, other.major),
            (self.minor, other.minor),
            (self.patch, other.patch),
        ] {
            match a.cmp(&b) {
                Ordering::Equal => {}
                non_equal => return non_equal,
            }
        }
        // A release outranks any prerelease of the same core version.
        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.prerelease.cmp(&other.prerelease),
        }
    }
}

/// Why a version string failed to parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemverError {
    /// Not shaped like `major.minor.patch` (with optional `-pre` / `+build`).
    #[error("`{0}` is not a `major.minor.patch` semver string")]
    InvalidFormat(String),
    /// A numeric component was not a non-negative integer.
    #[error("version component `{0}` is not a non-negative integer")]
    InvalidComponent(String),
    /// A prerelease or build identifier was empty.
    #[error("`{0}` contains an empty prerelease or build identifier")]
    EmptyIdentifier(String),
}

/// Parse a semver string such as `1.2.3`, `1.2.3-rc.1` or `1.2.3+build.5`.
///
/// Build metadata is validated for shape but discarded from the result.
pub fn parse(input: &str) -> Result<Version, SemverError> {
    let core_and_prerelease = input
        .split('+')
        .next()
        .ok_or_else(|| SemverError::InvalidFormat(input.to_string()))?;
    let (core, prerelease) = match core_and_prerelease.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (core_and_prerelease, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(SemverError::InvalidFormat(input.to_string()));
    }
    let major = parse_component(parts[0])?;
    let minor = parse_component(parts[1])?;
    let patch = parse_component(parts[2])?;
    let prerelease = match prerelease {
        None => Vec::new(),
        Some("") => return Err(SemverError::EmptyIdentifier(input.to_string())),
        Some(pre) => pre
            .split('.')
            .map(|id| parse_prerelease_id(id, input))
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(Version {
        major,
        minor,
        patch,
        prerelease,
    })
}

fn parse_component(part: &str) -> Result<u64, SemverError> {
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(SemverError::InvalidComponent(part.to_string()));
    }
    part.parse::<u64>()
        .map_err(|_| SemverError::InvalidComponent(part.to_string()))
}

fn parse_prerelease_id(id: &str, original: &str) -> Result<PrereleaseId, SemverError> {
    if id.is_empty() {
        return Err(SemverError::EmptyIdentifier(original.to_string()));
    }
    if id.bytes().all(|b| b.is_ascii_digit()) {
        // Leading zeros are accepted and compared numerically — a pragmatic
        // subset of the strict semver grammar.
        return id
            .parse::<u64>()
            .map(PrereleaseId::Numeric)
            .map_err(|_| SemverError::InvalidComponent(id.to_string()));
    }
    Ok(PrereleaseId::Alpha(id.to_string()))
}

/// Whether the host version is at least the `required` minimum version.
///
/// An unparseable version on either side is logged and treated as
/// **incompatible** (`false`) — gating must never panic or crash the host.
pub fn host_satisfies(host: &str, required: &str) -> bool {
    let host_version = match parse(host) {
        Ok(version) => version,
        Err(e) => {
            log::warn!(
                "host version `{host}` is not valid semver: {e}; treating plugin as incompatible"
            );
            return false;
        }
    };
    let required_version = match parse(required) {
        Ok(version) => version,
        Err(e) => {
            log::warn!("min_host_version `{required}` is not valid semver: {e}; treating plugin as incompatible");
            return false;
        }
    };
    host_version >= required_version
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        parse(s).unwrap_or_else(|e| panic!("`{s}` should parse: {e}"))
    }

    #[test]
    fn semver_gate_rejects_incompatible() {
        // host 0.1.0 < required 0.2.0 -> reject
        assert!(!host_satisfies("0.1.0", "0.2.0"));
        // equal-core gaps on each component
        assert!(!host_satisfies("0.2.0", "0.2.1"));
        assert!(!host_satisfies("1.0.0", "2.0.0"));
        assert!(!host_satisfies("1.2.3", "1.2.4"));
    }

    #[test]
    fn semver_gate_accepts_compatible() {
        // host 0.2.0 >= required 0.1.0 -> accept
        assert!(host_satisfies("0.2.0", "0.1.0"));
        // exact match
        assert!(host_satisfies("0.2.0", "0.2.0"));
        // newer host, older requirement
        assert!(host_satisfies("1.5.0", "1.4.9"));
        assert!(host_satisfies("2.0.0", "1.99.99"));
    }

    #[test]
    fn semver_prerelease_ordering() {
        // alpha < beta < rc < release (task contract)
        assert!(v("1.0.0-alpha") < v("1.0.0-beta"));
        assert!(v("1.0.0-beta") < v("1.0.0-rc"));
        assert!(v("1.0.0-rc") < v("1.0.0"));
        // numeric identifiers compare numerically, not lexically
        assert!(v("1.0.0-alpha.1") < v("1.0.0-alpha.2"));
        assert!(v("1.0.0-alpha.2") < v("1.0.0-alpha.10"));
        // shorter identifier list < longer list with same prefix
        assert!(v("1.0.0-alpha") < v("1.0.0-alpha.1"));
        // numeric < alphanumeric
        assert!(v("1.0.0-1") < v("1.0.0-alpha"));
        // gate respects prerelease ordering
        assert!(host_satisfies("1.0.0-rc", "1.0.0-alpha"));
        assert!(!host_satisfies("1.0.0-alpha", "1.0.0"));
    }

    #[test]
    fn semver_parse_error_handled() {
        // invalid min_host_version -> treated as incompatible, never panics
        assert!(!host_satisfies("0.1.0", "not-a-version"));
        assert!(!host_satisfies("0.1.0", "1.2")); // two components
        assert!(!host_satisfies("0.1.0", "1.2.3.4")); // four components
        assert!(!host_satisfies("0.1.0", "1.x.3")); // non-numeric core
        assert!(!host_satisfies("0.1.0", "1.2.3-")); // empty prerelease
                                                     // invalid host version is also incompatible, never panics
        assert!(!host_satisfies("banana", "0.1.0"));
        // build metadata is ignored for precedence
        assert!(host_satisfies("0.1.0+build.5", "0.1.0"));
        assert_eq!(v("1.0.0+a"), v("1.0.0+b"));
    }
}
