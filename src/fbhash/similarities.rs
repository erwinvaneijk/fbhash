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

extern crate frequency;
extern crate frequency_hashmap;

use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;
use hash_hasher::HashBuildHasher;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, BTreeMap};
use hashbrown::HashMap;
use std::fs::File;
use std::io;

use crate::fbhash::chunker::ChunkIterator;
use crate::fbhash::heap::Heap;

pub fn file_to_chunks(file: File) -> Vec<u64> {
    let chunk_iterator = ChunkIterator::new(file);
    let chunks: Vec<u64> = chunk_iterator.into_iter().map(|e| e.digest).collect();
    chunks
}

pub fn compute_document_frequencies(doc: &[u64]) -> BTreeMap<&u64, usize> {
    let mut hmf : BTreeMap<&u64, usize> = BTreeMap::new();
    for chunk in doc {
        hmf.entry(&chunk).and_modify(|e| {*e += 1 }).or_insert(1);
    }
    hmf
}

pub fn compute_document(file_name: &str) -> io::Result<(Document, HashMap<u64, usize>)> {
    let file = File::open(file_name)?;
    let chunks = file_to_chunks(file);
    let mut file_frequencies: HashMap<u64, usize> = HashMap::new();
    for chunk in chunks.clone() {
        file_frequencies.entry(chunk).and_modify(|e| {*e += 1}).or_insert(1);
    }
    
    let doc = Document {
        file: file_name.to_string(),
        chunks,
        digest: vec![],
    };
    Ok((doc, file_frequencies))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub file: String,
    pub chunks: Vec<u64>,
    pub digest: Vec<(u64, f64)>,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
    }
}

impl Eq for Document {}

