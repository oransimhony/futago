use native_tls::{TlsConnector, TlsStream};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

const GEMINI_PORT: u16 = 1965;

#[derive(Debug, Clone)]
struct ResponseHeader {
    status: StatusCodes,
    meta: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u8)]
enum StatusCodes {
    /* 1X */
    Input = 10,
    SensitiveInput = 11,
    /* 2X */
    Success = 20,
    /* 3X */
    RedirectTemporary = 30,
    RedirectPermanent = 31,
    /* 4X */
    TemporaryFailure = 40,
    ServerUnavailable = 41,
    CgiError = 42,
    ProxyError = 43,
    SlowDown = 44,
    /* 5X */
    PermanentFailure = 50,
    NotFound = 51,
    Gone = 52,
    ProxyRequestRefused = 53,
    BadRequest = 59,
    /* 6X */
    ClientCertificateRequired = 60,
    CertificateNotAuthorised = 61,
    CertificateNotValid = 62,
}

impl Into<StatusCodes> for u8 {
    fn into(self) -> StatusCodes {
        match self {
            /* 1X */
            10 => StatusCodes::Input,
            11 => StatusCodes::SensitiveInput,
            /* 2X */
            20 => StatusCodes::Success,
            /* 3X */
            30 => StatusCodes::RedirectTemporary,
            31 => StatusCodes::RedirectPermanent,
            /* 4X */
            40 => StatusCodes::TemporaryFailure,
            41 => StatusCodes::ServerUnavailable,
            42 => StatusCodes::CgiError,
            43 => StatusCodes::ProxyError,
            44 => StatusCodes::SlowDown,
            /* 5X */
            50 => StatusCodes::PermanentFailure,
            51 => StatusCodes::NotFound,
            52 => StatusCodes::Gone,
            53 => StatusCodes::ProxyRequestRefused,
            59 => StatusCodes::BadRequest,
            /* 6X */
            60 => StatusCodes::ClientCertificateRequired,
            61 => StatusCodes::CertificateNotAuthorised,
            62 => StatusCodes::CertificateNotValid,
            _ => panic!("unknown status code"),
        }
    }
}

fn create_stream(domain: &str) -> TlsStream<TcpStream> {
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let stream = TcpStream::connect((domain, GEMINI_PORT)).unwrap();
    connector.connect(domain, stream).unwrap()
}

fn build_uri(domain: &str, resource: &str) -> String {
    format!("gemini://{}{}\r\n", domain, resource)
}

fn send_request(stream: &mut TlsStream<TcpStream>, uri: &str) {
    stream.write_all(uri.as_bytes()).expect("write failed");
}

fn read_response_header(stream: &mut TlsStream<TcpStream>) -> ResponseHeader {
    let mut space = String::new();
    let mut status_buf = String::new();
    stream.take(2).read_to_string(&mut status_buf).unwrap();
    stream.take(1).read_to_string(&mut space).unwrap();
    assert_eq!(space, " ".to_owned());
    let buf = BufReader::new(stream);
    ResponseHeader {
        status: status_buf.parse::<u8>().unwrap().into(),
        meta: buf.lines().next().unwrap().unwrap(),
    }
}

fn read_response_body(stream: &mut TlsStream<TcpStream>) -> String {
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    buf
}

fn handle_success(_header: &ResponseHeader, stream: &mut TlsStream<TcpStream>) {
    let body = read_response_body(stream);
    println!("Server returned:\n{body}");
}

fn handle_response_header(header: ResponseHeader, mut stream: TlsStream<TcpStream>) {
    match header.status {
        StatusCodes::Success => handle_success(&header, &mut stream),
        StatusCodes::NotFound => eprintln!("Page not found!"),
        _ => eprintln!("I don't know how to handle {:?}", header.status),
    }
}

fn main() {
    let domain = "gemini.circumlunar.space";
    let mut stream = create_stream(domain);
    let uri = build_uri(domain, "/docs/specification.gmi");

    send_request(&mut stream, &uri);
    let header = read_response_header(&mut stream);

    handle_response_header(header, stream);

}
