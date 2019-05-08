pub struct ByteStream {
    data: Vec<u8>,
    pos: usize,
}

impl ByteStream {
    pub fn new(bytes: Vec<u8>) -> Self {
        ByteStream { data: bytes, pos : 0 }
    }
    
    /// Return the number of bytes still available for reading
    pub fn available(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Return the total number of bytes in the stream
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_pos(&self) -> usize {
        self.pos
    }
    
    pub fn last(&self) -> Option<&u8> {
        self.data.last()
    }
    
    /// Return the next available byte without updating the position    
    pub fn peek(&self) -> u8 {
        self.data[self.pos]
    }

    pub fn peek_u32_at(&self, pos_offset: usize) -> u32 {
        (self.data[self.pos + pos_offset] as u32) |
        (self.data[self.pos + pos_offset + 1] as u32) << 8 |
        (self.data[self.pos + pos_offset + 2] as u32) << 16 |
        (self.data[self.pos + pos_offset + 3] as u32) << 24        
    }

    /// Replace the byte at `pos_offset` with `val`- The offset is relative to the current position.
    pub fn replace_at(&mut self, pos_offset: usize, val: u8) {
        self.data[self.pos + pos_offset] = val;
    }
    
    /// Replace the u32 at `pos_offset` with `val`- The offset is relative to the current position.
    pub fn replace_u32_at(&mut self, pos_offset: usize, val: u32) {
        self.data[self.pos + pos_offset] = (val & 0xFF) as u8;
        self.data[self.pos + pos_offset + 1] = (val >> 8) as u8;
        self.data[self.pos + pos_offset + 2] = (val >> 16) as u8;
        self.data[self.pos + pos_offset + 3] = (val >> 24) as u8;
    }
    
    /// Reset the position
    pub fn reset(&mut self) {
        self.pos = 0;
    }
    
    /// Return the next available byte and increment the position
    pub fn read(&mut self) -> u8 {
        let b = self.data[self.pos];
        self.pos += 1;
        b    
    }

    /// Return the next `n` available bytes and increment the position
    pub fn read_n(&mut self, n: usize) -> Vec<u8> {
        let v = self.data[self.pos..self.pos+n].to_vec();
        self.pos += n;
        v
    }
    
    /// Return a Vec with all available bytes
    pub fn read_available(&mut self) -> Vec<u8> {
        let v = self.data[self.pos..].to_vec();
        self.pos = self.data.len();
        v
    }

     pub fn borrow_n(&mut self, n: usize) -> &[u8] {
        let r = &self.data[self.pos..self.pos+n];
        self.pos += n;
        r
    }

    pub fn skip(&mut self, n: usize) {
        self.pos += n;
    }
    
    /// Place `val` at *the end* of the stream
    pub fn write(&mut self, val: u8) {
        self.data.push(val);
    }
    
    /// Place the values in `s` at *the end* of the stream
    pub fn write_n(&mut self, s: &[u8]) {
        self.data.extend_from_slice(s);
    }    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create() {
        let bs = ByteStream::new(vec![1, 2, 3]);
        assert_eq!(bs.available(), 3);
        assert_eq!(bs.len(), 3);
    }

    #[test]
    fn test_read() {
        let mut bs = ByteStream::new(vec![1, 2, 3]);
        assert_eq!(bs.read(), 1);
        assert_eq!(bs.available(), 2);
        assert_eq!(bs.len(), 3);
    }

    #[test]
    fn test_read_n() {
        let mut bs = ByteStream::new(vec![1, 2, 3]);
        assert_eq!(bs.read_n(2), vec![1, 2]);
        assert_eq!(bs.available(), 1);
    }

    #[test]
    fn test_skip() {
        let mut bs = ByteStream::new(vec![1, 2, 3]);
        bs.skip(2);
        assert_eq!(bs.read(), 3);
        assert_eq!(bs.available(), 0);
    }

    #[test]
    fn test_reset() {
        let mut bs = ByteStream::new(vec![1, 2, 3]);
        assert_eq!(bs.read(), 1);
        assert_eq!(bs.available(), 2);
        bs.reset();
        assert_eq!(bs.available(), 3);
        assert_eq!(bs.read(), 1);
    }

    #[test]
    fn test_write() {
        let mut bs = ByteStream::new(vec![1, 2, 3]);
        bs.write(4);
        assert_eq!(bs.available(), 4);
        assert_eq!(bs.get_pos(), 0);
    }
}