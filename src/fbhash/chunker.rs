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

use std::fs::File;
use std::io::Read;

const CHUNK_SIZE: usize = 7;
const A: u64 = 255;
const MODULUS: u64 = 801385653117583579;

#[derive(Clone, Debug, Copy)]
pub struct Chunk {
    pub number: usize,
    pub digest: u64,
}

#[derive(Debug)]
pub struct ChunkContent {
    current_number: usize,
    content: Vec<u8>,
    max_a: u64,
}

impl ChunkContent {
    #[allow(dead_code)]
    pub fn new() -> ChunkContent {
        ChunkContent {
            current_number: 0,
            content: vec![0; CHUNK_SIZE],
            max_a: A.pow(CHUNK_SIZE as u32),
        }
    }

    //
    // Initialize the content with the first bytes
    //
    pub fn setup(&mut self, v: &[u8]) -> Chunk {
        self.content.copy_from_slice(v);
        Chunk {
            number: self.current_number,
            digest: self.compute_digest(),
        }
    }

    /*
     * Compute the new digest, starting with previous.
     */
    pub fn update(&mut self, previous: u64, new_byte: u8) -> Chunk {
        // Shift everything one byte to the left
        let first_byte = self.content[0];
        self.content.copy_within(1.., 0);
        self.content[CHUNK_SIZE - 1] = new_byte;
        self.current_number += 1;
        let new_digest = self.rehash_digest(previous, first_byte, new_byte);
        Chunk {
            number: self.current_number,
            digest: new_digest,
        }
    }

    fn rehash_digest(&mut self, digest: u64, old_byte: u8, new_byte: u8) -> u64 {
        let b_i = old_byte as u64;
        let b_k = new_byte as u64;
        //((A * digest).wrapping_sub(b_i * A.pow(k)) + b_k) % MODULUS
        ((A * digest).wrapping_sub(b_i * self.max_a) + b_k) % MODULUS
    }

    fn compute_digest(&mut self) -> u64 {
        let mut h = 0;
        let mut k: u32 = CHUNK_SIZE as u32;
        for e in &self.content {
            let elem = *e as u64;
            h = (h + elem * A.pow(k - 1)) % MODULUS;
            k -= 1;
        }
        h
    }
}

#[derive(Debug)]
pub struct ChunkIterator {
    file: File,
    chunk_content: ChunkContent,
    // The option is None when not yet completed
    last_chunk: Option<Chunk>,
}

impl ChunkIterator {
    #[allow(dead_code)]
    pub fn new(file: File) -> ChunkIterator {
        ChunkIterator {
            file,
            chunk_content: ChunkContent::new(),
            last_chunk: None,
        }
    }
}

impl Iterator for ChunkIterator {
    type Item = Chunk;

    fn next(&mut self) -> Option<Chunk> {
        match self.last_chunk {
            None => {
                let mut initial_content = vec![0; CHUNK_SIZE];
                match self.file.read(&mut initial_content) {
                    Err(_) => None,
                    Ok(_) => {
                        let chunk = self.chunk_content.setup(&initial_content);
                        self.last_chunk = Some(chunk);
                        Some(chunk)
                    }
                }
            }
            Some(_) => {
                let mut b: Vec<u8> = vec![1];
                match self.file.read(&mut b) {
                    Ok(0) => None,
                    Ok(..) => {
                        let previous_digest = self.last_chunk.unwrap().digest;
                        let new_value = self.chunk_content.update(previous_digest, b[0]);
                        self.last_chunk = Some(new_value);
                        //let new_value = self.chunk_content.update(0, b[0]);
                        Some(new_value)
                    }
                    Err(_) => None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChunkIterator;
    use pretty_assertions::assert_eq;
    use std::fs::File;
    use std::io;

    #[test]
    fn test_litmus() {
        assert_eq!(1, 1)
    }

    #[test]
    fn test_litmus2() {
        assert!(approx_eq!(f32, 2.0 - 1.0, 1.0, epsilon = 0.00001));
    }

    #[test]
    fn test_first_chunk_from_zero_file() -> io::Result<()> {
        let f = File::open("testdata/testfile-zero.bin")?;
        let mut chunk_iterator = ChunkIterator::new(f);
        let chunk = chunk_iterator.next().unwrap();

        assert_eq!(chunk.number, 0);
        assert_eq!(chunk.digest, 0);
        Ok(())
    }

    #[test]
    fn test_get_all_chunks_from_zero_file() -> io::Result<()> {
        let f = File::open("testdata/testfile-zero.bin")?;
        let chunk_iterator = ChunkIterator::new(f);
        let chunks: Vec<_> = chunk_iterator.collect();
        assert_eq!(chunks.len(), 512 - 6);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.number, i);
            assert_eq!(chunk.digest, 0);
        }
        Ok(())
    }

    #[test]
    fn test_zero_length_file() -> io::Result<()> {
        let f = File::open("testdata/testfile-zero-length")?;
        let chunk_iterator = ChunkIterator::new(f);
        let chunks: Vec<_> = chunk_iterator.collect();
        assert_eq!(chunks.len(), 1);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.number, i);
            assert_eq!(chunk.digest, 0);
        }
        Ok(())
    }

    #[test]
    fn test_get_three_chunks_from_yes_file() -> io::Result<()> {
        let f = File::open("testdata/testfile-yes.bin")?;
        let mut chunk_iterator = ChunkIterator::new(f);
        let chunk0 = chunk_iterator.next().unwrap();
        assert_eq!(chunk0.number, 0);
        assert_eq!(chunk0.digest, 33279275454869446);
        let chunk1 = chunk_iterator.next().unwrap();
        assert_eq!(chunk1.number, 1);
        assert_eq!(chunk1.digest, 2879926931474365);
        let chunk2 = chunk_iterator.next().unwrap();
        assert_eq!(chunk2.number, 2);
        assert_eq!(chunk2.digest, 33279275454869446);
        Ok(())
    }

    #[test]
    fn test_get_all_chunks_from_yes_file() -> io::Result<()> {
        // This file contains 'y' and '\n' for 256 times. The digests should
        // thus be alternating.
        let f = File::open("testdata/testfile-yes.bin")?;
        let chunk_iterator = ChunkIterator::new(f);
        let chunks: Vec<_> = chunk_iterator.collect();
        assert_eq!(chunks.len(), 512 - 6);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.number, i);
            if i % 2 == 0 {
                assert_eq!(chunk.digest, 33279275454869446);
            } else {
                assert_eq!(chunk.digest, 2879926931474365);
            }
        }
        Ok(())
    }
}
