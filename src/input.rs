use core::str;
use no_std_io::io::{Error, ErrorKind, Read};

// pub type InputStream<'a> = &'a dyn Read;
pub trait InputStream {
    fn peek(&mut self) -> Result<Option<&str>, Error>;
    fn next_word(&mut self) -> Result<Option<&str>, Error>;
}

/// lexes tokens from a ['Read'] stream
/// this type has a few gurntees:
///    1. we call read as late as possible to avoid blocking as much as we can
///    2. once read returned 0 no subsequent calls would be made
///    3. input would be fully validated for UTF8 and whitespaces are checked based on UTF8
///    4. memory error only happens once the buffer is fully used we never error on fragmentation
pub struct WordStream<R: Read, const N: usize = 4096> {
    r: R,
    r_done: bool,
    buf: [u8; N],
    start: usize,
    valid_len: usize,
    len: usize,
}

impl<R: no_std_io::io::Read, const N: usize> WordStream<R, N> {
    pub fn new(r: R) -> Self {
        Self {
            r,
            r_done: false,
            buf: [0; N],
            start: 0,
            valid_len: 0,
            len: 0,
        }
    }

    fn shift_buffer(&mut self) {
	    if self.start == 0 { return; }

	    let end = self.start + self.len;          // exclusive upper bound
	    self.buf.copy_within(self.start..end, 0); // safe, handles overlap
	    self.start = 0;
	}


    fn remainder(&mut self) -> &[u8] {
        let idx = self.start + self.valid_len;
        let len = self.len - self.valid_len;
        &self.buf[idx..][..len]
    }

    fn extend_valid(&mut self) -> Result<(), Error> {
        match str::from_utf8(self.remainder()) {
            Ok(s) => {
                self.valid_len += s.len();
                Ok(())
            }
            Err(e) => {
                self.valid_len += e.valid_up_to();
                match e.error_len() {
                    Some(_) => Err(Error::new(ErrorKind::InvalidData, "invalid UTF-8")),
                    None => Ok(()),
                }
            }
        }
    }

    ///try fiilling the internal buffer with a blocking read
    ///note that this may require a memove internally
    pub fn fill(&mut self) -> Result<usize, Error> {
        if self.r_done {
            return Ok(0);
        }

        if self.len == N {
            return Err(Error::new(ErrorKind::Other, "Buffer Overflow on word"));
        }
        if self.start + self.len == N {
            self.shift_buffer();
        }

        let spot = &mut self.buf[self.start + self.len..];

        let added = self.r.read(spot)?;
        self.len += added;

        if added == 0 {
            self.r_done = true;

            //verify there is no weird junk in the end
            self.extend_valid()?;
            if self.valid_len != self.len {
                return Err(Error::new(ErrorKind::InvalidData, "invalid UTF-8"));
            }
        }
        Ok(added)
    }

    /// this peeks the buffer without calling read internaly
    /// note that we dont cache the result so this is a recalculation every time
    /// however validaton logic is cached as well as skiping white space
    pub fn scan(&mut self) -> Result<Option<&str>, Error> {
        //make sure we dont acidently return a none
        self.extend_valid()?;

        //current string
        let spot = &self.buf[self.start..][..self.valid_len];
        let s = unsafe { str::from_utf8_unchecked(spot) };

        let mut termed = true;

        //skip whitespaces
        for c in s.chars() {
            if c.is_whitespace() {
                self.len -= c.len_utf8();
                self.valid_len -= c.len_utf8();
                self.start += c.len_utf8();
                continue;
            }

            termed = false;
            break;
        }

        if termed {
            //sometimes start would be 1 past the end of the buff
            //so we set it to 0 in that case
            if self.len == 0 {
                self.start = 0;
            }
            return Ok(None);
        }

        let spot = &self.buf[self.start..][..self.valid_len];
        let s = unsafe { str::from_utf8_unchecked(spot) };
        let mut total_len = 0;

        termed = true;
        for c in s.chars() {
            if !c.is_whitespace() {
                total_len += c.len_utf8();
                continue;
            }

            termed = false;
            break;
        }

        //maybe not enough input
        if termed && !self.r_done {
            return Ok(None);
        }

        unsafe { Ok(Some(str::from_utf8_unchecked(&spot[..total_len]))) }
    }

    pub unsafe fn consume_bytes(&mut self, total_len: usize) {
        self.len -= total_len;
        self.valid_len -= total_len;
        self.start += total_len;
    }
}

impl<R: no_std_io::io::Read, const N: usize> InputStream for WordStream<R, N> {
    fn peek(&mut self) -> Result<Option<&str>, no_std_io::io::Error> {
        //try and avoid reading
        match self.scan()?.map(|s| s as *const str) {
            //# Safety
            //rust is dumb about this lifetime for no good reason
            //it basically thinks that since the Some case borrows that the None case does
            //scaning twice as well as a todo both work so this is basically just rustc being anoying
            Some(s) => Ok(Some(unsafe { &*s })),

            //only now do we try filling and then check again
            None => {
                if self.fill()? == 0 {
                    return Ok(None);
                } else {
                    self.peek() //TCO 
                }
            }
        }
    }
    fn next_word(&mut self) -> Result<Option<&str>, no_std_io::io::Error> {
        match self.scan()?.map(|s| s as *const str) {
            Some(s) => unsafe {
                //we need to be careful not to make a ref that lives between calls
                let len = (&*s).as_bytes().len();
                let addr = s.addr();

                //borrows mut
                self.consume_bytes(len);

                //now s no longer valid... need to reconstruct it
                //this is basically a no op but the provance changes
                let p = self.buf.as_ptr().with_addr(addr);
                let s = core::slice::from_raw_parts(p, len);
                Ok(Some(str::from_utf8_unchecked(s)))
            },
            None => {
                if self.fill()? == 0 {
                    Ok(None)
                } else {
                    self.next_word()
                }
            }
        }
    }
}

/*──────────────────────────── tests ────────────────────────────────*/
#[cfg(test)]
mod tests {
    use super::*;
    use no_std_io::io::Cursor;

    #[test]
    fn refill_with_incomplete_utf8_is_ok() {
        let src = "αβγ δεζ "; // two words + space
        let bytes = src.as_bytes();

        //explicit per-token byte lengths
        let mut parts = src.split_whitespace();
        let first = parts.next().unwrap();
        let second = parts.next().unwrap();
        assert_eq!(first.as_bytes().len(), 6);
        assert_eq!(second.as_bytes().len(), 6);

        //choose a buffer which would require a memove
        let mut rdr = WordStream::<_, 9>::new(Cursor::new(bytes));

        assert_eq!(rdr.peek().unwrap(), Some(first));
        assert_eq!(rdr.next_word().unwrap(), Some(first));
        assert_eq!(rdr.next_word().unwrap(), Some(second));
        assert_eq!(rdr.next_word().unwrap(), None);
    }

    #[test]
    fn eof_with_incomplete_seq_errors() {
        let bad = b"\xE2\x82"; // first two bytes of '€'
        let mut rdr = WordStream::<_, 8>::new(Cursor::new(bad));

        let err = rdr.next_word().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }
}
