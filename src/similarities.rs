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


use std::fs::File;
use std::collections::HashMap;
use frequency::Frequency;
use frequency_hashmap::HashMapFrequency;

use crate::chunker::ChunkIterator;

pub fn compute_document_frequencies(file: File) -> HashMapFrequency<u64> {
    let chunk_iterator = ChunkIterator::new(file);
    let chunks: Vec<_> = chunk_iterator.collect();
    let mut frequency_map: HashMapFrequency<u64> = HashMapFrequency::new();

    chunks.into_iter().for_each(|e| frequency_map.increment(e.digest));

    frequency_map
}

pub fn compute_document_scores(file: File) -> HashMap<u64, f64> {
    let frequencies = compute_document_frequencies(file);
    let scores: HashMap<u64, f64> = frequencies.into_iter().map(|(k, v)| (k.clone(), 1.0 + (*v as f64).log10())).collect();
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::similarities::compute_document_scores;
    use crate::similarities::compute_document_frequencies;
    use std::io;
    use std::fs::File;
    use frequency::Frequency;

    #[test]
    fn test_compute_document_frequencies() -> io::Result<()> {
        let f = File::open("testdata/testfile-yes.bin")?;
        let m = compute_document_frequencies(f);

        assert_ne!(m.is_empty(), true);
        assert_eq!(m.len(), 2);
        assert_eq!(m.count(&2879926931474365), 253);
        assert_eq!(m.count(&33279275454869446), 253);
        Ok(())
    }

    #[test]
    fn test_compute_document_scores() -> io::Result<()> {
        let f = File::open("testdata/testfile-yes.bin")?;
        let m = compute_document_scores(f);

        assert_ne!(m.is_empty(), true);
        assert_eq!(m.len(), 2);
        match m.get(&2879926931474365) {
            Some(score) => {
                let expected_score: f64 = 1.0 + (253.0 as f64).log10();
                assert!(approx_eq!(f64, *score, expected_score, epsilon = 0.001))
            },
            None => panic!("This value should exist")
        }

        Ok(())
    }
}
