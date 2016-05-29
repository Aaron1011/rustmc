use std::io;

use std::io::net::tcp::TcpStream;
use std::string::String;
use std::str;
use openssl::crypto::hash::Hasher;
use std::collections::HashMap;

use serialize::json;

use packet;
use crypto;
use openssl;
use std::rand;
use std::rand::Rng;
use packet::Packet;
use util::{ReaderExtensions, WriterExtensions, special_digest};
use std::io::{BufferedReader, Reader, Writer};
use openssl::crypto::pkey;
use openssl::crypto::hash::SHA1;
use std::io::Command;
use std::comm;
use term;

#[deriving(Hash, Eq, PartialEq, Show)]
pub struct Position {
    x: int,
    y: int,
    z: int
}

#[deriving(Hash, Eq, PartialEq, Show)]
pub struct Entity {
    pos: Position,
    kind: u8
}

pub struct Connection {
    host: String,
    sock: Option<Sock>,
    name: String,
    port: u16,
    term: Box<term::Terminal<term::WriterWrapper>>,
    entities: HashMap<int, Entity>
}

enum Sock {
    Plain(TcpStream),
    Encrypted(crypto::AesStream<TcpStream>)
}

impl Connection {
    pub fn new(name: &str, host: &str, port: u16) -> Result<Connection, String> {

        println!("Connecting to server")

        let sock = TcpStream::connect(host, port);

        let sock = match sock {
                Ok(s) => s,
                Err(e) => return Err(format!("{} - {}", e.kind, e.desc))
        };

        println!("Connected!")

        let t = match term::stdout() {
            Some(t) => t,
            None => return Err(String::from_str("Terminal could not be created"))
        };

        let mut e = HashMap::new();


        Ok(Connection {
            host: String::from_str(host),
            sock: Some(Plain(sock)),
            name: String::from_str(name),
            port: port,
            term: t,
            entities: e
        })
    }

