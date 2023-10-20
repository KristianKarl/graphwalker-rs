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
fn offline_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("offline");
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
fn offline_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("offline").arg("file_doesnt_exist");
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Could not open file"));

    Ok(())
}

#[test]
fn offline() -> Result<(), Box<dyn std::error::Error>> {
    let list_of_files = vec!["login.json", "petclinic.json"];
    for file in list_of_files {
        println!("Testing file: {}", file);
        let mut cmd = Command::cargo_bin("graphwalker")?;
        cmd.arg("offline").arg(resource_path(file));
        cmd.assert().success();
    }

    Ok(())
}
