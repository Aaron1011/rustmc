use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::boxed::Box;
use rustc_serialize::json;
use util::Either;
use util::WriterExtensions;
use std::marker::PhantomData;

/**
 * The Packet struct has a type parameter that isn't used in any
 * of it's field (i.e. a phantom type). We use it to only implement
 * certain methods for certain kinds of packets. This works since
 * the struct itself is private and the only way to get a Packet is
 * through one of two static methods: `new_in` and `new_out`.
 *
 * Packet<In> is basically just a wrapper around some buffer. It
 * represents a complete packet that we've read in. We also
 * implement the Reader trait to make it more convenient to
 * access the data it encompasses.
 *
 * Packet<Out> represents a buffer we can write to as we build
 * up a complete packet. It implements the Writer trait so we can
 * use all those convenient methods.
 */

enum In {}
enum Out {}

pub type InPacket = Packet<In>;
pub type OutPacket = Packet<Out>;

pub struct Packet<T> {
    pub buf: Either<Box<Read>, Box<Write>>,
    phantom: PhantomData<T>
    //packetType: T
}

impl Packet<In> {
    pub fn new_in(buf: Vec<u8>) -> Packet<In> {
        Packet {
            buf: Either::Left(Box::new(BufReader::new(buf.as_slice()))),
            //packetType: In
        }
    }
}

impl Packet<Out> {
    pub fn new_out(packet_id: i32) -> Packet<Out> {
        let mut p = Packet {
            buf: Either::Right(Box::new(Vec::new())),
            //packetType: Out
        };
        p.write_varint(packet_id);

        p
    }

    pub fn buf(self) -> Vec<u8> {
        self.buf.unwrap_right();
    }
}

impl Read for Packet<In> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.buf {
            Either::Left(ref mut r) => r.read(buf),
            Either::Right(..) => unreachable!()
        }
    }
}

impl Write for Packet<Out> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.buf {
            Either::Left(..) => unreachable!(),
            Either::Right(ref mut w) => w.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.buf {
            Either::Left(..) => unreachable!(),
            Either::Right(ref mut w) => w.flush()
        }
    }
}
