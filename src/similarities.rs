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

use std::io;
use std::fs::File;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;

use crate::chunker::ChunkIterator;

pub fn file_to_chunks(file: File) -> Vec<u64> {
    let chunk_iterator = ChunkIterator::new(file);
    let chunks: Vec<u64> = chunk_iterator.into_iter().map(|e| e.digest).collect();
    chunks
}

pub fn compute_file_frequencies(file: File) -> HashMapFrequency<u64> {
    let mut frequency_map: HashMapFrequency<u64> = HashMapFrequency::new();

    // This is a roundabout way, because HashMapFrequency needs &u64
    file_to_chunks(file).into_iter().for_each(|e| frequency_map.increment(e));

    frequency_map
}

pub fn compute_document_frequencies(doc: &[u64]) -> HashMapFrequency<&u64> {
    let hmf : HashMapFrequency<&u64> = doc.iter().collect();
    hmf
}

pub fn compute_document_scores(file: File) -> HashMap<u64, f64> {
    let frequencies = compute_file_frequencies(file);
    compute_scores_from_frequencies(&frequencies)
}

fn compute_scores_from_frequencies(freq_map: &HashMapFrequency<u64>) -> HashMap<u64, f64> {
    freq_map.into_iter().map(|(k, v)| (*k, 1.0 + (*v as f64).log10())).collect()
}

#[derive(Clone, Debug)]
pub struct Document {
    pub file: String,
    pub chunks: Vec<u64>,
    pub digest: Vec<f64>
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
    }
}

impl Eq for Document {}

impl std::hash::Hash for Document {

fn hash<H>(&self, h: &mut H) where H: std::hash::Hasher {
        return self.file.hash(h)
    }
}

pub struct DocumentCollection {
    files: HashMap<String, Document>,
    collection_digests: HashMapFrequency<u64>,
}

impl DocumentCollection {
    pub fn new() -> DocumentCollection {
        DocumentCollection {
            files: HashMap::new(),
            collection_digests: HashMapFrequency::new()
        }
    }

    pub fn add_file(&mut self, name: &str) -> io::Result<&Document> {
        match self.files.entry(name.to_string()) {
            Entry::Occupied(o) => Ok(o.into_mut()),
            Entry::Vacant(v) => {
                let file = File::open(name)?;
                let chunks = file_to_chunks(file);
                for chunk in chunks.clone() {
                    self.collection_digests.increment(chunk);
                }
                let doc = Document{ file: name.to_string(), chunks, digest: vec![]};
                Ok(v.insert(doc))
            }
        }
    }

    pub fn compute_digest(&mut self, name: &str) -> io::Result<Vec<f64>> {
        let file = File::open(name)?;
        let document: Vec<u64> = file_to_chunks(file);
        Ok(self.compute_document_digest(&document))
    }

    fn compute_chunk_weight(&self, chunk: u64, frequency: usize) -> f64 {
        let n = self.collection_digests.len() as f64;
        let count = self.collection_digests.count(&chunk) as f64;
        let doc_weight = if count > 0.0 { (n/count).log10() } else { 1.0_f64 };
        let chunk_weight = 1.0_f64 + (frequency as f64).log10();
        doc_weight * chunk_weight
    }

    pub fn compute_document_digest(&self, doc: &[u64]) -> Vec<f64> {
        // The following is correct according to the paper
        // let frequencies = compute_document_frequencies(doc.clone());
        // doc.into_iter().map(|chunk| self.compute_chunk_weight(chunk, frequencies.count(&chunk))).collect()
        // This is correct according to my understanding of how TF/IDF works.
        let hashed_doc = compute_document_frequencies(doc);
        return self.collection_digests.items()
            .map(|known_chunk| {self.compute_chunk_weight(*known_chunk, hashed_doc.count(&known_chunk))})
            .collect();
    }
}

pub fn cosine_similarity(vec1: &[f64], vec2: &[f64]) -> f64 {
    if vec1.is_empty() || vec2.is_empty() {
        return if vec1.len() == vec1.len() { 0. } else {1.};
    }
    let iter_a = vec1.iter();
    let iter_b = vec2.iter();
    let (norm_a, norm_b, norm_prod) = iter_a.zip(iter_b).into_iter().fold(
        (0_f64, 0_f64, 0_f64),
        |(norm_a, norm_b, norm_prod), (n1, n2)| {
            (norm_a + n1 * n1, norm_b + n2 * n2, norm_prod + n1 * n2)
        },
    );
    norm_prod / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::similarities::compute_document_scores;
    use crate::similarities::compute_file_frequencies;
    use crate::similarities::cosine_similarity;
    use std::io;
    use std::fs::File;
    use frequency::Frequency;

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
            },
            None => panic!("This value should exist")
        }

        Ok(())
    }

    fn construct_expected_vec() -> Vec<u64> {
        (0..506).map(|i|{ if i%2 == 0 {33279275454869446_u64} else {2879926931474365_u64} }).collect()
    }

    #[test]
    fn test_document_collection() -> io::Result<()> {
        let name = String::from("testdata/testfile-yes.bin");
        let mut document_collection = DocumentCollection::new();
        let result = document_collection.add_file(&name);
        let expected_vec = construct_expected_vec();
        assert_eq!(result.unwrap().chunks, expected_vec);
        assert_eq!(document_collection.files[&name].file, name);
        let again_result = document_collection.add_file(&name);
        assert_eq!(again_result.unwrap().chunks, expected_vec);
        assert!(!document_collection.collection_digests.is_empty());
        let doc_vector = document_collection.compute_digest(&name)?;
        assert_eq!(doc_vector.len(), 2);
        Ok(())
    }

    #[test]
    fn test_cosine_distance() {
        let vec1 = vec![0.0, 0.1, 0.2];
        let vec2 = vec![0.0, -0.1, -0.2];
        assert!(approx_eq!(f64, cosine_similarity(&vec1, &vec1), 1.0_f64, ulps=2));
        assert!(approx_eq!(f64, cosine_similarity(&vec1, &vec2), -1.0_f64, ulps=2))
    }
}
