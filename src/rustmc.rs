extern crate getopts;
extern crate term;
extern crate serialize;
extern crate openssl;
extern crate serialize;

use std::os;

mod conn;
mod crypto;
mod packet;
mod util;
mod json;

static DEFAULT_NAME: &'static str = "rustmc-bot";
static DEFAULT_HOST: &'static str = "127.0.0.1";
static DEFAULT_PORT: u16          = 25565;


fn usage(prog: &str, opts: &[getopts::OptGroup]) {
    let message = format!("Usage: {} [OPTIONS]", prog).to_string();
    std::io::println(getopts::usage(message.as_slice(), opts).as_slice());
}

fn main() {
    let args = os::args();
    let opts = [
        getopts::optflag("h", "help", "Display this message"),
        getopts::optopt("s", "server", "Minecraft server host", "HOST"),
        getopts::optopt("p", "port", "Minecraft server port", "PORT"),
        getopts::optopt("n", "name", "Username to use.", "NAME"),
        getopts::optflag("c", "status", "Get info about the server."),
    ];

    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(e) => fail!(e.to_string())
    };

    // Should we print out the usage message?
    if matches.opt_present("help") {
        usage(args.get(0).as_slice(), opts);
        return;
    }

    let status = matches.opt_present("status");
    let name = matches.opt_str("name").unwrap_or(DEFAULT_NAME.to_string());
    let host = matches.opt_str("server").unwrap_or(DEFAULT_HOST.to_string());
    let port = matches.opt_str("port").map_or(DEFAULT_PORT, |x| from_str(x.as_slice()).expect("invalid port"));

    match conn::Connection::new(name.as_slice(), host.as_slice(), port) {
        Ok(ref mut c) if status => c.status(),
        Ok(c) => c.run(),
        Err(e) => fail!("Unable to connect to server: {}.", e)
    }


    println!("Status: {}, name: {}, host: {}, port: {}", status, name, host, port)
}
