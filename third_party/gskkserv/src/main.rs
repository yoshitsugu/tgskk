#[cfg(unix)]
use daemonize::Daemonize;
use docopt::Docopt;
use encoding::all::{EUC_JP, UTF_8};
use encoding::{DecoderTrap, EncoderTrap, Encoding};
use log::{debug, error};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

mod cache;
mod error;

use crate::cache::{Cache, LockedCache, new_cache};
use crate::error::SearchError;

enum RequestCode {
    Disconnect,
    Convert,
    Version,
    Name,
    Completion,
    Invalid(u8),
}

impl From<u8> for RequestCode {
    fn from(code: u8) -> Self {
        use RequestCode::*;
        match code {
            b'0' => Disconnect,
            b'1' => Convert,
            b'2' => Version,
            b'3' => Name,
            b'4' => Completion,
            code => Invalid(code),
        }
    }
}

const SERVER_VERSION: &str = concat!("Google IME SKK Server in Rust.", env!("CARGO_PKG_VERSION"));
#[cfg(unix)]
const PID_DIR: &str = "/tmp/gskkserv.pid";
#[cfg(unix)]
const WORK_DIR: &str = "/tmp";
const GOOGLE_IME_URL: &str = "https://www.google.com/transliterate";

const JSON_ERROR_MSG: &str = "Cannot find expected json structure";
const MAX_STDIO_READING: usize = 4096;
const MAX_STDIO_CANDIDATES: usize = 256;
const MAX_STDIO_OUTPUT: usize = 64 * 1024;
const MAX_CANDIDATE_BYTES: usize = 1024;
const MAX_API_BODY_BYTES: usize = 1024 * 1024;
const API_TIMEOUT: Duration = Duration::from_secs(2);

static HTTP_CLIENT: LazyLock<Result<reqwest::blocking::Client, reqwest::Error>> =
    LazyLock::new(|| {
        reqwest::blocking::Client::builder()
            .timeout(API_TIMEOUT)
            .build()
    });

fn request_api(kana: &str, endpoint: &str) -> Result<String, SearchError> {
    let client = match &*HTTP_CLIENT {
        Ok(client) => client,
        Err(_) => return Err(SearchError::Msg("Cannot initialize the Google IME client")),
    };
    // The trailing comma tells the CGI API that the whole reading is one
    // conversion span, preventing resegmentation.
    let text = format!("{kana},");
    let response = client
        .get(endpoint)
        .query(&[("langpair", "ja-Hira|ja"), ("text", text.as_str())])
        .send()?
        .error_for_status()?;
    let mut body = Vec::new();
    response
        .take((MAX_API_BODY_BYTES + 1) as u64)
        .read_to_end(&mut body)?;
    if body.len() > MAX_API_BODY_BYTES {
        return Err(SearchError::Msg("Google IME response is too large"));
    }
    String::from_utf8(body).map_err(|_| SearchError::Msg("Google IME response is not valid UTF-8"))
}

#[cfg(not(test))]
fn search_with_api(kana: &str) -> Result<String, SearchError> {
    request_api(kana, GOOGLE_IME_URL)
}

fn format_response(json: &str, delimiter: &str) -> Result<Vec<u8>, SearchError> {
    let segments: Vec<(String, Vec<String>)> = serde_json::from_str(json)?;
    let (_, candidates) = segments.first().ok_or(JSON_ERROR_MSG)?;
    if candidates.is_empty() {
        return Err(JSON_ERROR_MSG.into());
    }
    let response = format!("1{delimiter}{}{delimiter}\n", candidates.join(delimiter));
    Ok(EUC_JP.encode(&response, EncoderTrap::Ignore).unwrap())
}

fn search(buf: &[u8], delimiter: &str) -> Result<Vec<u8>, SearchError> {
    let kana = EUC_JP.decode(buf, DecoderTrap::Ignore).unwrap();
    format_response(&search_with_api(&kana)?, delimiter)
}

fn load_from_cache_or_search<'a>(
    cache_key: Vec<u8>,
    cache: &'a mut LockedCache,
    delimiter: &str,
) -> &'a [u8] {
    if cache.contains_key(&cache_key) {
        cache.get(&cache_key).unwrap().as_slice()
    } else {
        match search(&cache_key, delimiter) {
            Ok(result) => {
                cache.insert(cache_key.clone(), result.clone());
                cache.get(&cache_key).unwrap().as_slice()
            }
            Err(_) => {
                error!("Google IME lookup failed");
                b"0"
            }
        }
    }
}