impl std::hash::Hash for Document {
    fn hash<H>(&self, h: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.file.hash(h);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentCollection {
    files: BTreeSet<String>,
    // Ordering is important, as it determines the order of the digests that
    // are created for the file contents.
    // Using HashBuildHasher enables that insertion order stays the same.
    // TODO:
    // Determine if it's more beneficial to replace it with a strictly ordered data structure
    // in the first place, and take the overhead as a calculated downside.
    collection_digests: BTreeMap<u64, usize>,
}

impl DocumentCollection {
    pub fn new() -> DocumentCollection {
        DocumentCollection {
            files: BTreeSet::new(),
            collection_digests: BTreeMap::default()
        }
    }

    pub fn copy(&self) -> DocumentCollection {
        DocumentCollection {
            files: self.files.clone(),
            collection_digests: self.collection_digests.clone()
        }
    }
 
    pub fn extend(&mut self, other: &DocumentCollection) {
        self.files.extend(other.files.iter().cloned());
        for (k, v) in &other.collection_digests {
            self.collection_digests.entry(*k).and_modify(|e| {*e += v}).or_insert(*v);
        }
    }

    pub fn add_file(&mut self, name: &str) -> io::Result<Option<Document>> {
        if !self.exists_file(name) {
            match compute_document(name) {
                Ok((document, file_frequencies)) => {
                    // Update internal state.
                    for (k, v) in file_frequencies {
                        self.collection_digests.entry(k).and_modify(|e| {*e += v}).or_insert(v);
                    }
                    self.files.insert(name.to_string());
                    Ok(Some(document))
                },
                Err(v) => Err(v)
            }
        } else {
            Ok(None)
        }
    }

    pub fn update_collection(&mut self, frequencies: &HashMap<u64, usize>, names: &[String]) -> usize {
        for (k, v) in frequencies.iter() {
            self.collection_digests.entry(*k).and_modify(|e| {*e += v}).or_insert(*v);
        }
        self.files.extend(names.iter().cloned());
        self.collection_digests.len()
    }

    pub fn exists_file(&self, name: &str) -> bool {
        self.files.contains(name)
    }

    pub fn compute_digest(&self, name: &str) -> io::Result<Vec<(u64, f64)>> {
        let file = File::open(name)?;
        let document: Vec<u64> = file_to_chunks(file);
        Ok(self.compute_document_digest(&document))
    }

    fn compute_chunk_weight(&self, chunk: u64, frequency: usize) -> Option<f64> {
        if frequency == 0 {
            None
        } else {
            let entry = self.collection_digests.get(&chunk);
            match entry {
                None => None,
                Some(value) => {
                    let count = *value as f64;
                    if *value > 0 && frequency > 0 {
                        let n = self.collection_digests.len() as f64;
                        let doc_weight = (n / count).log10();
                        // Avoid getting infinity as an answer, as it will not serialize well with json
                        let chunk_weight = (1.0_f64 + frequency as f64).log10();
                        Some(doc_weight * chunk_weight)
                    } else {
                        None
                    }
                }
            }
        }
    }

    pub fn compute_document_digest(&self, doc: &[u64]) -> Vec<(u64, f64)> {
        // The following is correct according to the paper
        // let frequencies = compute_document_frequencies(doc.clone());
        // doc.into_iter().map(|chunk| self.compute_chunk_weight(chunk, frequencies.count(&chunk))).collect()
        // This is correct according to my understanding of how TF/IDF works.
        // Because hashed_doc gets a BTreeMap, it is in sorted order. And because of that
        // it will also have the same order as the internal state chunks.
        let hashed_doc = compute_document_frequencies(doc);
        let digest = hashed_doc.iter().map(|(chunk, count)|
            (chunk, self.compute_chunk_weight(**chunk, *count))
            )
            .filter(|(_, v)| v.is_some()).map(|(k, v)| (**k, v.unwrap())).collect();
        digest
    }
}

impl PartialEq for DocumentCollection {
    fn eq(&self, other: &Self) -> bool {
        // First check if other contains the same files.
        let ret: bool = self
            .files
            .iter()
            .zip(other.files.iter())
            .fold(true, |sum, (a, b)| sum && (a == b));
        if ret {
            // Now check if the contents of the digests is the same.
            self.collection_digests
                .iter()
                .zip(other.collection_digests.iter())
                .fold(true, |acc, (a, b)| acc && (a == b))
        } else {
            ret
        }
    }
}

impl Eq for DocumentCollection {}

impl std::hash::Hash for DocumentCollection {
    fn hash<H>(&self, h: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.files.iter().for_each(|file| file.hash(h));
        self.collection_digests.iter().for_each(|(k, v)| {
            k.hash(h);
            v.hash(h);
        })
    }
}

pub fn ranked_search(doc: &[(u64, f64)], documents: &[Document], k: usize) -> Vec<(f64, Document)> {
    let mut queue = Heap::new(k);
    documents
        .iter()
        .map(|other_doc| (other_doc, cosine_distance(&other_doc.digest, doc)))
        .for_each(|(d, score)| {
            let _ = queue.insert(score, d);
        });
    let mut result = Vec::new();
    for i in queue.get_elements() {
        result.push((i.0, i.1.clone()));
    }
    result
}

//
// Compute the cosine similarity between these two vectors,
// it is assumed that the index in the vectors is sorted
//
pub fn cosine_similarity(vec1: &[(u64, f64)], vec2: &[(u64, f64)]) -> f64 {
    if vec1.is_empty() || vec2.is_empty() {
        return if vec1.len() == vec1.len() { 0. } else { 1. };
    }
    let mut coll: BTreeMap<u64, (Option<f64>, Option<f64>)> 
        = vec1.iter().map(|(k, v)| (*k, (Some(*v), None))).collect();
    
    vec2.iter().for_each(|(k, v)| {
        coll.entry(*k)
            .and_modify(|e| e.1 = Some(*v)).or_insert((None, Some(*v)));
    });

    let (norm_a, norm_b, norm_prod)  =
    coll.values().fold(
        (0_f64, 0_f64, 0_f64),
        |(norm_a, norm_b, norm_prod), (n1, n2)| {
            if n1.is_some() && n2.is_some() {
                (norm_a + n1.unwrap() * n1.unwrap(), norm_b + n2.unwrap() * n2.unwrap(), norm_prod + n1.unwrap() * n2.unwrap())
            } else if n1.is_some() {
                (norm_a + n1.unwrap() * n1.unwrap(), norm_b , norm_prod)
            } else {
                (norm_a, norm_b + n2.unwrap() * n2.unwrap() , norm_prod)
            }
        }
    );
    norm_prod / (norm_a.sqrt() * norm_b.sqrt())
}

pub fn cosine_distance(vec1: &[(u64, f64)], vec2: &[(u64, f64)]) -> f64 {
    1.0_f64 - cosine_similarity(vec1, vec2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fbhash::similarities::cosine_similarity;
    use frequency::Frequency;
    use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};
    use std::fs::File;
    use std::io;

    fn compute_file_frequencies(file: File) -> HashMapFrequency<u64> {
        let mut frequency_map: HashMapFrequency<u64> = HashMapFrequency::new();

        // This is a roundabout way, because HashMapFrequency needs &u64
        file_to_chunks(file)
            .into_iter()
            .for_each(|e| frequency_map.increment(e));

        frequency_map
    }

    fn compute_scores_from_frequencies(freq_map: &HashMapFrequency<u64>) -> HashMap<u64, f64> {
        freq_map
            .into_iter()
            .map(|(k, v)| (*k, 1.0 + (*v as f64).log10()))
            .collect()
    }

    fn compute_document_scores(file: File) -> HashMap<u64, f64> {
        let frequencies = compute_file_frequencies(file);
        compute_scores_from_frequencies(&frequencies)
    }

    #[test]
    fn test_compute_document_frequencies() -> io::Result<()> {
        let f = File::open("testdata/testfile-yes.bin")?;
        let m = compute_file_frequencies(f);

        assert_eq!(m.is_empty(), false);
        assert_eq!(m.len(), 2);
        assert_eq!(m.count(&2879926931474365), 253);
        assert_eq!(m.count(&33279275454869446), 253);
        Ok(())
    }

    #[test]
    fn test_compute_document_scores() -> io::Result<()> {
        let f = File::open("testdata/testfile-yes.bin")?;
        let m = compute_document_scores(f);

        assert_eq!(m.is_empty(), false);
        assert_eq!(m.len(), 2);
        match m.get(&2879926931474365) {
            Some(score) => {
                let expected_score: f64 = 1.0 + (253.0_f64).log10();
                assert!(approx_eq!(f64, *score, expected_score, epsilon = 0.001))
            }
            None => panic!("This value should exist"),
        }

        Ok(())
    }

    fn construct_expected_vec() -> Vec<u64> {
        (0..506)
            .map(|i| {
                if i % 2 == 0 {
                    33279275454869446_u64
                } else {
                    2879926931474365_u64
                }
            })
            .collect()
    }

    #[test]
    fn test_document_collection() -> io::Result<()> {
        let name = String::from("testdata/testfile-yes.bin");
        let mut document_collection = DocumentCollection::new();
        let result = document_collection.add_file(&name);
        let expected_vec = construct_expected_vec();
        assert!(result.is_ok(), "We should get a document back.");
        let unpacked_result = result.unwrap();
        assert!(unpacked_result.is_some());
        assert_eq!(unpacked_result.unwrap().chunks, expected_vec);
        assert!(document_collection.exists_file(&name));
        let again_result = document_collection.add_file(&name);
        assert!(again_result.is_ok(), "We should get the option back.");
        assert_eq!(again_result.unwrap(), None);
        assert!(!document_collection.collection_digests.is_empty());
        let doc_vector = document_collection.compute_digest(&name)?;
        assert_eq!(doc_vector.len(), 2);
        Ok(())
    }

    #[test]
    fn test_cosine_distance() {
        let vec1 = vec![(0, -1.0), (1, 0.1), (2, 0.2)];
        let vec2 = vec![(0, 1.0), (1, -0.1), (2, -0.2)];
        assert!(approx_eq!(
            f64,
            cosine_similarity(&vec1, &vec1),
            1.0_f64,
            ulps = 2
        ));
        assert!(approx_eq!(
            f64,
            cosine_similarity(&vec1, &vec2),
            -1.0_f64,
            ulps = 2
        ))
    }

    #[test]
    fn test_serialization_of_document() -> io::Result<()> {
        let name = String::from("testdata/testfile-yes.bin");
        let mut document_collection = DocumentCollection::new();
        let chunks = document_collection.add_file(&name)?.unwrap().chunks;
        let doc_vector = document_collection.compute_digest(&name)?;
        let doc = Document {
            file: name,
            chunks,
            digest: doc_vector,
        };

        assert_tokens(
            &doc,
            &[
                Token::Struct {
                    name: "Document",
                    len: 3,
                },
                Token::String("file"),
                Token::String("testdata/testfile-yes.bin"),
                Token::String("chunks"),
                Token::Seq { len: Some(506) },
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::U64(33279275454869446),
                Token::U64(2879926931474365),
                Token::SeqEnd,
                Token::Str("digest"),
                Token::Seq { len: Some(2) },
                Token::Tuple { len: 2},
                Token::U64(2879926931474365),
                Token::F64(-5.055178171138189),
                Token::TupleEnd,
                Token::Tuple { len: 2},
                Token::U64(33279275454869446),
                Token::F64(-5.055178171138189),
                Token::TupleEnd,
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );

        Ok(())
    }

    #[test]
    fn test_serialization_document_set_state() {
        let names = vec!["testdata/testfile-yes.bin", "testdata/testfile-zero.bin"];
        let mut document_collection = DocumentCollection::new();
        let _ = document_collection.add_file(names[0]);
        let _ = document_collection.add_file(names[1]);

        println!(
            "{}",
            serde_json::to_string_pretty(&document_collection).unwrap()
        );
        assert_ser_tokens(
            &document_collection,
            &[
                Token::Struct {
                    name: "DocumentCollection",
                    len: 2,
                },
                Token::String("files"),
                Token::Seq { len: Some(2) },
                Token::String("testdata/testfile-yes.bin"),
                Token::String("testdata/testfile-zero.bin"),
                Token::SeqEnd,
                Token::Str("collection_digests"),
                Token::Map { len: Some(3) },
                Token::U64(0),
                Token::U64(506),
                Token::U64(2879926931474365),
                Token::U64(253),
                Token::U64(33279275454869446),
                Token::U64(253),
                Token::MapEnd,
                Token::StructEnd,
            ],
        );
        assert_de_tokens(
            &document_collection,
            &[
                Token::Struct {
                    name: "DocumentCollection",
                    len: 2,
                },
                Token::String("files"),
                Token::Seq { len: Some(2) },
                Token::String("testdata/testfile-yes.bin"),
                Token::String("testdata/testfile-zero.bin"),
                Token::SeqEnd,
                Token::Str("collection_digests"),
                Token::Map { len: Some(3) },
                Token::U64(2879926931474365),
                Token::U64(253),
                Token::U64(33279275454869446),
                Token::U64(253),
                Token::U64(0),
                Token::U64(506),
                Token::MapEnd,
                Token::StructEnd,
            ],
        );
    }
}
