use anyhow::{bail, ensure, Context, Result};
use std::path::{Path, MAIN_SEPARATOR_STR};
/// Convert npm's generated Windows shim to a direct node invocation. No shell is used.
pub fn resolve(program: &str, args: &[String]) -> Result<(String, Vec<String>)> {
    let path = Path::new(program);
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"))
    {
        return Ok((program.into(), args.into()));
    }
    ensure!(
        std::fs::metadata(path)?.len() <= 65536,
        "Batch launcher exceeds size limit"
    );
    let script = std::fs::read_to_string(path).context("Cannot read Windows CLI launcher")?;
    let root = path
        .parent()
        .context("CLI launcher has no parent directory")?;
    let target = script
        .split('"')
        .filter_map(|part| part.strip_prefix("%dp0%"))
        .map(|tail| {
            root.join(
                tail.trim_start_matches(['\\', '/'])
                    .replace('\\', MAIN_SEPARATOR_STR),
            )
        })
        .find(|candidate| {
            candidate
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| ["js", "mjs", "cjs"].contains(&s))
                && candidate.is_file()
        });
    let Some(target) = target else {
        bail!("This batch launcher is not an npm Node CLI; configure its native executable or Node entry point directly");
    };
    let bundled = root.join("node.exe");
    let node = if bundled.is_file() {
        bundled
    } else {
        which::which("node").context("Node.js is required by this CLI launcher")?
    };
    let mut resolved = vec![target.to_string_lossy().into_owned()];
    resolved.extend_from_slice(args);
    Ok((node.to_string_lossy().into_owned(), resolved))
}
