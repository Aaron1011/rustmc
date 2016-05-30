extern crate getopts;
extern crate term;
extern crate openssl;
extern crate rand;
extern crate rustc_serialize;
extern crate byteorder;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};
use std::os;
use std::env;
use getopts::Options;

mod conn;
mod crypto;
mod packet;
mod util;
mod json;

static DEFAULT_NAME: &'static str = "rustmc-bot";
static DEFAULT_HOST: &'static str = "127.0.0.1";
static DEFAULT_PORT: u16          = 25565;


fn usage(prog: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", prog);
    println!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();

    opts.optflag("h", "help", "Display this message");
    opts.optopt("s", "server", "Minecraft server host", "HOST");
    opts.optopt("p", "port", "Minecraft server port", "PORT");
    opts.optopt("n", "name", "Username to use.", "NAME");
    opts.optflag("c", "status", "Get info about the server.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!(e.to_string())
    };

    // Should we print out the usage message?
    if matches.opt_present("help") {
        usage(args.get(0).unwrap().as_str(), opts);
        return;
    }

    let status = matches.opt_present("status");
    let name = matches.opt_str("name").unwrap_or(DEFAULT_NAME.to_string());
    let host = matches.opt_str("server").unwrap_or(DEFAULT_HOST.to_string());
    let port = matches.opt_str("port").map_or(DEFAULT_PORT, |x| x.parse::<u16>().unwrap());

    match conn::Connection::new(name.as_str(), host.as_str(), port) {
        Ok(ref mut c) if status => c.status(),
        Ok(c) => c.run(),
        Err(e) => panic!("Unable to connect to server: {}.", e)
    }


    println!("Status: {}, name: {}, host: {}, port: {}", status, name, host, port)
}
