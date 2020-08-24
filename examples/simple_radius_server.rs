/*
 * Simpple example of RADIUS server
 * 
 * cargo run --example simple_radius_server 
 */

use radius_rust::protocol::dictionary::Dictionary;
use radius_rust::protocol::radius_packet::{ RadiusAttribute, RadiusPacket, TypeCode };
use radius_rust::server::{ RadiusMsgType, Server };
use radius_rust::tools::ipv6_string_to_bytes;

use std::io::Error;


// Define handlers
fn handle_auth_request(server: &Server, request: &mut [u8]) -> Result<Vec<u8>, Error> {
    let ipv6_bytes = ipv6_string_to_bytes("fc66::1/64").unwrap();
    let attributes = vec![
        server.create_attribute_by_name("Service-Type",       vec![2]).unwrap(),
        server.create_attribute_by_name("Framed-IP-Address",  vec![192, 168,0,1]).unwrap(),
        server.create_attribute_by_name("Framed-IPv6-Prefix", ipv6_bytes).unwrap()
    ];
    let mut reply_packet = RadiusPacket::initialise_packet(TypeCode::AccessAccept, attributes);

    // We can create new authenticator only after we set correct reply packet ID
    reply_packet.override_id(request[1]);

    let authenticator = server.create_reply_authenticator(&mut reply_packet.to_bytes(), request[4..20].to_vec());
    reply_packet.override_authenticator(authenticator);

    Ok(reply_packet.to_bytes())
}

fn handle_acct_request(server: &Server, request: &mut [u8]) -> Result<Vec<u8>, Error> {
    let attributes: Vec<RadiusAttribute> = Vec::with_capacity(1);
    let mut reply_packet                 = RadiusPacket::initialise_packet(TypeCode::AccountingResponse, attributes);

    // We can create new authenticator only after we set correct reply packet ID
    reply_packet.override_id(request[1]);

    let authenticator = server.create_reply_authenticator(&mut reply_packet.to_bytes(), request[4..20].to_vec());
    reply_packet.override_authenticator(authenticator);

    Ok(reply_packet.to_bytes())
}

fn handle_coa_request(server: &Server, request: &mut [u8]) -> Result<Vec<u8>, Error> {
    let attributes: Vec<RadiusAttribute> = Vec::with_capacity(1);
    let mut reply_packet                 = RadiusPacket::initialise_packet(TypeCode::CoAACK, attributes);

    // We can create new authenticator only after we set correct reply packet ID
    reply_packet.override_id(request[1]);

    let authenticator = server.create_reply_authenticator(&mut reply_packet.to_bytes(), request[4..20].to_vec());
    reply_packet.override_authenticator(authenticator);

    Ok(reply_packet.to_bytes())
}
// ------------------------


fn main() {
    let dictionary = Dictionary::from_file("./dict_examples/integration_dict").unwrap();
    let mut server = Server::initialise_server(1812, 1813, 3799, &dictionary, String::from("127.0.0.1"), String::from("secret"), 1, 2).unwrap();

    server.add_allowed_hosts(String::from("127.0.0.1"));

    server.add_request_handler(RadiusMsgType::AUTH, handle_auth_request).unwrap();
    server.add_request_handler(RadiusMsgType::ACCT, handle_acct_request).unwrap();
    server.add_request_handler(RadiusMsgType::COA,  handle_coa_request).unwrap();

    server.run_server().unwrap();
}
