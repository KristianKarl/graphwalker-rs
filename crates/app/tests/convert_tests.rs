use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn resource_path(resource: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    path.push(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("resources");
    path.push("models");
    path.push(resource);
    path
}

#[test]
fn convert_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("convert");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(
            "the following required arguments were not provided:",
        ))
        .stderr(predicate::str::contains("<INPUT>"))
        .stderr(predicate::str::contains(
            "For more information, try '--help'",
        ));

    Ok(())
}

#[test]
fn convert_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("convert").arg("file_doesnt_exist");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Could not open file"));

    Ok(())
}

#[test]
fn convert_json_to_dot() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("--debug").arg("TRACE");
    cmd.arg("convert").arg(resource_path("login.json"));
    cmd.arg("--format").arg("dot");
    cmd.assert()
        .stdout(predicate::str::contains("digraph Login"));

    Ok(())
}

#[test]
fn convert_dot_to_json() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("convert").arg(resource_path("dot/login.dot"));
    cmd.arg("--format").arg("json");
    cmd.assert()
        .stderr(predicate::str::contains("Feature not implemented"));

    Ok(())
}

#[test]
fn input_file_format_not_yet_implemented() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("convert")
        .arg(resource_path("graphml/login.graphml"));
    cmd.arg("--format").arg("json");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("File type is not implemented"));

    Ok(())
}

#[test]
fn ouput_file_format_not_yet_implemented() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("convert").arg(resource_path("login.json"));
    cmd.arg("--format").arg("graphml");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Output format for file is not yet implemented",
    ));

    Ok(())
}
