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

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[cfg(test)]
#[macro_use]
extern crate float_cmp;

use clap::{App, Arg, SubCommand};
use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

mod chunker;
mod similarities;

fn get_files_from_dir(start_path: &str) -> Vec<String> {
    WalkDir::new(start_path)
        .follow_links(false)
        .into_iter()
        .map(|e| String::from(e.ok().unwrap().path().to_str().unwrap()))
        .filter(|name| Path::new(name).is_file())
        .collect()
}

fn index_directory(
    start_path: &str,
    document_collection: &RefCell<similarities::DocumentCollection>,
) -> std::io::Result<Vec<similarities::Document>> {
    let files: Vec<String> = get_files_from_dir(start_path);
    let mut results: Vec<similarities::Document> = Vec::new();
    for file_name in files {
        let mut dc = document_collection.borrow_mut();
        println!("Processing: {}", file_name);
        let added_file = dc.add_file(&file_name);
        match added_file {
            Err(_) => println!("Ignoring file {}", file_name),
            Ok(document) => {
                results.push(document.unwrap());
            }
        }
    }
    Ok(results)
}

fn index_paths(
    paths: &Vec<&str>,
    output_state_file: &str,
    results_file: &str,
) -> std::io::Result<()> {
    let document_collection = RefCell::new(similarities::DocumentCollection::new());

    let mut results: Vec<_> = Vec::new();
    for path in paths.iter() {
        results.append(&mut index_directory(path, &document_collection).unwrap());
    }

    println!("Output the frequencies state");
    let mut state_output = File::create(output_state_file)?;
    let doc_ref: &similarities::DocumentCollection = &(document_collection.borrow());
    state_output.write_all(serde_json::to_string_pretty(doc_ref).unwrap().as_bytes())?;

    println!("Updating statistics");

    let updated_results: Vec<similarities::Document> = results
        .iter()
        .map(|doc| similarities::Document {
            file: doc.file.to_string(),
            chunks: doc.chunks.clone(),
            digest: document_collection
                .borrow()
                .compute_document_digest(&doc.chunks),
        })
        .collect();

    println!("Output to file");

    // Now start serializing it to a json file.
    let mut output = File::create(results_file)?;
    for doc in updated_results {
        output.write_all(serde_json::to_string(&doc).unwrap().as_bytes())?;
        output.write_all(b"\n")?;
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let matches = App::new("fbhash")
        .version("0.1.0")
        .author("Erwin van Eijk")
        .about("Find near duplicates of files")
        .subcommand(
            SubCommand::with_name("index")
                .arg(
                    Arg::with_name("INPUT ...")
                        .required(true)
                        .index(1)
                        .help("Path to directories to process")
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("output_state")
                        .short("o")
                        .value_name("STATE_FILE")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("results_file")
                        .short("r")
                        .value_name("RESULTS_FILE")
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Some(subcommand_matches) = matches.subcommand_matches("index") {
        let paths: Vec<_> = subcommand_matches.values_of("INPUT").unwrap().collect();
        let output_state_file = subcommand_matches
            .value_of("STATE_FILE")
            .unwrap_or("collection_state.json");
        let results_file = subcommand_matches
            .value_of("RESULTS_FILE")
            .unwrap_or("results.json");

        index_paths(&paths, output_state_file, results_file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO:
    //   Move this to a separate testing toolkit?
    fn eq_lists<T>(a: &[T], b: &[T]) -> bool
    where
        T: PartialEq + Ord,
    {
        let mut a: Vec<_> = a.iter().collect();
        let mut b: Vec<_> = b.iter().collect();
        a.sort();
        b.sort();

        a == b
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_get_files_from_path() {
        let result = get_files_from_dir("testdata");
        assert!(eq_lists(
            &[
                String::from("testdata/testfile-zero-length"),
                String::from("testdata/testfile-yes.bin"),
                String::from("testdata/testfile-zero.bin"),
            ],
            &result[..]
        ));
        assert_eq!(result.len(), 3);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_files_from_path() {
        let result: Vec<String> = get_files_from_dir("testdata");
        assert!(eq_lists(
            &[
                String::from("testdata\\testfile-yes.bin"),
                String::from("testdata\\testfile-zero-length"),
                String::from("testdata\\testfile-zero.bin"),
            ],
            &result[..]
        ));
        assert_eq!(result.len(), 3);
    }
}
