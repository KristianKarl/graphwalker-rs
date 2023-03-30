use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

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
            "Usage: graphwalker offline <INPUT>",
        ))
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
        .stderr(predicate::str::contains("Could not open file"));

    Ok(())
}

#[test]
fn offline() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("graphwalker")?;

    cmd.arg("offline")
        .arg("/home/krikar//dev/graphwalker-rs/models/login.json");
    cmd.assert().success();

    Ok(())
}
