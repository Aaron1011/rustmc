use std::io;
use std::io::{Read, Write, Result};
use std::net::TcpStream;
use openssl::crypto::symm::{Type, Mode, Crypter};


pub struct AesStream {
    stream: TcpStream,
    encrypt: Crypter,
    decrypt: Crypter,
    key: Vec<u8>
}

impl AesStream {
    pub fn new(s: TcpStream, key: Vec<u8>) -> AesStream {
        let encrypt = Crypter::new(Type::AES_128_CFB);
        let decrypt = Crypter::new(Type::AES_128_CFB);
        encrypt.init(Mode::Encrypt, key.as_slice(), key.clone());
        decrypt.init(Mode::Decrypt, key.as_slice(), key.clone());
        AesStream {
            stream: s,
            encrypt: encrypt,
            decrypt: decrypt,
            key: key
        }
    }
}

impl Read for AesStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
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

impl  Write for AesStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let data = self.encrypt.update(buf);
        self.encrypt.final();
        self.stream.write(data.as_slice())
    }

    fn flush(&mut self) -> Result<()> {
        self.stream.flush()
    }
}