fn create_response<'a>(
    buf: &[u8; 512],
    n: usize,
    cache: &'a mut LockedCache,
    host_and_port: &'a str,
) -> &'a [u8] {
    debug!("CODE: {}", buf[0]);
    match RequestCode::from(buf[0]) {
        RequestCode::Disconnect => b"0",
        RequestCode::Convert => load_from_cache_or_search(buf[1..(n - 1)].to_vec(), cache, "/"),
        RequestCode::Version => SERVER_VERSION.as_bytes(),
        RequestCode::Name => host_and_port.as_bytes(),
        RequestCode::Completion => load_from_cache_or_search(buf[1..(n - 1)].to_vec(), cache, " "),
        RequestCode::Invalid(code) => {
            error!("INVALID CODE: {}", code);
            b"0"
        }
    }
}

fn handle_client(mut stream: TcpStream, cache: Cache, host_and_port: &str) {
    loop {
        let mut buf = [0; 512];
        match stream.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                let mut cache = cache.lock().unwrap();
                let result = create_response(&buf, n, &mut cache, host_and_port);
                stream.write_all(result).unwrap();
            }
            Err(err) => {
                error!("{:?}", err);
                stream.write_all(b"0").unwrap();
            }
        }
    }
}

fn listen(args: &docopt::ArgvMap) {
    let host_and_port = format!("{}:{}", args.get_str("--host"), args.get_str("--port"));
    println!("listen on {}", host_and_port);
    let listener = TcpListener::bind(&host_and_port).unwrap();
    let cache = new_cache();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let cache = cache.clone();
                let host_and_port = host_and_port.clone();
                thread::spawn(move || {
                    handle_client(stream, cache, &host_and_port);
                });
            }
            Err(_) => {
                error!("Error!");
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WireEncoding {
    EucJp,
    Utf8,
}

impl WireEncoding {
    fn parse(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("euc-jp") {
            Some(Self::EucJp)
        } else if name.eq_ignore_ascii_case("utf-8") {
            Some(Self::Utf8)
        } else {
            None
        }
    }

    fn decode(self, bytes: &[u8]) -> Option<String> {
        match self {
            Self::EucJp => EUC_JP.decode(bytes, DecoderTrap::Strict).ok(),
            Self::Utf8 => UTF_8.decode(bytes, DecoderTrap::Strict).ok(),
        }
    }

    fn encode(self, text: &str) -> Option<Vec<u8>> {
        match self {
            Self::EucJp => EUC_JP.encode(text, EncoderTrap::Strict).ok(),
            Self::Utf8 => UTF_8.encode(text, EncoderTrap::Strict).ok(),
        }
    }
}

fn read_byte<R: Read>(input: &mut R) -> io::Result<Option<u8>> {
    let mut byte = [0u8; 1];
    loop {
        match input.read(&mut byte) {
            Ok(0) => return Ok(None),
            Ok(_) => return Ok(Some(byte[0])),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

fn read_reading<R: Read>(input: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut reading = Vec::new();
    loop {
        match read_byte(input)? {
            None => return Ok(None),
            Some(b' ') => {
                if reading.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(reading));
            }
            Some(byte) => {
                if reading.len() >= MAX_STDIO_READING {
                    return Ok(None);
                }
                reading.push(byte);
            }
        }
    }
}

fn escape_candidate(text: &str) -> String {
    if !text.contains('/') && !text.contains(';') {
        return text.to_string();
    }
    let mut escaped = String::with_capacity(text.len() + 16);
    escaped.push_str("(concat \"");
    for character in text.chars() {
        match character {
            '/' => escaped.push_str("\\057"),
            ';' => escaped.push_str("\\073"),
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            other => escaped.push(other),
        }
    }
    escaped.push_str("\")");
    escaped
}

fn format_stdio_response(json: &str, encoding: WireEncoding) -> Result<Vec<u8>, SearchError> {
    let segments: Vec<(String, Vec<String>)> = serde_json::from_str(json)?;
    if segments.len() != 1 {
        return Err(JSON_ERROR_MSG.into());
    }
    let (_, candidates) = &segments[0];
    if candidates.is_empty() {
        return Err(JSON_ERROR_MSG.into());
    }

    let mut response = Vec::new();
    response.extend_from_slice(b"1/");
    let mut accepted = 0;
    for candidate in candidates {
        if accepted == MAX_STDIO_CANDIDATES {
            break;
        }
        if candidate.is_empty()
            || candidate.len() > MAX_CANDIDATE_BYTES
            || candidate.chars().any(char::is_control)
        {
            continue;
        }
        let escaped = escape_candidate(candidate);
        let Some(encoded) = encoding.encode(&escaped) else {
            continue;
        };
        if response.len() + encoded.len() + 2 > MAX_STDIO_OUTPUT {
            break;
        }
        response.extend_from_slice(&encoded);
        response.push(b'/');
        accepted += 1;
    }
    response.push(b'\n');
    Ok(response)
}

fn write_error_and_stop<W: Write>(output: &mut W) -> io::Result<()> {
    output.write_all(b"0")?;
    output.flush()
}

fn run_stdio<R: Read, W: Write, F: FnMut(&str) -> Result<String, SearchError>>(
    input: &mut R,
    output: &mut W,
    encoding: WireEncoding,
    mut search: F,
) -> io::Result<()> {
    loop {
        let Some(code) = read_byte(input)? else {
            return Ok(());
        };
        if code == b'0' {
            return Ok(());
        }
        if code != b'1' {
            return write_error_and_stop(output);
        }
        let Some(reading_bytes) = read_reading(input)? else {
            return write_error_and_stop(output);
        };
        let Some(reading) = encoding.decode(&reading_bytes) else {
            return write_error_and_stop(output);
        };
        if reading.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return write_error_and_stop(output);
        }
        let response = match search(&reading) {
            Ok(json) => format_stdio_response(&json, encoding).unwrap_or_else(|_| b"0".to_vec()),
            Err(_) => b"0".to_vec(),
        };
        output.write_all(&response)?;
        output.flush()?;
    }
}

static USAGE: &str = "
Usage:
  gskkserv [--host=<host>] [--port=<port>] [--encoding=<encoding>] [--stdio] [-d]
  gskkserv --help

Options:
  -h --host=<host>       Server Host [default: 0.0.0.0]
  -p --port=<port>       Server Port [default: 55100]
  --encoding=<encoding>  Wire encoding for --stdio (euc-jp or utf-8) [default: euc-jp]
  --stdio                Serve the SKK protocol on stdin/stdout
  -d                     Daemonize (Unix only)
  --help                 Show this help
";

fn main() {
    env_logger::init();
    let args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.parse())
        .unwrap_or_else(|e| e.exit());

    let Some(encoding) = WireEncoding::parse(args.get_str("--encoding")) else {
        eprintln!("Unsupported encoding. Use euc-jp or utf-8.");
        std::process::exit(2);
    };

    if args.get_bool("--stdio") {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut input = stdin.lock();
        let mut output = stdout.lock();
        if let Err(error) = run_stdio(&mut input, &mut output, encoding, search_with_api) {
            eprintln!("gskkserv stdio failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    if encoding != WireEncoding::EucJp {
        eprintln!("TCP mode supports EUC-JP only; use --stdio for other encodings.");
        std::process::exit(2);
    }

    if args.get_bool("-d") {
        #[cfg(not(unix))]
        {
            eprintln!("Daemon mode is only supported on Unix");
            std::process::exit(1);
        }
        #[cfg(unix)]
        let daemonize = Daemonize::new()
            .pid_file(PID_DIR)
            .working_directory(WORK_DIR);
        #[cfg(unix)]
        match daemonize.start() {
            Ok(_) => {
                listen(&args);
            }
            Err(e) => eprintln!("{}", e),
        }
    } else {
        listen(&args);
    }
}

#[cfg(test)]
fn search_with_api(_kana: &str) -> Result<String, SearchError> {
    Ok(r#"[["ともえごぜん",["巴御前","ともえごぜん","トモエゴゼン"]]]"#.to_string())
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    use encoding::all::{EUC_JP, UTF_8};
    use encoding::{DecoderTrap, EncoderTrap, Encoding};

    use super::{
        MAX_STDIO_CANDIDATES, MAX_STDIO_OUTPUT, SERVER_VERSION, WireEncoding, create_response,
        format_stdio_response, run_stdio, search,
    };

    use super::cache::new_cache;

    #[derive(Default)]
    struct FlushRecorder {
        bytes: Vec<u8>,
        flushes: usize,
    }

    impl Write for FlushRecorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    fn run_frame(
        input: &[u8],
        encoding: WireEncoding,
        search: impl FnMut(&str) -> Result<String, super::SearchError>,
    ) -> FlushRecorder {
        let mut input = input;
        let mut output = FlushRecorder::default();
        run_stdio(&mut input, &mut output, encoding, search).expect("stdio run");
        output
    }

    #[test]
    fn rejects_invalid_api_responses() {
        for json in [
            "[]",
            "null",
            "not json",
            "[[]]",
            r#"[["かな",[]]]"#,
            r#"[["かな",[1]]]"#,
        ] {
            assert!(super::format_response(json, "/").is_err(), "{json}");
        }
    }

    #[test]
    fn formats_completion_and_half_width_kana() {
        let response = super::format_response(r#"[["かな",["仮名","ｶﾅ"]]]"#, " ").unwrap();
        assert_eq!(
            EUC_JP.decode(&response, DecoderTrap::Strict).unwrap(),
            "1 仮名 ｶﾅ \n"
        );
    }

    #[test]
    fn api_encodes_query_and_checks_http_status() {
        for status in ["200 OK", "503 Service Unavailable"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}", listener.local_addr().unwrap());
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    request.push_str(&line);
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]"
                );
                stream.write_all(response.as_bytes()).unwrap();
                request
            });
            let result = super::request_api("かな&?#", &endpoint);
            let request = server.join().unwrap();
            assert!(request.starts_with(
                "GET /?langpair=ja-Hira%7Cja&text=%E3%81%8B%E3%81%AA%26%3F%23%2C HTTP/1.1\r\n"
            ));
            if status.starts_with("200") {
                assert_eq!(result.unwrap(), "[]");
            } else {
                assert!(matches!(
                    result,
                    Err(super::SearchError::Request(e))
                        if e.status() == Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
                ));
            }
        }
    }

    #[test]
    fn api_rejects_oversized_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(1)))
                .ok();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let length = super::MAX_API_BODY_BYTES + 1;
            let header =
                format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n");
            if stream.write_all(header.as_bytes()).is_err() {
                return;
            }
            let chunk = vec![b'a'; 64 * 1024];
            let mut remaining = length;
            while remaining > 0 {
                let size = remaining.min(chunk.len());
                if stream.write_all(&chunk[..size]).is_err() {
                    break;
                }
                remaining -= size;
            }
        });
        let result = super::request_api("かな", &endpoint);
        let _ = server.join();
        assert!(
            matches!(result, Err(super::SearchError::Msg(message)) if message.contains("too large"))
        );
    }

    #[test]
    #[ignore = "requires access to the live Google CGI API"]
    fn live_google_api() {
        let json = super::request_api("ともえごぜん", super::GOOGLE_IME_URL).unwrap();
        let response = super::format_response(&json, "/").unwrap();
        let response = EUC_JP.decode(&response, DecoderTrap::Strict).unwrap();
        assert!(response.starts_with("1/"));
        assert!(response.contains("/巴御前/"));
        assert!(response.ends_with("/\n"));
    }

    #[test]
    fn test_search() {
        let utf8_bytes = EUC_JP.encode("ともえごぜん", EncoderTrap::Ignore).unwrap();
        let result = search(utf8_bytes.as_slice(), "/");
        assert!(result.is_ok());
        let euc_jp_str = EUC_JP
            .decode(result.unwrap().as_slice(), DecoderTrap::Ignore)
            .unwrap();
        let utf8_encoded_arr = UTF_8.encode(&euc_jp_str, EncoderTrap::Ignore).unwrap();
        assert_eq!(
            UTF_8
                .decode(utf8_encoded_arr.as_slice(), DecoderTrap::Ignore)
                .unwrap(),
            "1/巴御前/ともえごぜん/トモエゴゼン/\n"
        )
    }

    #[test]
    fn test_create_response() {
        let cache = new_cache();
        assert_eq!(
            create_response(&[b'0'; 512], 1, &mut cache.lock().unwrap(), "0.0.0.0:5555"),
            b"0"
        );
        assert_eq!(
            create_response(&[b'2'; 512], 1, &mut cache.lock().unwrap(), "0.0.0.0:5555"),
            SERVER_VERSION.as_bytes()
        );
        assert_eq!(
            create_response(&[b'3'; 512], 1, &mut cache.lock().unwrap(), "0.0.0.0:5555"),
            b"0.0.0.0:5555"
        );
    }

    #[test]
    fn stdio_serves_sequential_requests_and_flushes_each_response() {
        let json = r#"[["ともえごぜん",["巴御前","ともえごぜん","トモエゴゼン"]]]"#;
        let mut calls = 0;
        let frame = format!("1{} ", "ともえごぜん");
        let mut joined = Vec::new();
        joined.extend_from_slice(frame.as_bytes());
        joined.extend_from_slice(frame.as_bytes());
        let mut input = joined.as_slice();
        let mut output = FlushRecorder::default();
        run_stdio(&mut input, &mut output, WireEncoding::Utf8, |reading| {
            calls += 1;
            assert_eq!(reading, "ともえごぜん");
            Ok(json.to_string())
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(output.flushes, 2);
        assert_eq!(
            output.bytes,
            "1/巴御前/ともえごぜん/トモエゴゼン/\n1/巴御前/ともえごぜん/トモエゴゼン/\n".as_bytes()
        );
    }

    #[test]
    fn stdio_eof_and_disconnect_exit_cleanly() {
        let output = run_frame(b"", WireEncoding::Utf8, |_| {
            panic!("search must not run on EOF")
        });
        assert!(output.bytes.is_empty());
        assert_eq!(output.flushes, 0);

        let output = run_frame(b"0", WireEncoding::Utf8, |_| {
            panic!("search must not run on disconnect")
        });
        assert!(output.bytes.is_empty());
        assert_eq!(output.flushes, 0);
    }

    #[test]
    fn stdio_rejects_unsupported_code_malformed_and_oversized_frames() {
        let mut oversized = b"1".to_vec();
        oversized.resize(super::MAX_STDIO_READING + 2, b'a');
        oversized.push(b' ');
        for input in [
            b"2".as_slice(),
            b"1".as_slice(),
            b"1 ".as_slice(),
            b"1abc".as_slice(),
            oversized.as_slice(),
        ] {
            let output = run_frame(input, WireEncoding::Utf8, |_| {
                panic!("search must not run for a rejected frame")
            });
            assert_eq!(output.bytes, b"0", "input {input:?}");
            assert_eq!(output.flushes, 1);
        }
    }

    #[test]
    fn stdio_rejects_invalid_encoding_bytes() {
        let output = run_frame(b"1\xff ".as_slice(), WireEncoding::EucJp, |_| {
            panic!("search must not run for invalid bytes")
        });
        assert_eq!(output.bytes, b"0");

        let output = run_frame(b"1\xe3\x81 ".as_slice(), WireEncoding::Utf8, |_| {
            panic!("search must not run for invalid bytes")
        });
        assert_eq!(output.bytes, b"0");
    }

    #[test]
    fn stdio_decodes_utf8_and_known_euc_jp_requests() {
        let json = r#"[["か",["仮名","ｶﾅ"]]]"#;

        let output = run_frame(
            b"1\xe3\x81\x8b ".as_slice(),
            WireEncoding::Utf8,
            |reading| {
                assert_eq!(reading, "か");
                Ok(json.to_string())
            },
        );
        assert_eq!(output.bytes, "1/仮名/ｶﾅ/\n".as_bytes());

        let output = run_frame(b"1\xa4\xab ".as_slice(), WireEncoding::EucJp, |reading| {
            assert_eq!(reading, "か");
            Ok(json.to_string())
        });
        assert_eq!(output.bytes, b"1/\xb2\xbe\xcc\xbe/\x8e\xb6\x8e\xc5/\n");
    }

    #[test]
    fn stdio_escapes_frame_delimiters() {
        let json = serde_json::to_string(&vec![("か", vec!["a/b;c\"d\\e", "plain"])]).unwrap();
        let output = run_frame(b"1\xe3\x81\x8b ".as_slice(), WireEncoding::Utf8, |_| {
            Ok(json.clone())
        });
        let expected = format!("1/{}/plain/\n", r#"(concat "a\057b\073c\"d\\e")"#);
        assert_eq!(output.bytes, expected.as_bytes());
    }

    #[test]
    fn stdio_skips_unrepresentable_and_invalid_candidates() {
        let long = "x".repeat(super::MAX_CANDIDATE_BYTES + 1);
        let json = serde_json::to_string(&vec![(
            "か",
            vec!["😀", "仮名", "a\nb", "", long.as_str(), "東京"],
        )])
        .unwrap();
        let output = run_frame(b"1\xa4\xab ".as_slice(), WireEncoding::EucJp, |_| {
            Ok(json.clone())
        });
        assert_eq!(output.bytes, b"1/\xb2\xbe\xcc\xbe/\xc5\xec\xb5\xfe/\n");
    }

    #[test]
    fn stdio_rejects_multi_segment_json() {
        let json = r#"[["あ",["x"]],["い",["y"]]]"#;
        let output = run_frame(b"1a ".as_slice(), WireEncoding::Utf8, |_| {
            Ok(json.to_string())
        });
        assert_eq!(output.bytes, b"0");
        assert_eq!(output.flushes, 1);
    }

    #[test]
    fn stdio_caps_candidates_and_output_size() {
        let many: Vec<String> = (0..MAX_STDIO_CANDIDATES + 44)
            .map(|index| format!("c{index}"))
            .collect();
        let json = serde_json::to_string(&vec![("か", many)]).unwrap();
        let output = run_frame(b"1a ".as_slice(), WireEncoding::Utf8, |_| Ok(json.clone()));
        let slashes = output.bytes.iter().filter(|&&byte| byte == b'/').count();
        assert_eq!(slashes, MAX_STDIO_CANDIDATES + 1);
        assert!(output.bytes.starts_with(b"1/c0/c1/"));
        assert!(output.bytes.ends_with(b"/\n"));

        let large = "a".repeat(1024);
        let large_candidates = vec![large; 80];
        let json = serde_json::to_string(&vec![("か", large_candidates)]).unwrap();
        let output = run_frame(b"1a ".as_slice(), WireEncoding::Utf8, |_| Ok(json.clone()));
        assert!(output.bytes.len() <= MAX_STDIO_OUTPUT);
        let expected = (MAX_STDIO_OUTPUT - 3) / 1025;
        let slashes = output.bytes.iter().filter(|&&byte| byte == b'/').count();
        assert_eq!(slashes, expected + 1);
        assert!(output.bytes.ends_with(b"/\n"));
    }

    #[test]
    fn stdio_continues_after_search_error_with_valid_framing() {
        let json = r#"[["あ",["x"]]]"#;
        let mut calls = 0;
        let mut input = b"1a 1a ".as_slice();
        let mut output = FlushRecorder::default();
        run_stdio(&mut input, &mut output, WireEncoding::Utf8, |_| {
            calls += 1;
            if calls == 1 {
                Err(super::SearchError::Msg("boom"))
            } else {
                Ok(json.to_string())
            }
        })
        .unwrap();
        assert_eq!(output.bytes, b"01/x/\n");
        assert_eq!(output.flushes, 2);
    }

    #[test]
    fn stdio_response_format_enforces_single_segment() {
        assert!(format_stdio_response(r#"[["あ",[]]]"#, WireEncoding::Utf8).is_err());
        assert!(format_stdio_response("[]", WireEncoding::Utf8).is_err());
        assert_eq!(
            format_stdio_response(r#"[["あ",["x"]]]"#, WireEncoding::Utf8).unwrap(),
            b"1/x/\n"
        );
    }

    #[test]
    fn wire_encoding_names_are_case_insensitive_and_strict() {
        assert_eq!(WireEncoding::parse("euc-jp"), Some(WireEncoding::EucJp));
        assert_eq!(WireEncoding::parse("EUC-JP"), Some(WireEncoding::EucJp));
        assert_eq!(WireEncoding::parse("utf-8"), Some(WireEncoding::Utf8));
        assert_eq!(WireEncoding::parse("Utf-8"), Some(WireEncoding::Utf8));
        for name in ["", "utf8", "euc_jp", "shift_jis", "utf-8 "] {
            assert_eq!(WireEncoding::parse(name), None, "{name:?}");
        }
    }
}
