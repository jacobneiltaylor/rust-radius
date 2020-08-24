use super::protocol::host::Host;
use super::protocol::radius_packet::{ RadiusPacket, RadiusAttribute, TypeCode };
use super::protocol::dictionary::Dictionary;

use crypto::digest::Digest;
use crypto::md5::Md5;
use mio::{ Events, Interest, Poll, Token };
use mio::net::UdpSocket;
use std::collections::HashMap;
use std::fmt;
use std::io::{Error, ErrorKind};


const AUTH_SOCKET: Token = Token(1);
const ACCT_SOCKET: Token = Token(2);
const COA_SOCKET:  Token = Token(3);


#[derive(PartialEq, Eq, Hash)]
pub enum RadiusMsgType {
    AUTH,
    ACCT,
    COA
}

impl fmt::Display for RadiusMsgType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            RadiusMsgType::AUTH => f.write_str("Auth"),
            RadiusMsgType::ACCT => f.write_str("Acct"),
            RadiusMsgType::COA  => f.write_str("CoA"),
        }
    }
}


pub struct Server<'server> {
    host:      Host<'server>,
    allowed_hosts: Vec<String>,
    server:        String,
    secret:        String,
    retries:       u16,
    timeout:       u16,
    socket_poll:   Poll,
    handlers:      HashMap<RadiusMsgType, fn(server: &Server,request: &mut [u8])->Result<Vec<u8>, Error>>
}

impl<'server> fmt::Debug for Server<'server> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
         .field("host",          &self.host)
         .field("allowed_hosts", &self.allowed_hosts)
         .field("server",        &self.server)
         .field("secret",        &self.server)
         .field("retries",       &self.retries)
         .field("timeout",       &self.timeout)
         .field("socket_poll",   &self.socket_poll)
         .finish()
    }
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
                socket_poll:   Poll::new()?,
                handlers:      HashMap::with_capacity(3)
            }
        )
    }

    pub fn add_allowed_hosts(&mut self, host_addr: String) {
        self.allowed_hosts.push(host_addr);
    }

    pub fn add_request_handler(&mut self, handler_type: RadiusMsgType, handler_function: fn(server: &Server,request: &mut [u8])->Result<Vec<u8>, Error>) -> Result<(), Error> {
        match handler_type {
            RadiusMsgType::AUTH => {
                self.handlers.insert(handler_type, handler_function);
                Ok(())
            },
            RadiusMsgType::ACCT => {
                self.handlers.insert(handler_type, handler_function);
                Ok(())
            },
            RadiusMsgType::COA  => {
                self.handlers.insert(handler_type, handler_function);
                Ok(())
            }
        }
    }

    pub fn create_attribute_by_name(&self, attribute_name: &str, value: Vec<u8>) -> Result<RadiusAttribute, Error> {
        RadiusAttribute::create_by_name(&self.host.dictionary, attribute_name, value).ok_or(Error::new(ErrorKind::Other, format!("Failed to create: {:?} attribute", attribute_name)))
    }

    pub fn create_reply_authenticator(&self, raw_reply_packet: &mut Vec<u8>, mut request_authenticator: Vec<u8>) -> Vec<u8> {
        // We need to create authenticator as MD5 hash (similar to how client verifies server reply)
        let mut temp: Vec<u8> = Vec::new();

        temp.append(&mut raw_reply_packet[0..4].to_vec());  // Append reply's   type code, reply ID and reply length
        temp.append(&mut request_authenticator);            // Append request's authenticator 
        temp.append(&mut raw_reply_packet[20..].to_vec());  // Append reply's   attributes
        temp.append(&mut self.secret.as_bytes().to_vec());  // Append server's  secret. Possibly it should be client's secret, which sould be stored together with allowed hostnames ?

        let mut md5_hasher    = Md5::new();
        let mut authenticator = [0; 16];
        
        md5_hasher.input(&temp);
        md5_hasher.result(&mut authenticator);
        // ----------------
        authenticator.to_vec()
    }

    pub fn run_server(&mut self) -> Result<(), Error> {
        let auth_bind_addr = format!("{}:{}", &self.server, self.host.get_port(&TypeCode::AccessRequest)).parse().map_err(|e| Error::new(ErrorKind::Other, e))?;
        let acct_bind_addr = format!("{}:{}", &self.server, self.host.get_port(&TypeCode::AccountingRequest)).parse().map_err(|e| Error::new(ErrorKind::Other, e))?;
        let coa_bind_addr  = format!("{}:{}", &self.server, self.host.get_port(&TypeCode::CoARequest)).parse().map_err(|e| Error::new(ErrorKind::Other, e))?;
        
        let mut auth_server = UdpSocket::bind(auth_bind_addr)?;
        let mut acct_server = UdpSocket::bind(acct_bind_addr)?;
        let mut coa_server  = UdpSocket::bind(coa_bind_addr)?;
        
        self.socket_poll.registry().register(&mut auth_server, AUTH_SOCKET, Interest::READABLE)?;
        self.socket_poll.registry().register(&mut acct_server, ACCT_SOCKET, Interest::READABLE)?;
        self.socket_poll.registry().register(&mut coa_server,  COA_SOCKET,  Interest::READABLE)?;

        let mut events = Events::with_capacity(1024);
        
        loop {
            self.socket_poll.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    AUTH_SOCKET => loop {
                        println!("Received AUTH request");
                        let mut request = [0; 4096];
                        
                        match auth_server.recv_from(&mut request) {
                            Ok((packet_size, source_address)) => {
                                if self.host_allowed(&source_address) {
                                    let handle_auth_request = self.handlers.get(&RadiusMsgType::AUTH).expect("Auth handler is not defined!");
                                    let response            = handle_auth_request(&self, &mut request[..packet_size])?;
                                    auth_server.send_to(&response.as_slice(), source_address)?;
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
                    ACCT_SOCKET => loop {
                        println!("Received ACCT request");
                        let mut request = [0; 4096];
                        
                        match acct_server.recv_from(&mut request) {
                            Ok((packet_size, source_address)) => {
                                if self.host_allowed(&source_address) {
                                    let handle_acct_request = self.handlers.get(&RadiusMsgType::ACCT).expect("Acct handler is not defined!");
                                    let response            = handle_acct_request(&self, &mut request[..packet_size])?;
                                    
                                    acct_server.send_to(&response.as_slice(), source_address)?;
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
                    COA_SOCKET  => loop {
                        println!("Received CoA  request");
                        let mut request = [0; 4096];
                        
                        match coa_server.recv_from(&mut request) {
                            Ok((packet_size, source_address)) => {
                                if self.host_allowed(&source_address) {
                                    let handle_coa_request = self.handlers.get(&RadiusMsgType::COA).expect("CoA handler is not defined!");
                                    let response           = handle_coa_request(&self, &mut request[..packet_size])?;
                                    
                                    coa_server.send_to(&response.as_slice(), source_address)?;
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
                    _ => {
                        return Err(Error::new(ErrorKind::Other, format!("Non-supported UDP request: {:?}", event)))
                    }
                }
            }
        }
    }


    fn validate_request(&self, request: &[u8]) -> Result<&[u8], Error> {
        // Step 1. Check that it doesn't contain unsupported attibutes
        todo!();
    }

    fn host_allowed(&self, remote_host: &std::net::SocketAddr) -> bool {
        let remote_host_name            = remote_host.to_string();
        let remote_host_name: Vec<&str> = remote_host_name.split(":").collect();

        self.allowed_hosts.iter().any(|host| host==remote_host_name[0]) 
    }
}
