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

use crate::fbhash::similarities::*;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

pub fn query_for_results(
    state_path: &str,
    database_path: &str,
    files: &[&str],
    number_of_results: usize,
) -> std::result::Result<(), std::io::Error> {
    println!("Reading database: {}", state_path);
    let state_file = File::open(state_path)?;
    let document_collection: DocumentCollection = serde_json::from_reader(state_file)?;

    println!("Reading the database with the files: {}", database_path);
    let file = BufReader::new(File::open(database_path)?);
    let mut documents: Vec<Document> = Vec::new();
    for line in file.lines() {
        match line {
            Ok(ok_line) => {
                let doc: Document = serde_json::from_str(ok_line.as_str()).unwrap();
                documents.push(doc);
            }
            Err(v) => panic!(v),
        }
    }

    for file_name in files {
        let document = document_collection.compute_digest(file_name).ok().unwrap();
        let results = ranked_search(&document, &documents, number_of_results);
        println!("Results: {}", results.len());
        for result in &results {
            println!("{} => ({}) {}", file_name, result.0, result.1.file);
        }
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
