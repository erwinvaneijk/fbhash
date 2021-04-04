
use std::fs::File;
use std::io::{Read, Error};


const CHUNK_SIZE: usize = 7;
const A : u64 = 255;
const MODULUS: u64 = 801385653117583579;

#[derive(Clone, Debug, Copy)]
pub struct Chunk {
    number: usize,
    digest: u64
}

#[derive(Debug)]
pub struct ChunkContent {
    current_number: usize,
    content: Vec<u8>
}

impl ChunkContent {
    pub fn new() -> ChunkContent {
        ChunkContent{
            current_number: 0,
            content: vec![0; CHUNK_SIZE]
        }
    }

    //
    // Initialize the content with the first bytes
    //
    pub fn setup(&mut self, v: &Vec<u8>) -> Chunk {
        self.content.copy_from_slice(v);
        Chunk{number: self.current_number, digest: self.compute_digest()}
    }

    /*
     * Compute the new digest, starting with previous.
     *
     * Current implementation computes too much, but works for now.
     * TODO:
     *   Speed this up because we can compute the new digest by subtracting
     *   the first byte's formula from the digest and adding the last bytes formula
     *   to the new digest.
     */
    pub fn update(&mut self, _previous: u64, new_byte: u8) -> Chunk {
        // Shift everything one byte to the left
        self.content.copy_within(1.., 0);
        self.content[CHUNK_SIZE - 1] = new_byte;
        self.current_number += 1;
        Chunk{number: self.current_number, digest: self.compute_digest()}
    }

    fn compute_digest(&mut self) -> u64 {
        let mut h = 0;
        let mut k = CHUNK_SIZE;
        for e in &self.content {
            let elem = *e as u64;
            h = (h + elem * A.pow((k as u32) - 1)) % MODULUS;
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
    last_chunk: Option<Chunk>
}

impl ChunkIterator {
    fn new(file: File) -> Result<ChunkIterator, Error> {
        Ok(ChunkIterator{
            file: file,
            chunk_content: ChunkContent::new(),
            last_chunk: None
        })
    }
}

impl<'a> Iterator for ChunkIterator {
    type Item = Chunk;

    fn next(& mut self) -> Option<Chunk> {
        match self.last_chunk {
            None =>
            {
                let mut initial_content = vec![0; CHUNK_SIZE];
                match self.file.read(&mut initial_content) {
                    Err(_) =>
                        None,
                    Ok(_) =>
                    {
                        let mut chunk_content = ChunkContent::new();
                        let chunk = chunk_content.setup(&initial_content);
                        self.last_chunk = Some(chunk);
                        Some(chunk)
                    }
                }
            },
            Some(_) => {
                let mut b: Vec<u8> = vec![1];
                match self.file.read(&mut b) {
                    Ok(0) => None,
                    Ok(..) => {
                        Some(self.chunk_content.update(self.last_chunk.unwrap().digest, b[0]))
                    }
                    Err(_) =>
                        None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::fs::File;

    #[test]
    fn test_litmus() {
        assert_eq!(1, 1)
    }

    #[test]
    fn test_first_chunk_from_file() -> io::Result<()> {
        let f = File::open("testdata/testfile-zero.bin")?;
        let mut chunk_iterator = ChunkIterator::new(f)?;
        let chunk = chunk_iterator.next().unwrap();

        assert_eq!(chunk.number, 0);
        assert_eq!(chunk.digest, 0);
        Ok(())
    }

    #[test]
    fn test_get_all_chunks_from_file() -> io::Result<()> {
        let f = File::open("testdata/testfile-zero.bin")?;
        let chunk_iterator = ChunkIterator::new(f)?;
        let chunks: Vec<_> = chunk_iterator.collect();
        assert_eq!(chunks.len(), 512 - 6);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.number, i);
            assert_eq!(chunk.digest, 0);
        }
        Ok(())
    }
}
