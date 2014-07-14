use std::io;

use std::io::net::tcp::TcpStream;
use std::string::String;

use serialize::json;


use packet;
use packet::Packet;
use util::{ReaderExtensions, WriterExtensions};
use std::io::stdio::StdWriter;
use std::io::{BufferedReader, LineBufferedWriter, Reader, Writer};
use std::comm;
use json::ExtraJSON;
use term;
use term::terminfo;


pub struct Connection {
    host: String,
    sock: TcpStream,
    name: String,
    port: u16,
    term: Box<term::Terminal<term::WriterWrapper>>
}

enum Sock {
    Plain(TcpStream),
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


        Ok(Connection {
            host: String::from_str(host),
            sock: sock,
            name: String::from_str(name),
            port: port,
            term: t
        })
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
                        comm::TryRecvErr => 'msg,
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

            /*self.enable_encryption(&mut packet);

            // Read the next packet...
            let (pi, p) = self.read_packet();
            packet_id = pi;
            packet = p;*/
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

        // and the actual payload
        self.sock.write(buf.as_slice());
    }

    fn read_packet(&mut self) -> (i32, packet::InPacket) {
        // Read the packet length
        let len = self.sock.read_varint();

        // Now the payload
        let buf = self.sock.read_exact(len as uint).unwrap();

        let mut p = Packet::new_in(buf);

        // Get the packet id
        let id = p.read_varint();

        (id, p)
    }

}