    fn authenticate(&mut self, hash: String) {
        let url = "https://authserver.mojang.com/authenticate".to_string();
        /*let c = process::ProcessConfig {
            program: "/usr/bin/curl",
            args: &[box "-d", box "@-", box "-H", box "Content-Type:application/json", url],
            env: None,
            cwd: None,
            stdin: process::CreatePipe(true, false),
            stdout: process::CreatePipe(false, true),
            .. process::ProcessConfig::new()
        };*/
        //let mut p = process::Process::configure(c).unwrap();

        let mut p = match Command::new("/usr/bin/curl").
            args(&["-d".to_string(), "@-".to_string(), "-H".to_string(), "Content-Type:application/json".to_string(), url]).
            spawn() {
                Ok(p) => p,
                Err(e) => fail!("Failed to execute process: {}", e)
            };

        /*match self.read_u8() {
            Ok(v) => v as i32,
            Err(e) => fail!("Error: {}", e)
        }*/
        // write json to stdin and close it
        p.stdin.get_mut_ref().write(format!(r#"
            {{
                "agent": {{
                    "name": "Minecraft",
                    "version": 1
                }},
                "username": "{}",
                "password": "{}"
            }}"#, "Aaron1011", "aaron11").as_bytes()); // XXX: Don't hardcode these...
        p.stdin = None;

        // read response
        let out = p.wait_with_output().unwrap().output;
        let out = str::from_utf8_owned(out.move_iter().collect()).unwrap();
        println!("Got - {}", out);

        //let json = ExtraJSON::new(json::from_str(out).unwrap());
        let json = json::from_str(out.as_slice()).unwrap();
        let token = json.find(&"accessToken".to_string()).unwrap().as_string().unwrap();
        let profile = json.find(&"selectedProfile".to_string()).unwrap().find(&"id".to_string()).unwrap().as_string().unwrap();

        println!("Data: {}, {}", token, profile);

        let url = "https://sessionserver.mojang.com/session/minecraft/join".to_string();
        /*let c = process::ProcessConfig {
            program: "/usr/bin/curl",
            args: &[box "-d", box "@-", box "-H", box "Content-Type:application/json", url],
            env: None,
            cwd: None,
            stdin: process::CreatePipe(true, false),
            stdout: process::CreatePipe(false, true),
            .. process::ProcessConfig::new()
        };
        let mut p = process::Process::configure(c).unwrap();*/

        let mut p = match Command::new("/usr/bin/curl").
            args(&["-d".to_string(), "@-".to_string(), "-H".to_string(), "Content-Type:application/json".to_string(), url]).
            spawn() {
                Ok(p) => p,
                Err(e) => fail!("Failed to execute process: {}", e)
            };


        // write json to stdin and close it
        p.stdin.get_mut_ref().write(format!(r#"
            {{
                "accessToken": "{}",
                "selectedProfile": "{}",
                "serverId": "{}"
            }}"#, token, profile, hash).as_bytes());
        p.stdin = None;

        println!("Starting")

        // read response
        p.wait_with_output();//.unwrap().output;
        println!("Done")
        //let out = str::from_utf8_owned(out.move_iter().collect()).unwrap();
        //println!("Got - {}", out);
    }

    fn read_messages(&self) -> Receiver<String> {
        let (chan, port) = comm::channel();

        spawn(proc() {
            println!("Type message and then [ENTER] to send:");

            let mut stdin = BufferedReader::new(io::stdin());
            for line in stdin.lines() {
                match line {
                    Ok(text) => chan.send(text),
                    _ => chan.send(String::from_str("")),
                }
            }
        });

        port
    }


    pub fn run(mut self) {

        // If the server is in online-mode
        // we need to do authentication and
        // enable encryption
        self.login();
        self.respawn();

        // Get a port to read messages from stdin
        let msgs = self.read_messages();

        // Yay, all good.
        // Now we just loop and read in all the packets we can
        // We don't actually do anything for most of them except
        // for chat and keep alives.
        loop {
            // Got a message in the queue to send?
            'msg: loop {
                match msgs.try_recv() {
                    Ok(msg) => {
                        if msg.is_empty() {
                            continue;
                        } else if msg.len() > 100 {
                            println!("Message too long.");
                            continue;
                        }

                        // Send the message!
                        let mut p = Packet::new_out(0x1);
                        p.write_string(msg.as_slice());
                        self.write_packet(p);
                    }
                    Err(ref err) => {
                        match err {
                            Empty => break 'msg,
                            //_ => fail!("Input disconnected")
                        }
                    }
                    /*Err(err) => match err {
                        com
                        println!("Food")m::TryRecvErr => 'msg,
                        _ => continue
                    }*/
                    //comm::Disconnected => fail!("input stream disconnected")
                }
            }

            // Read in and handle a packet
            let (packet_id, mut packet) = self.read_packet();
            self.handle_message(packet_id, &mut packet);
        }
    }

    fn handle_message(&mut self, packet_id: i32, packet: &mut packet::InPacket) {
        // Keep Alive
        if packet_id == 0x0 {
            let x = packet.read_be_i32().unwrap();

            // Need to respond
            let mut resp = Packet::new_out(0x0);
            resp.write_be_i32(x);
            self.write_packet(resp);

        // Chat Message
        } else if packet_id == 0x01 {
            println!("Joined!!")
            let id = packet.read_be_int_n(4);
            println!("Id")
            let gamemode = packet.read_be_uint_n(1);
            println!("Gamemode")
            let dimension = packet.read_be_int_n(1);
            println!("Dimm")
            let difficulty = packet.read_be_uint_n(1);
            println!("Diff")
            let players = packet.read_be_uint_n(1);
            println!("Players")
            //let level = packet.read_to_end();
            //println!("Data: {}", level)
            let level = packet.read_string();
            println!("Join game: {} {} {} {} {} {}", id, gamemode, dimension, difficulty, players, level)
        } else if packet_id == 0x06 {
            println!("Food")
            let health = packet.read_be_f32().unwrap();
            let food = packet.read_be_int_n(2).unwrap();
            let sat = packet.read_be_f32().unwrap();
            println!("Health: {}, Food: {}, Saturation: {}", health, food, sat)
        } else if packet_id == 0x0F {
            println!("Mob spawn!")
            let id = packet.read_varint();
            let type_ = packet.read_be_uint_n(1).unwrap();

            let pos = Position{x: 0, y: 0, z: 0};
            let entity = Entity{pos: pos, kind: type_ as u8};
            println!("Type of entity: {}", type_)

            self.entities.insert(id as int, entity);

            let mut p = Packet::new_out(0x02);
            p.write_be_i32(id);
            p.write_i8(1 as i8);
            println!("Hitting: {}", type_)
            self.write_packet(p);

            println!("Id: {}, Type: {}", id, type_)
        } else if packet_id == 0x15 {
            let id = packet.read_be_int_n(4).unwrap();
            let entity = self.entities.get(&(id as int));
            let mut p = Packet::new_out(0x02);
            p.write_be_i32(id as i32);
            p.write_i8(1 as i8);
            println!("Hitting - don't move!: {}", entity.kind)
            self.write_packet(p);
        } else if packet_id == 0x13 {
            let count = packet.read_varint();
            println!("Removing: {}", count);
        } else if packet_id == 0x05 {
            println!("Spawn coord")
            let x = packet.read_be_int_n(4);
            let y = packet.read_be_int_n(4);
            let z = packet.read_be_int_n(4);
            println!("Spawn: {} {} {}", x, y, z)
        } else if packet_id == 0x2 {
            let json = packet.read_string();
            println!("Got chat message: {}", json);

            // Let's wrap up the Json so that we can
            // deal with it more easily
            let j = json::from_str(json.as_slice()).unwrap();
            //let j = ExtraJSON::new(j);

            let ty = match j.find(&String::from_str("translate")) {
                Some(json) => json.as_string().unwrap(),
                _ => ""
            };

            // Player Chat
            if "chat.type.text" == ty {

                let user = j.find(&String::from_str("with")).unwrap().as_list().unwrap().get(0).find(&String::from_str("text")).unwrap().as_string().unwrap();
                let msg = j.find(&String::from_str("with")).unwrap().as_list().unwrap().get(1).as_string().unwrap();

                self.term.attr(term::attr::ForegroundColor(term::color::BRIGHT_GREEN));
                //write!(&mut self.term as &mut Writer, "<{}> ", user);
                self.term.write(user.as_bytes());
                self.term.reset();

                self.term.write(msg.as_bytes());
                self.term.write(b"\n");

            // Server Message
            } else if "chat.type.announcement" == ty {

                let msg = j.find(&String::from_str("with")).unwrap().as_list().unwrap().get(1).find(&String::from_str("extra")).unwrap().as_list().unwrap();
                let mut msg_vec = Vec::new();
                msg.iter().map(|x| msg_vec.push(x.as_string().unwrap()));
                let msg = msg_vec.concat();

                self.term.attr(term::attr::ForegroundColor(term::color::BRIGHT_YELLOW));
                self.term.write(b"[Server] ");
                self.term.reset();

                self.term.write(msg.as_bytes());
                self.term.write(b"\n");

            }
        }
    }

    fn send_username(&mut self) {
        let mut p = Packet::new_out(0x0);
        p.write_string(self.name.as_slice());

        self.write_packet(p);
    }

    fn respawn(&mut self) {
        println!("Respawning!")
        let mut p = Packet::new_out(0x16);
        p.write_u8(0);
        self.write_packet(p);
    }


    fn enable_encryption(&mut self, packet: &mut packet::InPacket) {

        // Get all the data from the Encryption Request packet
        let server_id = packet.read_string();
        let key_len = packet.read_be_i16().unwrap();
        let public_key = packet.read_exact(key_len as uint).unwrap();
        let token_len = packet.read_be_i16().unwrap();
        let verify_token = packet.read_exact(token_len as uint).unwrap();

        // Server's public key
        println!("Still alive")
        let mut pk = openssl::crypto::pkey::PKey::new();
        println!("Loading")

        /*let header = "-----begin public key-----";
        let footer = "-----end public key-----";
        let config = base64::Config{char_set: base64::Standard, pad: false, line_length: None};

        let final = String::new().append(header).append(public_key.as_slice().to_base64(config).as_slice()).append(footer);*/

        pk.load_pub_bytes(public_key.as_slice());
        println!("Loaded: {}", public_key.as_slice())

        // Generate random 16 byte key
        let mut key = [0u8, ..16];
        rand::task_rng().fill_bytes(key);

        // Encrypt shared secret with server's public key
        let ekey = pk.encrypt_with_padding(key, pkey::PKCS1v15);
        println!("Encrypted")

        // Encrypt verify token with server's public key
        let etoken = pk.encrypt_with_padding(verify_token.as_slice(), pkey::PKCS1v15);

        // Generate the server id hash
        let mut sha1 = Hasher::new(SHA1);
        sha1.update(server_id.as_bytes());
        sha1.update(key);
        sha1.update(public_key.as_slice());
        let hash = special_digest(sha1);

        println!("Hash: {}", hash);

        // Do client auth
        self.authenticate(hash);

        println!("Authenticated!");

        // Create Encryption Response Packet
        let mut erp = Packet::new_out(0x1);

        println!("Writing");

        // Write encrypted shared secret
        erp.write_be_i16(ekey.len() as i16);
        erp.write(ekey.as_slice());

        println!("And again");

        // Write encrypted verify token
        erp.write_be_i16(etoken.len() as i16);
        erp.write(etoken.as_slice());

        println!("Sending");

        // Send
        self.write_packet(erp);

        println!("Ciphering");

        // Create AES cipher with shared secret
        //let aes = crypto::AES::new(key.to_owned(), key.to_owned()).unwrap();

        // Get the plain TCP stream
        let sock = match self.sock.take_unwrap() {
            Plain(s) => s,
            _ => fail!("Expected plain socket!")
        };

        println!("Wrapping");

        // and wwrap it in an AES Stream
        let sock = crypto::AesStream::new(sock, Vec::from_slice(key));

        // and put the new encrypted stream back
        // everything form this point is encrypted
        //
        self.sock = Some(Encrypted(sock));
        println!("All done");
    }


    fn login(&mut self) {
        self.send_handshake(true);
        self.send_username();

        // Read the next packet and find out whether we need
        // to do authentication and encryption
        let (mut packet_id, mut packet) = self.read_packet();
        println!("Packet ID: {}", packet_id);

        if packet_id == 0x1 {
            // Encryption Request
            // online-mode = true

            self.enable_encryption(&mut packet);

            // Read the next packet...
            println!("About to read packet");
            let (pi, p) = self.read_packet();
            println!("Read");
            packet_id = pi;
            packet = p;
        }

        if packet_id == 0x0 {
            // Disconnect

            let reason = packet.read_string();
            println!("Reason: {}", reason);
            fail!("Received disconnect.");
        }

        // Login Success
        assert_eq!(packet_id, 0x2);
        let uuid = packet.read_string();
        let username = packet.read_string();

        println!("UUID: {}", uuid);
        println!("Username: {}", username);
    }
    
    pub fn status(&mut self) {
        self.send_handshake(false);

        // Send the status request
        self.write_packet(Packet::new_out(0x0));

        // and read back the response
        let (packet_id, mut packet) = self.read_packet();

        // Make sure we got the right response
        assert_eq!(packet_id, 0x0);

        println!("Packet: {}", packet.read_string())
    }

    fn send_handshake(&mut self, login: bool) {
        let mut p = Packet::new_out(0x0);

        // Protocol Version
        p.write_varint(5);

        // Server host
        p.write_string(self.host.as_slice());

        // Server port
        p.write_be_u16(self.port);

        // State
        // 1 - status, 2 - login
        p.write_varint(if login { 2 } else { 1 });

        self.write_packet(p);
    }

    fn write_packet(&mut self, p: packet::OutPacket) {
        // Get the actual buffer
        let buf = p.buf();

        // Write out the packet length
        self.sock.write_varint(buf.len() as i32);
        //
        /*let l = buf.len() as i32;

        match self.sock.unwrap() {
            Plain(refs) => {
                s.write_varint(l);
                s.write(buf.as_slice());
            }
            Encrypted(s) => {
                s.write_varint(l);
                s.write(buf.as_slice());
            }
        };*/

        // and the actual payload
        self.sock.write(buf.as_slice());
    }

    fn read_packet(&mut self) -> (i32, packet::InPacket) {
        // Read the packet length
        //println!("Reading length")
        let len = self.sock.read_varint();

        // Now the payload
        //println!("Reading payload")
        let buf = match self.sock.read_exact(len as uint) {
            Ok(d) => d,
            Err(err) => fail!("Error: {} - {}", err.kind.to_string(), err.desc)//return Err(format!("{} - {}", err.kind.to_string(), err.desc))
        };

        //println!("Buf: {}, {}", len, buf);

        let mut p = Packet::new_in(buf);

        //println!("Packet");

        // Get the packet id
        let id = p.read_varint();

        //println!("Id")

        (id, p)
    }

}

impl Reader for Sock {
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        match *self {
            Plain(ref mut s) => s.read(buf),
            Encrypted(ref mut s) => s.read(buf)
        }
    }
}

impl Writer for Sock {
    fn write(&mut self, buf: &[u8]) -> io::IoResult<()> {
        match *self {
            Plain(ref mut s) => s.write(buf),
            Encrypted(ref mut s) => s.write(buf)
        }
    }

    fn flush(&mut self) -> io::IoResult<()> {
        match *self {
            Plain(ref mut s) => s.flush(),
            Encrypted(ref mut s) => s.flush()
        }
    }
}

impl Reader for Option<Sock> {
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        match *self {
            Some(ref mut s) => s.read(buf),
            None => Err(io::standard_error(io::OtherIoError))
        }
    }
}

impl Writer for Option<Sock> {
    fn write(&mut self, buf: &[u8]) -> io::IoResult<()> {
        match *self {
            Some(ref mut s) => s.write(buf),
            None => Err(io::standard_error(io::OtherIoError))
        }
    }

    fn flush(&mut self) -> io::IoResult<()> {
        match *self {
            Some(ref mut s) => s.flush(),
            None => Err(io::standard_error(io::OtherIoError))
        }
    }
}
