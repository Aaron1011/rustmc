use std::io;
use std::io::{Reader, Writer, Stream};
use openssl::crypto::symm;
use openssl::crypto::symm::Crypter;


pub struct AesStream<T> {
    stream: T,
    encrypt: Crypter,
    decrypt: Crypter,
    key: Vec<u8>
}

impl<T: Stream> AesStream<T> {
    pub fn new(s: T, key: Vec<u8>) -> AesStream<T> {
        let encrypt = Crypter::new(symm::AES_128_CFB);
        let decrypt = Crypter::new(symm::AES_128_CFB);
        encrypt.init(symm::Encrypt, key.as_slice(), key.clone());
        decrypt.init(symm::Decrypt, key.as_slice(), key.clone());
        AesStream {
            stream: s,
            encrypt: encrypt,
            decrypt: decrypt,
            key: key
        }
    }
}

impl<T: Reader> Reader for AesStream<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        let ein = self.stream.read_exact(buf.len()).unwrap();
        let din =self.decrypt.update(ein.as_slice());
        self.decrypt.final();
        //let din = self.decrypt.final();
        /*let din = match self.cipher.decrypt(ein.as_slice()) {
            Ok(d) => d,
            Err(_) => return Err(io::standard_error(io::OtherIoError))
        };*/
        let l = din.len();
        buf.move_from(din, 0, l);

        Ok(buf.len())
    }
}

impl<T: Writer> Writer for AesStream<T> {
    fn write(&mut self, buf: &[u8]) -> io::IoResult<()> {
        let data = self.encrypt.update(buf);
        self.encrypt.final();
        self.stream.write(data.as_slice())
    }

    fn flush(&mut self) -> io::IoResult<()> {
        self.stream.flush()
    }
}
