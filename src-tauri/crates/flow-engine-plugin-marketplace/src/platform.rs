//! Platform key mapping for marketplace assets.

/// The current platform key, `"<os>-<arch>"` with `x86_64` → `x64` and
/// `aarch64` → `arm64` (e.g. `"windows-x64"`, `"linux-arm64"`), or `None`
/// when the host OS/ARCH combination has no marketplace coverage.
pub fn current_platform() -> Option<String> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_is_known_key() {
        let platform = current_platform();
        assert!(platform.is_some(), "host platform must be mapped");
        let key = platform.unwrap_or_default();
        assert!(
            key.starts_with("windows-") || key.starts_with("linux-") || key.starts_with("macos-"),
            "unexpected OS prefix: {key}"
        );
        assert!(
            key.ends_with("-x64") || key.ends_with("-arm64"),
            "unexpected arch suffix: {key}"
        );
    }
}
