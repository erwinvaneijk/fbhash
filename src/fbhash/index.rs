// Copyright 2021 -- 2023 Erwin van Eijk
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

use console::style;
use hashbrown::HashMap;
use std::cell::RefCell;
use std::convert::TryInto;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use indicatif::{ProgressBar, ProgressIterator};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::RwLock;
use walkdir::WalkDir;

use crate::fbhash::similarities::*;
use crate::fbhash::utils::*;

fn get_files_from_dir(start_path: &PathBuf) -> Vec<PathBuf> {
    WalkDir::new(start_path)
        .follow_links(false)
        .into_iter()
        .map(|e| e.ok().unwrap().path().to_owned())
        .filter(|path_name| path_name.is_file())
        .collect()
}

// Write the results to a binary file.
fn write_database_state_binary(results: &[Document], results_file: &PathBuf) -> io::Result<()> {
    let output = File::create(results_file)?;
    bincode::serialize_into(output, &results).unwrap();
    Ok(())
}

fn write_database_state(
    updated_results: &[Document],
    results_file: &PathBuf,
    config: &Configuration,
) -> io::Result<()> {
    match config.output_format {
        OutputFormat::Binary => write_database_state_binary(updated_results, results_file),
        OutputFormat::Json => {
            let final_progress =
                create_progress_bar(updated_results.len().try_into().unwrap(), config);
            let mut output = File::create(results_file)?;
            let errors: Vec<io::Result<()>> = updated_results
                .iter()
                .progress_with(final_progress)
                .map(|doc| {
                    output.write_all(serde_json::to_string(&doc).unwrap().as_bytes())?;
                    output.write_all(b"\n")?;
                    Ok(())
                })
                .filter(|e| e.is_err())
                .collect();
            if errors.is_empty() {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    errors[0].as_ref().err().unwrap().to_string(),
                ))
            }
        }
    }
}

fn index_directory(
    start_path: &PathBuf,
    document_collection: &RefCell<DocumentCollection>,
    config: &Configuration,
) -> Vec<Document> {
    let files: Vec<PathBuf> = get_files_from_dir(start_path);
    let number_of_files: u64 = files.len().try_into().unwrap();

    let pb = create_progress_bar(number_of_files, config);
    type HashSender = Sender<(HashMap<u64, usize>, String)>;
    type HashReceiver = Receiver<(HashMap<u64, usize>, String)>;
    let (sender, receiver): (HashSender, HashReceiver) = mpsc::channel();
    let results: Vec<Document> = files
        .par_iter()
        .map_with(sender, |s, file_path| {
            match compute_document(&file_path.to_string_lossy()) {
                Ok((document, file_frequencies)) => {
                    pb.inc(1);
                    s.send((file_frequencies, document.file.clone())).unwrap();
                    Some(document)
                }
                Err(_) => None,
            }
        })
        .fold(Vec::new, |mut left, right: Option<Document>| {
            if let Some(value) = right {
                left.push(value);
            }
            left
        })
        .reduce(Vec::new, |mut left, right| {
            left.extend(right);
            left
        });
    pb.finish_and_clear();

    if !config.quiet {
        println!(
            "{} Updating the internal dictionary...",
            style("[2/5]").bold().dim()
        );
    }

    let new_pb = create_progress_bar(number_of_files, config);
    let mut dc = document_collection.borrow_mut();
    receiver
        .iter()
        .progress_with(new_pb)
        .for_each(|(hash, name)| {
            dc.update_collection(&hash, &[name]);
        });
    pb.finish_and_clear();
    results
}

pub fn index_paths(
    paths: &[&PathBuf],
    output_state_file: &PathBuf,
    results_file: &PathBuf,
    config: &Configuration,
) -> io::Result<()> {
    let document_collection = RefCell::new(DocumentCollection::new());

    if !config.quiet {
        println!(
            "{} Processing paths to process...",
            style("[1/5]").bold().dim()
        );
    }

    let mut results: Vec<_> = Vec::new();
    for path in paths.iter() {
        let mut intermediate_results = index_directory(path, &document_collection, config);
        results.append(&mut intermediate_results);
    }

    if !config.quiet {
        println!(
            "{} Output the frequencies state...",
            style("[3/5]").bold().dim()
        );
    }

    let mut state_output = File::create(output_state_file)?;
    let doc_ref: &DocumentCollection = &(document_collection.borrow());
    match config.output_format {
        OutputFormat::Json => {
            state_output.write_all(serde_json::to_string_pretty(doc_ref).unwrap().as_bytes())?
        }
        OutputFormat::Binary => bincode::serialize_into(state_output, doc_ref).unwrap(),
    }

    if !config.quiet {
        println!("{} Updating statistics...", style("[4/5]").bold().dim());
    }

    let progress_bar: ProgressBar = create_progress_bar(results.len().try_into().unwrap(), config);
    let reference_collection = document_collection.borrow().copy();
    let document_collection_mutex = RwLock::new(reference_collection);
    let updated_results: Vec<Document> = results
        .into_par_iter()
        .map(|doc| {
            let the_collection = document_collection_mutex.read().unwrap();
            progress_bar.inc(1);
            Document {
                file: doc.file.to_string(),
                chunks: Vec::new(), // Remove the old chunks, we don't need them anymore
                digest: the_collection.compute_document_digest(&doc.chunks),
            }
        })
        .collect();
    progress_bar.finish_and_clear();

    if !config.quiet {
        println!(
            "{} Output file database to {}",
            console::style("[5/5]").bold().dim(),
            results_file.to_str().expect("Valid filename")
        );
    }

    progress_bar.finish_and_clear();
    write_database_state(&updated_results, results_file, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

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
        let test_path = PathBuf::from("testdata");
        let result = get_files_from_dir(&test_path);
        assert!(eq_lists(
            &[
                Path::new("testdata/testfile-zero-length").to_owned(),
                Path::new("testdata/testfile-yes.bin").to_owned(),
                Path::new("testdata/testfile-zero.bin").to_owned(),
            ],
            &result[..]
        ));
        assert_eq!(result.len(), 3);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_files_from_path() {
        let test_path = PathBuf::from("testdata");
        let result = get_files_from_dir(&test_path);
        assert!(eq_lists(
            &[
                Path::new("testdata\\testfile-yes.bin").to_owned(),
                Path::new("testdata\\testfile-zero-length").to_owned(),
                Path::new("testdata\\testfile-zero.bin").to_owned(),
            ],
            &result[..]
        ));
        assert_eq!(result.len(), 3);
    }
}
