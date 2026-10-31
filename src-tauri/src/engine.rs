//! Resolve only our packaged engine. Never execute an unrelated PATH installation.
use std::{
    path::{Path, PathBuf},
    process::Command,
};
fn name() -> &'static str {
    if cfg!(windows) {
        "imapsync-engine.exe"
    } else {
        "imapsync-engine"
    }
}
fn locate(executable: &Path, development: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(parent) = executable.parent() {
        let bundled = parent.join(name());
        if bundled.is_file() {
            return Ok(bundled);
        }
    }
    if let Some(root) = development {
        let extension = if cfg!(windows) { ".exe" } else { "" };
        let prepared = root.join("binaries").join(format!(
            "imapsync-engine-{}{extension}",
            env!("IMAPSYNC_TARGET")
        ));
        if prepared.is_file() {
            return Ok(prepared);
        }
    }
    Err("IMAPSYNC_BUNDLED_MISSING".into())
}
pub fn resolve() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|_| "IMAPSYNC_BUNDLED_MISSING")?;
    let development = if cfg!(debug_assertions) {
        Some(Path::new(env!("CARGO_MANIFEST_DIR")))
    } else {
        None
    };
    locate(&executable, development)
}
pub fn sanitize(command: &mut Command) {
    // PAR carries its own interpreter/modules. Host Perl configuration must not
    // inject code, replace packed libraries, or redirect the extraction cache.
    for key in [
        "PERL5OPT",
        "PERL5LIB",
        "PERLLIB",
        "PERL_LOCAL_LIB_ROOT",
        "PERL_MM_OPT",
        "PERL_MB_OPT",
        "PAR_TEMP",
        "PAR_GLOBAL_TEMP",
        "PAR_PROGNAME",
        "PAR_CLEAN",
        "PAR_RUN",
    ] {
        command.env_remove(key);
    }
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name.starts_with("PERL") || name.starts_with("PAR_") {
            command.env_remove(key);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW; retain stdout/stderr pipes.
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_packaged_engine_before_development_copy_and_never_path() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir(&app).unwrap();
        let dev = dir.path().join("dev");
        std::fs::create_dir_all(dev.join("binaries")).unwrap();
        let extension = if cfg!(windows) { ".exe" } else { "" };
        let prepared = dev.join("binaries").join(format!(
            "imapsync-engine-{}{extension}",
            env!("IMAPSYNC_TARGET")
        ));
        let executable = app.join("studio");
        assert!(locate(&executable, None).is_err());
        std::fs::write(&prepared, b"fixture").unwrap();
        assert_eq!(locate(&executable, Some(&dev)).unwrap(), prepared);
        assert!(locate(&executable, None).is_err());
        let bundled = app.join(name());
        std::fs::write(&bundled, b"fixture").unwrap();
        assert_eq!(locate(&executable, Some(&dev)).unwrap(), bundled);
        assert_eq!(locate(&executable, None).unwrap(), bundled);
    }
    #[test]
    fn clears_host_perl_configuration_without_mutating_parent_environment() {
        let mut command = Command::new("fixture");
        command
            .env("PERL5OPT", "-MUntrusted")
            .env("PERL5LIB", "host-modules")
            .env("PAR_GLOBAL_TEMP", "host-cache");
        sanitize(&mut command);
        for key in ["PERL5OPT", "PERL5LIB", "PAR_GLOBAL_TEMP"] {
            assert!(command
                .get_envs()
                .any(|(name, value)| name == key && value.is_none()));
        }
    }
}
