use std::io::{Read, Write};
use std::str;

use std::marker::Sized;

use std::str::FromStr;

use openssl::crypto::hash;
use openssl::crypto::hash::Hasher;

use rustc_serialize::hex::ToHex;

//use crypto;

pub fn special_digest(mut hasher: Hasher) -> String {
    let mut digest = hasher.finish();

    let neg = (digest.get(0).unwrap() & 0x80) == 0x80;
    if neg {
        let mut carry = true;
        for x in digest.iter_mut().rev() {
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
    let digest = String::from_str(digest.trim_left_matches('0')).unwrap();


    if neg {
        let mut a = "-".to_string();
        a.push_str(digest.as_str());
        a
    } else {
        digest
    }
}

pub trait WriterExtensions: Write {

    #[allow(exceeding_bitshifts)]
    fn write_varint(&mut self, mut x: i32) {
        let mut buf = [0u8; 10];
        let mut i = 0;
        /*if x < 0 {
            x = x + (1 << 32);
        }*/
        while x >= 0x80 {
            buf[i] = (x & 0x7F) as u8 | 0x80;
            x = x >> 7;
            i = i + 1;
        }
        buf[i] = x as u8;

        println!("Trying to write varint: {:?}", x);

        self.write(&buf[0 .. (i + 1)]);
    }

    fn write_string(&mut self, s: &str) {
        self.write_varint(s.len() as i32);
        self.write(s.as_bytes());
    }
}

impl<T: Write> WriterExtensions for T {}

pub trait ReaderExtensions: Read {
    #[allow(exceeding_bitshifts)]
    fn read_varint(&mut self) -> i32 {
        let (mut total, mut shift, mut val) = (0, 0, 0x80);
        let mut buf = [0; 1];
        while (val & 0x80) != 0 {
            self.read_exact(&mut buf);
            val = buf[0] as i32;
            total = total | ((val & 0x7F) << shift);
            shift = shift + 7;
            buf = [0; 1]
        }

        if (total & (1 << 31)) != 0 {
            total = total - (1 << 32);
        }

        total
    }

    fn read_string(&mut self) -> String {
        let len = self.read_varint();

        let mut buf = Vec::new();
        self.take(len as u64).read_to_end(&mut buf);

        //let buf = repeat(0).take(len).collect::<Vec<_>>().as_mut_slice();
        //let a = self.read_exact(buf);

        return String::from_utf8(buf).unwrap();
    }

    fn read_len(&mut self, len: u64) -> Box<[u8]> where Self: Sized {
        let mut buf = Vec::new();
        self.take(len).read_to_end(&mut buf);

        println!("buf len = {}, parameter len = {}", buf.len(), len);
        return buf.into_boxed_slice();
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
