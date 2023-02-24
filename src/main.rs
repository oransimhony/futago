use clap::Parser;
use native_tls::{TlsConnector, TlsStream};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

const GEMINI_PORT: u16 = 1965;
const DEFAULT_HOST: &'static str = "gemini.circumlunar.space";

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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(default_value_t = DEFAULT_HOST.to_string(), help = "the domain to connect to (without scheme)")]
    domain: String,
    #[arg(default_value_t = GEMINI_PORT, value_parser = clap::value_parser!(u16).range(1..), help = "the port the gemini server runs on")]
    port: u16,
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

fn handle_success(header: &ResponseHeader, stream: &mut TlsStream<TcpStream>) {
    assert!(header.meta.starts_with("text/"), "I only know how to handle text MIME types");
    let body = read_response_body(stream);
    println!("Server returned:\n{body}");
}

fn handle_response_header(header: ResponseHeader, mut stream: TlsStream<TcpStream>) {
    match header.status {
        StatusCodes::Success => handle_success(&header, &mut stream),
        StatusCodes::NotFound => eprintln!("Page not found!"),
        StatusCodes::BadRequest => eprintln!("Oops! Looks like we made a bad request :( please try again."),
        _ => eprintln!("I don't know how to handle {:?}", header.status),
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let domain = cli.domain;
    let mut stream = create_stream(&domain);

    println!("What resource do you want to access on {domain}?: ");
    let resource = if let Some(resource_str) = io::stdin().lines().next() {
        resource_str?
    } else {
        "/".to_owned()
    };
    let uri = build_uri(&domain, &resource);

    send_request(&mut stream, &uri);
    let header = read_response_header(&mut stream);

    handle_response_header(header, stream);

    Ok(())
}
