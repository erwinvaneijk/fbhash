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

use hashbrown::HashSet;
use std::cmp::Ordering;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

use crate::fbhash::utils::*;
use crate::fbhash::similarities::*;

fn read_database_in_json<R: BufRead>(file: &mut R, expected_files: usize) -> Result<Vec<Document>, std::io::Error> {
    let progress_bar = create_progress_bar(expected_files as u64);
    let mut documents: Vec<Document> = Vec::new();
    for line in file.lines() {
        match line {
            Ok(ok_line) => {
                let doc: Document = serde_json::from_str(ok_line.as_str()).unwrap();
                documents.push(doc.clone());
                progress_bar.inc(1);
                progress_bar.set_message(format!("{:?}", doc.file.as_str()));
            }
            Err(v) => panic!("{}", v),
        }
    }
    progress_bar.finish_and_clear();
    Ok(documents)
}

fn read_database_binary<R: BufRead>(file: &mut R, expected_files: usize) -> Result<Vec<Document>, std::io::Error> {
    let progress_bar = create_progress_bar(expected_files as u64);
    let documents: Vec<Document> = bincode::deserialize_from(progress_bar.wrap_read(file)).unwrap();
    progress_bar.finish_and_clear();
    Ok(documents)
}

fn verify_consistency(_document_collection: &DocumentCollection, _documents: &[Document]) -> bool {
    let document_name_set: HashSet<String> = _documents.iter().map(|d| d.file.clone()).collect();
    let all_collection_in_documents = _document_collection.get_files().iter().all(|f| {
        document_name_set.contains(f)
    });
    let all_documents_in_collection = _documents.iter().all(|d| {
        _document_collection.exists_file(&d.file)
    });
    all_collection_in_documents && all_documents_in_collection
}

fn open_state_and_database(
    state_path: &str,
    database_path: &str,
    output_format: OutputFormat
) -> Result<(DocumentCollection, Vec<Document>), std::io::Error> {
    let state_file = File::open(state_path)?;
    let progress_bar = create_progress_bar(state_file.metadata()?.len());
    progress_bar.println(format!("Reading database: {}", state_path));
    let document_collection: DocumentCollection = 
        match output_format {
            OutputFormat::Json =>
                serde_json::from_reader(&mut progress_bar.wrap_read(state_file))?,
            OutputFormat::Binary =>
                bincode::deserialize_from(&mut progress_bar.wrap_read(state_file)).unwrap()
        };
    progress_bar.finish_and_clear();

    if console::user_attended() {
        println!("Reading the database with the files: {}", database_path);
    }
    let inner_file = File::open(database_path)?;
    let expected_length = inner_file.metadata()?.len();
    let mut file = BufReader::new(inner_file);
    let documents: Vec<Document> =
        match output_format {
            OutputFormat::Json =>
                read_database_in_json(&mut file, document_collection.number_of_files())?,
            OutputFormat::Binary =>
                read_database_binary(&mut file, expected_length as usize)?
        };
    if ! verify_consistency(&document_collection, &documents) {
        Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{} and {} are not consistent", state_path, database_path)))
    } else {
        Ok((document_collection, documents))
    }
}

pub fn query_for_results(
    state_path: &str,
    database_path: &str,
    files: &[&str],
    number_of_results: usize,
    output_format: OutputFormat
) -> std::result::Result<(), std::io::Error> {
    let (document_collection, documents) = open_state_and_database(state_path, database_path, output_format)?;
    for file_name in files {
        let document = document_collection.compute_digest(file_name).ok().unwrap();
        let progress_bar = create_progress_bar(document_collection.number_of_files() as u64);
        progress_bar.println("Compute the files that are most similar in the set");
        let mut results = ranked_search(&document, &documents, number_of_results, &progress_bar);
        // For better testing purposes, the result is sorted by priority, file,
        // so the output can be predictable.
        results.sort_by(|a, b| {
            if a.0 < b.0 {
                Ordering::Less
            } else if a.0 > b.0 {
                Ordering::Greater
            } else {
                a.1.file.cmp(&b.1.file)
            }
        });
        progress_bar.finish_and_clear();
        println!("Similarities for {}", file_name);
        println!("Results: {}", results.len());
        for result in &results {
            println!("{} => ({}) {}", file_name, result.0, result.1.file);
        }
        println!();
    }

    Ok(())
}

#[cfg(tests)]
mod tests {
    #[test]
    fn test_deserialization_from_string() {
        let s = "{\"file\":\"testdata/testfile-zero.bin\",\"chunks\":[],\"digest\":[-8.252427688355256,null,null]}";
        let doc: Document = serde_json::from_slice(s);
        assert_eq!("testdata/testfile-zero.bin", doc.file);
        assert!(approx_eq(f64, -8.252427, doc.digest[0], epsilon = 0.0));
    }
}
