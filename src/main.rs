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

use std::cell::RefCell;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;
use walkdir::WalkDir;

mod chunker;
mod similarities;

fn ranked_search<'a>(doc: &[f64], documents: &'a [similarities::Document], _: usize) -> Vec<&'a similarities::Document> {
    let mut queue: PriorityQueue<&similarities::Document, OrderedFloat<f64>> = PriorityQueue::new();
    documents.iter().map(|other_doc| (other_doc, similarities::cosine_similarity(&other_doc.digest, doc))).for_each(|(d, score)| { let _ = queue.push(d, OrderedFloat::from(score)); });
    let mut v = queue.into_sorted_vec();
    v.reverse();
    v
}

fn main() {
    let document_collection = RefCell::new(similarities::DocumentCollection::new());
    let files: Vec<String> = WalkDir::new(".").follow_links(false).into_iter().map(|e| String::from(e.ok().unwrap().path().to_str().unwrap())).collect();
    let mut results: Vec<similarities::Document> = Vec::new();
    for file_name in files {
        let mut dc = document_collection.borrow_mut();
        let added_file = dc.add_file(&file_name);
        match added_file {
            Err(_) =>
              println!("Ignoring file {}", file_name),
            Ok(document) =>
            {
                println!("Adding document: {}", document.file);
                results.push(document.clone())
            }
        }
    }

    let similarity_matches = ranked_search(&results[1].digest, &results, 2);
    similarity_matches.iter().for_each(|sim| println!("{}", sim.file));
}
