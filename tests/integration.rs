// Copyright 2021, Erwin van Eijk
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_testdata_integration() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let output_state_file = dir.path().join("output_state_file.json");
    let database_file = dir.path().join("database.json");
    let paths = vec!["testdata"];
    let files = vec!["testdata/testfile-yes.bin"];
    let number_of_results = 5;

    let mut index_command = Command::cargo_bin("fbhash")?;
    index_command
        .arg("index")
        .arg("--state")
        .arg(output_state_file.clone())
        .arg("--database")
        .arg(database_file.to_str().unwrap())
        .arg(paths[0]);
    index_command.assert().success();

    let mut query_command = Command::cargo_bin("fbhash")?;
    query_command
        .arg("query")
        .arg(format!("-n={}", number_of_results))
        .arg("--database")
        .arg(database_file.clone())
        .arg("--state")
        .arg(output_state_file.clone())
        .arg(files[0]);

    #[cfg(not(target_os = "windows"))]
    query_command.assert().success().stdout(format!(
        "Similarities for {}\n\
Results: 3\n\
testdata/testfile-yes.bin => (0.00000000000000011102230246251565) testdata/testfile-yes.bin\n\
testdata/testfile-yes.bin => (1) testdata/testfile-zero-length\n\
testdata/testfile-yes.bin => (1) testdata/testfile-zero.bin\n\n",
        files[0]
    ));

    #[cfg(target_os = "windows")]
    query_command.assert().success().stdout(format!(
        "Similarities for {}\n\
Results: 3\n\
testdata/testfile-yes.bin => (0.00000000000000011102230246251565) testdata\\testfile-yes.bin\n\
testdata/testfile-yes.bin => (1) testdata\\testfile-zero-length\n\
testdata/testfile-yes.bin => (1) testdata\\testfile-zero.bin\n\n",
        files[0]
    ));

    dir.close()?;
    Ok(())
}

#[test]
fn test_testdata_integration_binary() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let output_state_file = dir.path().join("output_state_file.bin");
    let database_file = dir.path().join("database.bin");
    let paths = vec!["testdata"];
    let files = vec!["testdata/testfile-yes.bin"];
    let number_of_results = 5;

    let mut index_command = Command::cargo_bin("fbhash")?;
    index_command
        .arg("--binary")
        .arg("index")
        .arg("--state")
        .arg(output_state_file.clone())
        .arg(format!("--database={}", database_file.to_str().unwrap()))
        .arg(paths[0]);
    index_command.assert().success();

    let mut query_command = Command::cargo_bin("fbhash")?;
    query_command
        .arg("--binary")
        .arg("query")
        .arg(format!("-n={}", number_of_results))
        .arg("--database")
        .arg(database_file.clone())
        .arg("--state")
        .arg(output_state_file.clone())
        .arg(files[0]);

    #[cfg(not(target_os = "windows"))]
    query_command.assert().success().stdout(format!(
        "Similarities for {}\n\
Results: 3\n\
testdata/testfile-yes.bin => (0.00000000000000011102230246251565) testdata/testfile-yes.bin\n\
testdata/testfile-yes.bin => (1) testdata/testfile-zero-length\n\
testdata/testfile-yes.bin => (1) testdata/testfile-zero.bin\n\n",
        files[0]
    ));

    #[cfg(target_os = "windows")]
    query_command.assert().success().stdout(format!(
        "Similarities for {}\n\
Results: 3\n\
testdata/testfile-yes.bin => (0.00000000000000011102230246251565) testdata\\testfile-yes.bin\n\
testdata/testfile-yes.bin => (1) testdata\\testfile-zero-length\n\
testdata/testfile-yes.bin => (1) testdata\\testfile-zero.bin\n\n",
        files[0]
    ));

    dir.close()?;
    Ok(())
}

