use std::io;
use std::io::{Error, ErrorKind};

use std::net::TcpStream;
use std::string::String;
use std::str;
use openssl::crypto::hash::{Hasher, Type};
use std::collections::HashMap;
use std::str::FromStr;

use rustc_serialize::json;
use rustc_serialize::json::Json;

use packet;
use crypto;
use openssl;
use rand;
use rand::Rng;
use packet::Packet;
use util::{ReaderExtensions, WriterExtensions, special_digest};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc::Receiver;
use openssl::crypto::pkey;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use term;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use json::ExtraJSON;

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Position {
    x: i64,
    y: i64,
    z: i64
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Entity {
    pos: Position,
    kind: u8
}

pub struct Connection {
    host: String,
    sock: Sock,
    name: String,
    port: u16,
    term: Box<term::StdoutTerminal>,
    entities: HashMap<i64, Entity>
}

enum Sock {
    Plain(TcpStream),
    Encrypted(crypto::AesStream)
}

impl Connection {
    pub fn new(name: &str, host: &str, port: u16) -> Result<Connection, String> {

        println!("Connecting to server");

        let sock = TcpStream::connect((host, port));

        let sock = match sock {
                Ok(s) => s,
                Err(e) => return Err(format!("{:?}", e.kind()))
        };

        println!("Connected!");

        let t = match term::stdout() {
            Some(t) => t,
            None => return Err(String::from("Terminal could not be created"))
        };

        let mut e = HashMap::new();


        Ok(Connection {
            host: String::from(host),
            sock: Sock::Plain(sock),
            name: String::from(name),
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
                Err(e) => panic!("paniced to execute process: {}", e)
            };

        /*match self.read_u8() {
            Ok(v) => v as i32,
            Err(e) => panic!("Error: {}", e)
        }*/
        // write json to stdin and close it
        p.stdin.unwrap().write(format!(r#"
            {{
                "agent": {{
                    "name": "Minecraft",
                    "version": 1
                }},
                "username": "{}",
                "password": "{}"
            }}"#, "Aaron1011", "xxx").as_bytes());
        p.stdin = None;

        // read response
        let out = p.wait_with_output().unwrap().stdout;
        let out = String::from_utf8(out).unwrap();
        println!("Got - {}", out);

        //let json = ExtraJSON::new(Json::from_str(out).unwrap());
        let json = Json::from_str(out.as_str()).unwrap();
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
                Err(e) => panic!("paniced to execute process: {}", e)
            };


        // write json to stdin and close it
        p.stdin.unwrap().write(format!(r#"
            {{
                "accessToken": "{}",
                "selectedProfile": "{}",
                "serverId": "{}"
            }}"#, token, profile, hash).as_bytes());
        p.stdin = None;

        println!("Starting");

        // read response
        p.wait_with_output();//.unwrap().output;
        println!("Done");
        //let out = str::from_utf8_owned(out.move_iter().collect()).unwrap();
        //println!("Got - {}", out);
    }

    fn read_messages(&self) -> Receiver<String> {
        let (chan, port) = mpsc::channel();

        thread::spawn(move || {
            println!("Type message and then [ENTER] to send:");

            let mut stdin = BufReader::new(io::stdin());
            for line in stdin.lines() {
                chan.send(line.unwrap_or(String::from("")));
            }
                /*match line {
                    Ok(text) => chan.send(text),
                    _ => chan.send(String::from("")),
                }*/
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
                        p.write_string(msg.as_str());
                        self.write_packet(p);
                    }
                    Err(ref err) => {
                        match err {
                            Empty => break 'msg,
                            //_ => panic!("Input disconnected")
                        }
                    }
                    /*Err(err) => match err {
                        com
                        println!("Food")m::TryRecvErr => 'msg,
                        _ => continue
                    }*/
                    //comm::Disconnected => panic!("input stream disconnected")
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
            let x = packet.read_i32::<BigEndian>().unwrap();

            // Need to respond
            let mut resp = Packet::new_out(0x0);
            resp.write_i32::<BigEndian>(x);
            self.write_packet(resp);

        // Chat Message
        } else if packet_id == 0x01 {
            println!("Joined!!");
            let id = packet.read_int::<BigEndian>(4).unwrap();
            println!("Id");
            let gamemode = packet.read_uint::<BigEndian>(1).unwrap();
            println!("Gamemode");
            let dimension = packet.read_int::<BigEndian>(1).unwrap();
            println!("Dimm");
            let difficulty = packet.read_uint::<BigEndian>(1).unwrap();
            println!("Diff");
            let players = packet.read_uint::<BigEndian>(1).unwrap();
            println!("Players");
            //let level = packet.read_to_end();
            //println!("Data: {}", level)
            let level = packet.read_string();
            println!("Join game: {:?} {:?} {:?} {:?} {:?} {:?}", id, gamemode, dimension, difficulty, players, level);
        } else if packet_id == 0x06 {
            println!("Food");
            let health = packet.read_f32::<BigEndian>().unwrap();
            let food = packet.read_int::<BigEndian>(2).unwrap();
            let sat = packet.read_f32::<BigEndian>().unwrap();
            println!("Health: {}, Food: {}, Saturation: {}", health, food, sat);
        } else if packet_id == 0x0F {
            println!("Mob spawn!");
            let id = packet.read_varint();
            let type_ = packet.read_uint::<BigEndian>(1).unwrap();

            let pos = Position{x: 0, y: 0, z: 0};
            let entity = Entity{pos: pos, kind: type_ as u8};
            println!("Type of entity: {}", type_);

            self.entities.insert(id as i64, entity);

            let mut p = Packet::new_out(0x02);
            p.write_i32::<BigEndian>(id);
            p.write_i8(1 as i8);
            println!("Hitting: {}", type_);
            self.write_packet(p);

            println!("Id: {}, Type: {}", id, type_);
        } else if packet_id == 0x15 {
            let id = packet.read_int::<BigEndian>(4).unwrap();
            /*let entity: &Entity = {
                self.get_entity(id as i64);
                //(&mut (self.entities)).get(&(id as i64)).unwrap()
            };*/

            let entity = self.get_entity(&id);

            let mut p = Packet::new_out(0x02);
            p.write_i32::<BigEndian>(id as i32);
            p.write_i8(1 as i8);
            println!("Hitting - don't move!: {}", entity.kind);
            self.write_packet(p);
        } else if packet_id == 0x13 {
            let count = packet.read_varint();
            println!("Removing: {}", count);
        } else if packet_id == 0x05 {
            println!("Spawn coord");
            let x = packet.read_int::<BigEndian>(4).unwrap();
            let y = packet.read_int::<BigEndian>(4).unwrap();
            let z = packet.read_int::<BigEndian>(4).unwrap();
            println!("Spawn: {} {} {}", x, y, z);
        } else if packet_id == 0x2 {
            let json = packet.read_string();
            println!("Got chat message: {}", json);

            // Let's wrap up the Json so that we can
            // deal with it more easily
            let j = Json::from_str(json.as_str()).unwrap();
            //let j = ExtraJSON::new(j);

            let ty = match j.find(&String::from("translate")) {
                Some(json) => json.as_string().unwrap(),
                _ => ""
            };

            // Player Chat
            if "chat.type.text" == ty {

                let user = j.find(&String::from("with")).unwrap().as_array().unwrap().get(0).unwrap().find(&String::from("text")).unwrap().as_string().unwrap();
                let msg  = j.find(&String::from("with")).unwrap().as_array().unwrap().get(1).unwrap().as_string().unwrap();

                self.term.fg(term::color::BRIGHT_GREEN);
                //write!(&mut self.term as &mut Writer, "<{}> ", user);
                self.term.write(user.as_bytes());
                self.term.reset();

                self.term.write(msg.as_bytes());
                self.term.write(b"\n");

            // Server Message
            } else if "chat.type.announcement" == ty {

                let msg = j.find(&String::from("with")).unwrap().as_array().unwrap().get(1).unwrap().find(&String::from("extra")).unwrap().as_array().unwrap();
                let mut msg_vec = Vec::new();
                msg.iter().map(|x| msg_vec.push(x.as_string().unwrap()));
                let msg = msg_vec.concat();

                self.term.fg(term::color::BRIGHT_YELLOW);
                self.term.write(b"[Server] ");
                self.term.reset();

                self.term.write(msg.as_bytes());
                self.term.write(b"\n");

            }
        }
    }

    fn get_entity(&self, id: &i64) -> &Entity {
        return self.entities.get(id).unwrap()
    }

    fn send_username(&mut self) {
        let mut p = Packet::new_out(0x0);
        p.write_string(self.name.as_str());

        self.write_packet(p);
    }

    fn respawn(&mut self) {
        println!("Respawning!");
        let mut p = Packet::new_out(0x16);
        p.write_u8(0);
        self.write_packet(p);
    }


    fn enable_encryption(&mut self, packet: &mut packet::InPacket) {

        // Get all the data from the Encryption Request packet
        let server_id = packet.read_string();
        let key_len = packet.read_i16::<BigEndian>().unwrap();
        let public_key = packet.read_len(key_len as u64);
        let token_len = packet.read_i16::<BigEndian>().unwrap();
        let verify_token = packet.read_len(token_len as u64);

        // Server's public key
        println!("Still alive");
        let mut pk = openssl::crypto::pkey::PKey::new();
        println!("Loading");

        /*let header = "-----begin public key-----";
        let footer = "-----end public key-----";
        let config = base64::Config{char_set: base64::Standard, pad: false, line_length: None};

        let final = String::new().append(header).append(public_key.as_str().to_base64(config).as_str()).append(footer);*/

        pk.load_pub(&public_key);
        println!("Loaded: {:?}", &public_key);

        // Generate random 16 byte key
        let mut key = [0u8, 16];
        rand::thread_rng().fill_bytes(&mut key);

        // Encrypt shared secret with server's public key
        let ekey = pk.encrypt_with_padding(&key, pkey::EncryptionPadding::PKCS1v15);
        println!("Encrypted");

        // Encrypt verify token with server's public key
        let etoken = pk.encrypt_with_padding(&verify_token, pkey::EncryptionPadding::PKCS1v15);

        // Generate the server id hash
        let mut sha1 = Hasher::new(Type::SHA1);
        sha1.write_all(server_id.as_bytes());
        sha1.write_all(&key);
        sha1.write_all(&public_key);
        let hash = special_digest(sha1);

        println!("Hash: {}", hash);

        // Do client auth
        self.authenticate(hash);

        println!("Authenticated!");

        // Create Encryption Response Packet
        let mut erp = Packet::new_out(0x1);

        println!("Writing");

        // Write encrypted shared secret
        erp.write_i16::<BigEndian>(ekey.len() as i16);
        erp.write(ekey.as_slice());

        println!("And again");

        // Write encrypted verify token
        erp.write_i16::<BigEndian>(etoken.len() as i16);
        erp.write(etoken.as_slice());

        println!("Sending");

        // Send
        self.write_packet(erp);

        println!("Ciphering");

        // Create AES cipher with shared secret
        //let aes = crypto::AES::new(key.to_owned(), key.to_owned()).unwrap();

        // Get the plain TCP stream
        let sock = match self.sock {
            Sock::Plain(ref s) => s,
            _ => panic!("Expected plain socket!")
        };

        println!("Wrapping");

        // and wwrap it in an AES Stream
        let mut vec = Vec::new();
        vec.extend_from_slice(&key);
        let sock = crypto::AesStream::new(*sock, vec);

        // and put the new encrypted stream back
        // everything form this point is encrypted
        //
        self.sock = Sock::Encrypted(sock);
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
            panic!("Received disconnect.");
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
        p.write_string(self.host.as_str());

        // Server port
        p.write_u16::<BigEndian>(self.port);

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
                s.write(buf.as_str());
            }
            Encrypted(s) => {
                s.write_varint(l);
                s.write(buf.as_str());
            }
        };*/

        // and the actual payload
        self.sock.write(buf.as_slice());
    }

    fn read_packet(&mut self) -> (i32, packet::InPacket) {
        // Read the packet length
        //println!("Reading length")
        let len = self.sock.read_varint();

        let mut buf = Vec::new();
        self.sock.take(len as u64).read_to_end(&mut buf);


        // Now the payload
        //println!("Reading payload")
        /*let buf = match self.sock.read_exact(len as u8) {
            Ok(d) => d,
            Err(err) => panic!("Error: {} - {}", err.kind.to_string(), err.desc)//return Err(format!("{} - {}", err.kind.to_string(), err.desc))
        };*/

        //println!("Buf: {}, {}", len, buf);

        let mut p = Packet::new_in(buf);

        //println!("Packet");

        // Get the packet id
        let id = p.read_varint();

        //println!("Id")

        (id, p)
    }

}

impl Read for Sock {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            Sock::Plain(ref mut s) => s.read(buf),
            Sock::Encrypted(ref mut s) => s.read(buf)
        }
    }
}

impl Write for Sock {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            Sock::Plain(ref mut s) => s.write(buf),
            Sock::Encrypted(ref mut s) => s.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Sock::Plain(ref mut s) => s.flush(),
            Sock::Encrypted(ref mut s) => s.flush()
        }
    }
}

/*impl ReaderExtensions for Option<Sock> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<u64> {
        match *self {
            Some(ref mut s) => s.read(buf),
            None => Err(Error::new(ErrorKind::Other, "error!"))
        }
    }
}

impl WriterExtensions for Option<Sock> {
    fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        match *self {
            Some(ref mut s) => s.write(buf),
            None => Err(Error::new(ErrorKind::Other, "error!"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Some(ref mut s) => s.flush(),
            None => Err(Error::new(ErrorKind::Other, "error!"))
        }
    }
}*/
