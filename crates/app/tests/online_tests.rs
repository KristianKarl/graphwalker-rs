use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

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
fn online_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("online");
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
fn online_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("online").arg("file_doesnt_exist");
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Could not open file"));

    Ok(())
}

#[test]
fn online() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    //cmd.arg("online").arg(resource_path("login.json"));
    //cmd.assert().success();

    Ok(())
}