#[test]
fn test_testdata_format_wrong() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let output_state_file = dir.path().join("output_state_file.bin");
    let database_file = dir.path().join("database.bin");
    let paths = vec!["testdata"];
    let files = vec!["testdata/testfile-yes.bin"];
    let number_of_results = 5;

    let mut index_command = Command::cargo_bin("fbhash")?;
    index_command
        .arg("--binary")
        .arg("index")
        .arg("--state")
        .arg(output_state_file.clone())
        .arg("--database")
        .arg(database_file.to_str().unwrap())
        .arg(paths[0]);
    index_command.assert().success();

    let mut query_command = Command::cargo_bin("fbhash")?;
    query_command
        .arg("query")
        .arg(format!("-n={}", number_of_results))
        .arg("--database")
        .arg(database_file.clone())
        .arg("--state")
        .arg(output_state_file.clone())
        .arg(files[0]);

    #[cfg(not(target_os = "windows"))]
    query_command.assert().failure().stderr("Error: Custom { kind: InvalidData, error: Error(\"expected value\", line: 1, column: 1) }\n");

    #[cfg(target_os = "windows")]
    query_command.assert().failure().stderr("Error: Custom { kind: InvalidData, error: Error(\"expected value\", line: 1, column: 1) }\n");

    dir.close()?;
    Ok(())
}

#[test]
fn test_testdata_format_wrong_json_to_binary() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let output_state_file = dir.path().join("output_state_file.bin");
    let database_file = dir.path().join("database.bin");
    let paths = vec!["testdata"];
    let files = vec!["testdata/testfile-yes.bin"];
    let number_of_results = 5;

    let mut index_command = Command::cargo_bin("fbhash")?;
    index_command
        .arg("index")
        .arg("--state")
        .arg(output_state_file.clone())
        .arg(format!("--database={}", database_file.to_str().unwrap()))
        .arg(paths[0]);
    index_command.assert().success();

    let mut query_command = Command::cargo_bin("fbhash")?;
    query_command
        .arg("--binary")
        .arg("query")
        .arg(format!("-n={}", number_of_results))
        .arg("--database")
        .arg(database_file.clone())
        .arg("--state")
        .arg(output_state_file.clone())
        .arg(files[0]);

    #[cfg(not(target_os = "windows"))]
    query_command
        .assert()
        .failure()
        .stderr("memory allocation of 2308757952953217893 bytes failed\n");

    #[cfg(target_os = "windows")]
    query_command
        .assert()
        .failure()
        .stderr("memory allocation of 2308757952953217893 bytes failed\n");

    dir.close()?;
    Ok(())
}

#[test]
fn test_testdata_wrong_combo() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let output_state_file = dir.path().join("output_state_file.bin");
    let second_output_state_file = dir.path().join("second_output_state_file.bin");
    let database_file = dir.path().join("database.bin");
    let second_database_file = dir.path().join("second_database.bin");
    let paths = vec!["testdata"];
    let files = vec!["testdata/testfile-yes.bin"];
    let number_of_results = 5;

    let mut index_command = Command::cargo_bin("fbhash")?;
    index_command
        .arg("--binary")
        .arg("index")
        .arg("--state")
        .arg(output_state_file.clone())
        .arg("--database")
        .arg(database_file.to_str().unwrap())
        .arg(paths[0]);
    index_command.assert().success();

    let temp_path_str: Option<&str> = dir.path().to_str();
    if temp_path_str.is_none() {
        panic!("Cannot convert path {:?}", dir.path());
    }

    let mut second_index_command = Command::cargo_bin("fbhash")?;
    second_index_command
        .arg("--binary")
        .arg("index")
        .arg("--state")
        .arg(second_output_state_file.clone())
        .arg("--database")
        .arg(second_database_file.to_str().unwrap())
        .arg(temp_path_str.unwrap());
    second_index_command.assert().success();

    let mut query_command = Command::cargo_bin("fbhash")?;
    query_command
        .arg("--binary")
        .arg("query")
        .arg(format!("-n={}", number_of_results))
        .arg("--database")
        .arg(database_file.clone())
        .arg("--state")
        .arg(second_output_state_file.clone())
        .arg(files[0]);

    #[cfg(not(target_os = "windows"))]
    query_command.assert().failure().stderr(format!(
        "Error: Custom {{ kind: InvalidInput, error: \"{} and {} are not consistent\" }}\n",
        second_output_state_file.to_str().unwrap(),
        database_file.to_str().unwrap()
    ));

    #[cfg(target_os = "windows")]
    query_command.assert().failure().stderr(format!(
        "Error: Custom {{ kind: InvalidInput, error: \"{} and {} are not consistent\" }}\n",
        second_output_state_file
            .to_str()
            .unwrap()
            .replace('\\', "\\\\"),
        database_file.to_str().unwrap().replace('\\', "\\\\")
    ));

    dir.close()?;
    Ok(())
}
