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
        .arg("-o")
        .arg(output_state_file.clone())
        .arg(format!("--database={}", database_file.to_str().unwrap()))
        .arg(paths[0]);
    index_command.assert().success();

    let mut query_command = Command::cargo_bin("fbhash")?;
    query_command
        .arg("query")
        .arg(format!("-n={}", number_of_results))
        .arg(database_file.clone())
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
