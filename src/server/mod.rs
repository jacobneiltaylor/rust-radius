use super::protocol::host::Host;
use super::protocol::radius_packet::{ RadiusPacket, RadiusAttribute, TypeCode };
use super::protocol::dictionary::Dictionary;

use crypto::digest::Digest;
use crypto::md5::Md5;
use mio::{ Events, Ready, Poll, PollOpt, Token };
use mio::net::UdpSocket;
use std::io::{Error, ErrorKind};
use std::time::Duration;


const AUTH_SOCKET: Token = Token(1);
const ACCT_SOCKET: Token = Token(2);
const COA_SOCKET: Token  = Token(3);

pub trait AuthHandler {

    fn validate_auth_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Check that it doesn't contain unsupported attibutes
        todo!();
    }

    fn handle_auth_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Read in incoming request
        // Step 2. Build a reply packet
        // Step 3. Return it as &[u8]
        todo!();
    }
}

pub trait AcctHandler {
    fn validate_acct_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Check that it doesn't contain unsupported attibutes
        todo!();
    }
    
    fn handle_acct_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Read in incoming request
        // Step 2. Build a reply packet
        // Step 3. Return it as &[u8]
        todo!();
    }
}

pub trait CoaHandler {
    fn validate_coa_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Check that it doesn't contain unsupported attibutes
        todo!();
    }
    
    fn handle_coa_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Read in incoming request
        // Step 2. Build a reply packet
        // Step 3. Return it as &[u8]
        todo!();
    }
}


#[derive(Debug)]
pub struct Server<'server> {
    host:      Host<'server>,
    allowed_hosts: Vec<String>,
    server:        String,
    secret:        String,
    retries:       u16,
    timeout:       u16,
    socket_poll:   Poll
}

impl<'server> Server<'server> {
    pub fn initialise_server(auth_port: u16, acct_port: u16, coa_port: u16, dictionary: &Dictionary, server: String, secret: String, retries: u16, timeout: u16) -> Result<Server, Error> {
        Ok(
            Server {
                host:          Host { auth_port, acct_port, coa_port, dictionary },
                allowed_hosts: Vec::new(),
                server:        server,
                secret:        secret,
                retries:       retries,
                timeout:       timeout,
                socket_poll:   Poll::new()?
            }
        )
    }

    pub fn add_allowed_hosts(&mut self, host_addr: String) {
        self.allowed_hosts.push(host_addr);
    }

    pub fn run_server(&self) -> Result<(), Error> {
        let auth_bind_addr = &format!("{}:{}", &self.server, self.host.get_port(&TypeCode::AccessRequest)).parse().map_err(|e| Error::new(ErrorKind::Other, e))?;
        let acct_bind_addr = &format!("{}:{}", &self.server, self.host.get_port(&TypeCode::AccountingRequest)).parse().map_err(|e| Error::new(ErrorKind::Other, e))?;
        let coa_bind_addr  = &format!("{}:{}", &self.server, self.host.get_port(&TypeCode::CoARequest)).parse().map_err(|e| Error::new(ErrorKind::Other, e))?;
        
        let auth_server = UdpSocket::bind(&auth_bind_addr).unwrap();
        let acct_server = UdpSocket::bind(&acct_bind_addr).unwrap();
        let coa_server  = UdpSocket::bind(&coa_bind_addr).unwrap();
        
        self.socket_poll.register(&auth_server, AUTH_SOCKET, Ready::readable(), PollOpt::edge())?;
        self.socket_poll.register(&acct_server, ACCT_SOCKET, Ready::readable(), PollOpt::edge())?;
        self.socket_poll.register(&coa_server,  COA_SOCKET,  Ready::readable(), PollOpt::edge())?;

        let timeout    = Duration::from_secs(self.timeout as u64);
        let mut events = Events::with_capacity(1024);
        let mut retry  = 0;
        
        loop {
            if retry >= self.retries {
                break;
            }
            
            self.socket_poll.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    AUTH_TOKEN => loop {
                        println!("Received AUTH request");
                        let mut request = [0; 4096];
                        
                        match auth_server.recv_from(&mut request) {
                            Ok((packet_size, source_address)) => {
                                // TODO: handle auth request
                                if self.host_allowed(&source_address) {
                                    println!("{:?}", &request[..packet_size]);
                                    auth_server.send_to(&request[..packet_size], &source_address)?;
                                    break;
                                } else {
                                    println!("{:?} is not listed as allowed", &source_address);
                                    break;
                                }
                            },
                            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                                break;
                            },
                            Err(error) => {
                                return Err(error);
                            }
                        }
                    },
                    ACCT_TOKEN => loop {
                        println!("Received ACCT request");
                        let mut request = [0; 4096];
                        
                        match acct_server.recv_from(&mut request) {
                            Ok((packet_size, source_address)) => {
                                if self.host_allowed(&source_address) {
                                    // TODO: handle acct request
                                    acct_server.send_to(&request[..packet_size], &source_address)?;
                                    break;
                                } else {
                                    println!("{:?} is not listed as allowed", &source_address);
                                    break;
                                }
                            },
                            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                                break;
                            },
                            Err(error) => {
                                return Err(error);
                            }
                        }
                    },
                    COA_TOKEN  => loop {
                        println!("Received CoA  request");
                        let mut request = [0; 4096];
                        
                        match coa_server.recv_from(&mut request) {
                            Ok((packet_size, source_address)) => {
                                if self.host_allowed(&source_address) {
                                    // TODO: handle coa request
                                    coa_server.send_to(&request[..packet_size], &source_address)?;
                                    break;
                                } else {
                                    println!("{:?} is not listed as allowed", &source_address);
                                    break;
                                }
                            },
                            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                                break;
                            },
                            Err(error) => {
                                return Err(error);
                            }
                        }
                    },
                    _ => println!("Non-supported UDP request: {:?}", event)
                }
            }
        }
        Err(Error::new(ErrorKind::Other, "Server failed unexpectedly"))
    }

    fn host_allowed(&self, remote_host: &std::net::SocketAddr) -> bool {
        let remote_host_name            = remote_host.to_string();
        let remote_host_name: Vec<&str> = remote_host_name.split(":").collect();

        self.allowed_hosts.iter().any(|host| host==remote_host_name[0]) 
    }
}
