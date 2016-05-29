use std::io::{Read, Write};
use std::str;

use openssl::crypto::hash;
use openssl::crypto::hash::Hasher;

use serialize::hex::ToHex;

//use crypto;

pub fn special_digest(hasher: Hasher) -> String {
    let mut digest = hasher.final();

    let neg = (digest.get(0) & 0x80) == 0x80;
    if neg {
        let mut carry = true;
        for x in digest.mut_iter().rev() {
            *x = !*x;
            if carry {
                carry = *x == 0xFF;
                *x = *x + 1;
            }
        }
    }

    let digest = digest.as_slice().to_hex();

    //let digest = hash::hash(hash::SHA1, digest);i
    /*match digest.as_slice().position_elem(&0) {
        Some(pos) => digest.remove(pos),
        None => Some(0)
    };*/
    let digest = digest.as_slice().trim_left_chars('0').to_owned();

    if neg { "-".to_string().append(digest.as_slice()) } else { digest }
}

pub trait WriterExtensions: Write {
    fn write_varint(&mut self, mut x: i32) {
        let mut buf = [0u8, ..10];
        let mut i = 0;
        if x < 0 {
            x = x + (1 << 32);
        }
        while x >= 0x80 {
            buf[i] = (x & 0x7F) as u8 | 0x80;
            x = x >> 7;
            i = i + 1;
        }
        buf[i] = x as u8;

        self.write(buf.slice_to(i + 1));
    }

    fn write_string(&mut self, s: &str) {
        self.write_varint(s.len() as i32);
        self.write(s.as_bytes());
    }
}

impl<T: Write> WriterExtensions for T {}

pub trait ReaderExtensions: Read {
    fn read_varint(&mut self) -> i32 {
        let (mut total, mut shift, mut val) = (0, 0, 0x80);

        while (val & 0x80) != 0 {
            val = self.read_u8().unwrap() as i32;
            total = total | ((val & 0x7F) << shift);
            shift = shift + 7;
        }

        if (total & (1 << 31)) != 0 {
            total = total - (1 << 32);
        }

        total
    }

    fn read_string(&mut self) -> String {
        let len = self.read_varint();
        let buf = self.read_exact(len as u64).unwrap();

        str::from_utf8_owned(buf.move_iter().collect()).unwrap()
    }
}

impl<T: Read> ReaderExtensions for T {}

pub enum Either<L, R> {
    Left(L),
    Right(R)
}

impl<L, R> Either<L, R> {
    pub fn unwrap_left(self) -> L {
        match self {
            Either::Left(x) => x,
            Either::Right(_) => panic!("tried to unwrap left but got right")
        }
    }

    pub fn unwrap_right(self) -> R {
        match self {
            Either::Left(_) => panic!("tried to unwrap right but got left"),
            Either::Right(x) => x
        }
    }
}
