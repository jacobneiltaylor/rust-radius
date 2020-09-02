use thiserror::Error;


#[derive(Debug, Error)]
/// Represents all errors generated by this library
pub enum RadiusError {
    #[error("Verification failed for incoming Radius packet")]
    /// Error happens, when Radius Packet fails validation
    ValidationError         { error: String },
    #[error("Radius packet is malformed")]
    /// Error happens, when packet has been badly constructed or got corrupted
    MalformedPacket         { error: String },
    #[error("Radius packet attribute is malformed")]
    /// Error happens, when attribute has been badly constructed or got corrupted
    MalformedAttribute      { error: String },
    #[error("Provided IPv6 address is malformed")]
    /// Error happens, when IPv6 Address was badly added to Radius Packet or got corrupted
    MalformedIpAddr         { error: String },
    #[error(transparent)]
    /// Error happens, when there is some sort of connection error between sockets, or socket
    /// cannot bind to the given hostname/port
    SocketConnectionError(#[from] std::io::Error),
    #[error("Invalid socket connection")]
    SocketInvalidConnection { error: String },
    #[error(transparent)]
    /// Error happens, when socket cannot parse given hostname/port
    SocketAddrParseError(#[from] std::net::AddrParseError),
    #[error("Dictionary is malformed or inaccessible")]
    /// Error happens, when dictionary file cannot be parsed
    MalformedDictionary     { error: std::io::Error },
    /// Error happens, when wrong RADIUS Code is supplied
    #[error("Supplied RADIUS Code is not supported by this library")]
    UnsupportedTypeCode     { error: String }
}
