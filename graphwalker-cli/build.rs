use std::path::Path;
use std::process::Command;

fn git_output(manifest_dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(manifest_dir)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn packaged_git_id(contents: &str) -> Option<String> {
    let metadata: serde_json::Value = serde_json::from_str(contents).ok()?;
    let sha = metadata.get("git")?.get("sha1")?.as_str()?;
    if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(sha[..7].to_string())
}

fn main() {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory");
    let manifest_dir = Path::new(&manifest_dir);
    let metadata_path = manifest_dir.join(".cargo_vcs_info.json");
    if metadata_path.exists() {
        println!("cargo:rerun-if-changed={}", metadata_path.display());
    }
    if let Some(git_id) = std::fs::read_to_string(&metadata_path)
        .ok()
        .and_then(|contents| packaged_git_id(&contents))
    {
        println!("cargo:rustc-env=GRAPHWALKER_GIT_ID={git_id}");
        return;
    }

    for path in ["HEAD", "packed-refs"] {
        if let Some(path) = git_output(manifest_dir, &["rev-parse", "--git-path", path]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(reference) = git_output(manifest_dir, &["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git_output(manifest_dir, &["rev-parse", "--git-path", &reference]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }

    let git_id = git_output(manifest_dir, &["rev-parse", "--short=7", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GRAPHWALKER_GIT_ID={git_id}");
}

#[cfg(test)]
mod tests {
    use super::packaged_git_id;

    #[test]
    fn packaged_revision_is_shortened() {
        let metadata = r#"{"git":{"sha1":"9a7ad456ba776c187a0a2ba326e7d17a40ca5e0a"},"path_in_vcs":"graphwalker-cli"}"#;
        assert_eq!(packaged_git_id(metadata).as_deref(), Some("9a7ad45"));
    }

    #[test]
    fn dirty_metadata_preserves_revision() {
        let metadata =
            r#"{"git":{"sha1":"9a7ad456ba776c187a0a2ba326e7d17a40ca5e0a","dirty":true}}"#;
        assert_eq!(packaged_git_id(metadata).as_deref(), Some("9a7ad45"));
    }

    #[test]
    fn invalid_metadata_has_no_revision() {
        for metadata in [
            "not json",
            "{}",
            r#"{"git":{}}"#,
            r#"{"git":{"sha1":null}}"#,
            r#"{"git":{"sha1":""}}"#,
            r#"{"git":{"sha1":"9a7ad45"}}"#,
            r#"{"git":{"sha1":"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"}}"#,
        ] {
            assert_eq!(packaged_git_id(metadata), None, "{metadata}");
        }
    }
}
