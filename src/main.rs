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

use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use std::cell::RefCell;
use walkdir::{WalkDir, DirEntry};

mod chunker;
mod similarities;

fn is_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
}

/*
fn get_files_from_dir(start_path: &str) -> Vec<String> {
    WalkDir::new(start_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| is_file(e))
        .map(|e| String::from(e.ok().unwrap().path().to_str().unwrap()))
        .collect()
}
*/

fn get_files_from_dir(start_path: &str) -> Vec<String> {
    WalkDir::new(start_path)
        .follow_links(false)
        .into_iter()
        .map(|e| String::from(e.ok().unwrap().path().to_str().unwrap()))
        .filter(|name| Path::new(name).is_file())
        .collect()
}


fn main() -> std::io::Result<()> {
    let document_collection = RefCell::new(similarities::DocumentCollection::new());
    let files: Vec<String> = get_files_from_dir(".");
    let mut results: Vec<similarities::Document> = Vec::new();
    for file_name in files.iter().take(100) {
        let mut dc = document_collection.borrow_mut();
        println!("Processing: {}", file_name);
        let added_file = dc.add_file(&file_name);
        match added_file {
            Err(_) =>
              println!("Ignoring file {}", file_name),
            Ok(document) =>
            {
                results.push(document.clone())
            }
        }
    }

    println!("Updating statistics");

    let updated_results: Vec<similarities::Document> = results.iter().map(|doc| similarities::Document{file: doc.file.to_string(), chunks: doc.chunks.clone(), digest: document_collection.borrow().compute_document_digest(&doc.chunks)}).collect();

    println!("Output to file");

    // Now start serializing it to a json file.
    let mut output = File::create("total.json")?;
    for doc in updated_results {
        output.write_all(serde_json::to_string(&doc).unwrap().as_bytes())?;
        output.write_all(b"\n")?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_files_from_path() {
        let result = get_files_from_dir("testdata");
        assert_eq!(result, vec!["testdata/testfile-zero.bin", "testdata/testfile-yes.bin", "testdata/testfile-zero-length"]);
        assert_eq!(result.len(), 3);

    }
}