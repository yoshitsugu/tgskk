use std::str::FromStr;

use tgskk_skkserv::{
    Candidate, Encoding, Error, MAX_CANDIDATES, MAX_READING_BYTES, MAX_RESPONSE_BYTES,
    decode_response, encode_request,
};

fn candidate(text: &str, annotation: Option<&str>) -> Candidate {
    Candidate {
        text: text.to_string(),
        annotation: annotation.map(str::to_string),
    }
}

fn assert_malformed(frame: &[u8]) {
    assert_eq!(
        decode_response(frame, Encoding::Utf8),
        Err(Error::MalformedResponse),
        "frame {frame:?}"
    );
}

#[test]
fn encodes_known_utf8_wire_bytes() {
    assert_eq!(
        encode_request("か", Encoding::Utf8).unwrap(),
        b"1\xe3\x81\x8b "
    );
    assert_eq!(
        encode_request("日本", Encoding::Utf8).unwrap(),
        b"1\xe6\x97\xa5\xe6\x9c\xac "
    );
    assert_eq!(
        encode_request("ｱ", Encoding::Utf8).unwrap(),
        b"1\xef\xbd\xb1 "
    );
}

#[test]
fn encodes_known_euc_jp_wire_bytes() {
    assert_eq!(
        encode_request("か", Encoding::EucJp).unwrap(),
        b"1\xa4\xab "
    );
    assert_eq!(
        encode_request("日本", Encoding::EucJp).unwrap(),
        b"1\xc6\xfc\xcb\xdc "
    );
    let request = encode_request("ｱ", Encoding::EucJp).unwrap();
    assert_eq!(request, b"1\x8e\xb1 ");
}

#[test]
fn decodes_known_euc_jp_wire_bytes() {
    assert_eq!(
        decode_response(b"1/\xa4\xab/\xa4\xad/\n", Encoding::EucJp).unwrap(),
        vec![candidate("か", None), candidate("き", None)]
    );
    assert_eq!(
        decode_response(b"1/\xc6\xfc\xcb\xdc/\n", Encoding::EucJp).unwrap(),
        vec![candidate("日本", None)]
    );
    assert_eq!(
        decode_response(b"1/\x8e\xb1/\n", Encoding::EucJp).unwrap(),
        vec![candidate("ｱ", None)]
    );
}

#[test]
fn decodes_known_utf8_wire_bytes() {
    assert_eq!(
        decode_response(b"1/\xe3\x81\x8b/\xe3\x81\x8d/\n", Encoding::Utf8).unwrap(),
        vec![candidate("か", None), candidate("き", None)]
    );
    let frame = b"1/\xe6\x97\xa5\xe6\x9c\xac;\xe3\x81\xab\xe3\x81\xbb\xe3\x82\x93/\n";
    let candidates = decode_response(frame, Encoding::Utf8).unwrap();
    assert_eq!(candidates, vec![candidate("日本", Some("にほん"))]);
}

#[test]
fn splits_annotation_at_first_semicolon() {
    assert_eq!(
        decode_response(b"1/a;b;c/\n", Encoding::Utf8).unwrap(),
        vec![candidate("a", Some("b;c"))]
    );
    assert_eq!(
        decode_response(b"1/a;/\n", Encoding::Utf8).unwrap(),
        vec![candidate("a", None)]
    );
}

#[test]
fn preserves_order_and_duplicates() {
    assert_eq!(
        decode_response(b"1/a/a/b/\n", Encoding::Utf8).unwrap(),
        vec![
            candidate("a", None),
            candidate("a", None),
            candidate("b", None)
        ]
    );
    assert_eq!(
        decode_response(b"1/\xa4\xab/\xa4\xab/\xa4\xad/\n", Encoding::EucJp).unwrap(),
        vec![
            candidate("か", None),
            candidate("か", None),
            candidate("き", None)
        ]
    );
}

#[test]
fn miss_variants_return_empty() {
    for frame in [&b"4\n"[..], &b"4\r\n"[..], &b"1/\n"[..]] {
        assert_eq!(
            decode_response(frame, Encoding::Utf8).unwrap(),
            Vec::<Candidate>::new(),
            "frame {frame:?}"
        );
    }
}

#[test]
fn echoed_miss_returns_empty() {
    assert_eq!(
        decode_response(b"4x\n", Encoding::Utf8).unwrap(),
        Vec::<Candidate>::new()
    );
    assert_eq!(
        decode_response(b"4\xe3\x81\x8b \n", Encoding::Utf8).unwrap(),
        Vec::<Candidate>::new()
    );
    assert_eq!(
        decode_response(b"4\xa4\xab \n", Encoding::EucJp).unwrap(),
        Vec::<Candidate>::new()
    );
    assert_eq!(
        decode_response(b"4 \r\n", Encoding::Utf8).unwrap(),
        Vec::<Candidate>::new()
    );
}

#[test]
fn standalone_zero_is_server_error() {
    for frame in [&b"0"[..], &b"0\n"[..], &b"0\r\n"[..]] {
        assert_eq!(
            decode_response(frame, Encoding::Utf8),
            Err(Error::ServerError),
            "frame {frame:?}"
        );
    }
}

#[test]
fn rejects_malformed_frames() {
    assert_malformed(b"");
    assert_malformed(b"\n");
    assert_malformed(b"4");
    assert_malformed(b"5/a/\n");
    assert_malformed(b"1\n");
    assert_malformed(b"1/a");
    assert_malformed(b"1//\n");
    assert_malformed(b"1/a//\n");
    assert_malformed(b"1/a/\n\n");
    assert_malformed(b"1/a\nb/\n");
    assert_malformed(b"1/a\rb/\n");
    assert_malformed(b"1/a\x00/\n");
    assert_malformed(b"1/a\x07/\n");
    assert_malformed(b"4\x00\n");
    assert_malformed(b"4\t\n");
}

#[test]
fn rejects_invalid_encoded_bytes() {
    for encoding in [Encoding::Utf8, Encoding::EucJp] {
        assert_eq!(
            decode_response(b"1/\xff/\n", encoding),
            Err(Error::InvalidResponseEncoding),
            "encoding {encoding:?}"
        );
    }
    assert_eq!(
        decode_response(b"1/\xe3\x81/\n", Encoding::Utf8),
        Err(Error::InvalidResponseEncoding)
    );
    assert_eq!(
        decode_response(b"1/\x8e\xa0/\n", Encoding::EucJp),
        Err(Error::InvalidResponseEncoding)
    );
}

#[test]
fn rejects_oversized_response() {
    let frame = vec![b'1'; MAX_RESPONSE_BYTES + 1];
    assert_eq!(
        decode_response(&frame, Encoding::Utf8),
        Err(Error::ResponseTooLarge)
    );
}

#[test]
fn rejects_too_many_candidates() {
    let mut frame = String::from("1/");
    for _ in 0..=MAX_CANDIDATES {
        frame.push_str("a/");
    }
    frame.push('\n');
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8),
        Err(Error::TooManyCandidates)
    );
}

#[test]
fn accepts_maximum_candidates() {
    let mut frame = String::from("1/");
    for _ in 0..MAX_CANDIDATES {
        frame.push_str("a/");
    }
    frame.push('\n');
    let candidates = decode_response(frame.as_bytes(), Encoding::Utf8).unwrap();
    assert_eq!(candidates.len(), MAX_CANDIDATES);
}

#[test]
fn parses_encoding_names_strictly() {
    assert_eq!(Encoding::from_str("euc-jp"), Ok(Encoding::EucJp));
    assert_eq!(Encoding::from_str("EUC-JP"), Ok(Encoding::EucJp));
    assert_eq!(Encoding::from_str("Utf-8"), Ok(Encoding::Utf8));
    assert_eq!(Encoding::from_str("UTF-8"), Ok(Encoding::Utf8));
    for value in ["", "utf8", "euc_jp", "shift_jis", "utf-8 ", " euc-jp"] {
        assert_eq!(
            Encoding::from_str(value),
            Err(Error::InvalidEncodingName),
            "value {value:?}"
        );
    }
}

#[test]
fn rejects_invalid_readings() {
    let empty = encode_request("", Encoding::Utf8);
    assert_eq!(empty, Err(Error::EmptyReading));
    for reading in [" ", "か な", "か\n", "a\u{0}", "\u{3000}"] {
        assert_eq!(
            encode_request(reading, Encoding::Utf8),
            Err(Error::InvalidReadingCharacter),
            "reading {reading:?}"
        );
    }
}

#[test]
fn enforces_reading_length_limit() {
    let too_long = "a".repeat(MAX_READING_BYTES + 1);
    assert_eq!(
        encode_request(&too_long, Encoding::Utf8),
        Err(Error::ReadingTooLong)
    );

    let at_limit = "a".repeat(MAX_READING_BYTES);
    assert_eq!(
        encode_request(&at_limit, Encoding::Utf8).unwrap().len(),
        MAX_READING_BYTES + 2
    );
}

#[test]
fn rejects_unrepresentable_euc_jp() {
    assert_eq!(
        encode_request("😀", Encoding::EucJp),
        Err(Error::UnrepresentableReading)
    );
    assert_eq!(
        encode_request("あ😀", Encoding::EucJp),
        Err(Error::UnrepresentableReading)
    );
    assert_eq!(
        encode_request("😀", Encoding::Utf8).unwrap(),
        b"1\xf0\x9f\x98\x80 "
    );
}

#[test]
fn decodes_canonical_concat_expressions() {
    let frame = "1/(concat \"DOS\\057V\")/(concat \"a\\073b\")/(concat \"x\\0571y\")/\n";
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8).unwrap(),
        vec![
            candidate("DOS/V", None),
            candidate("a;b", None),
            candidate("x/1y", None),
        ]
    );
}

#[test]
fn decodes_quote_and_backslash_escapes() {
    let frame = "1/(concat \"a\\\"b\\\\c\")/\n";
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8).unwrap(),
        vec![candidate("a\"b\\c", None)]
    );
}

#[test]
fn decodes_multiple_arguments_and_unicode_text() {
    let frame = "1/(concat \"日本\" \"\\057\" \"語\")/(concat \"\\343\")/\n";
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8).unwrap(),
        vec![candidate("日本/語", None), candidate("ã", None)]
    );
}

#[test]
fn decodes_annotation_expression_independently() {
    let frame = "1/東京;(concat \"注\\073釈\")/\n";
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8).unwrap(),
        vec![candidate("東京", Some("注;釈"))]
    );
}

#[test]
fn empty_decoded_annotation_becomes_none() {
    let frame = "1/東京;(concat \"\")/\n";
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8).unwrap(),
        vec![candidate("東京", None)]
    );
}

#[test]
fn plain_candidate_text_is_unchanged() {
    let frame = "1/(笑)/dos v/\n";
    assert_eq!(
        decode_response(frame.as_bytes(), Encoding::Utf8).unwrap(),
        vec![candidate("(笑)", None), candidate("dos v", None)]
    );
}

#[test]
fn rejects_malformed_concat_expressions() {
    for frame in [
        "1/(concat)/\n",
        "1/(concat )/\n",
        "1/(concat foo)/\n",
        "1/(concat (concat \"a\"))/\n",
        "1/(concat (foo))/\n",
        "1/(concat \"a\") extra/\n",
        "1/(concat \"a)/\n",
        "1/(concat \"a\" b)/\n",
        "1/(concat \"a\" \"b\"/\n",
        "1/(concat \"a\\x\")/\n",
        "1/(concat \"a\\n\")/\n",
        "1/(concatX)/\n",
        "1/東京;(concat \"a)/\n",
    ] {
        assert_malformed(frame.as_bytes());
    }
}

#[test]
fn rejects_decoded_controls_and_empty_text() {
    for frame in [
        "1/(concat \"\")/\n",
        "1/(concat \"\" \"\")/\n",
        "1/(concat \"a\\012b\")/\n",
        "1/(concat \"\\000\")/\n",
        "1/東京;(concat \"a\\011b\")/\n",
    ] {
        assert_malformed(frame.as_bytes());
    }
}

#[test]
fn allows_spaces_in_candidate_text() {
    assert_eq!(
        decode_response(b"1/a b/\n", Encoding::Utf8).unwrap(),
        vec![candidate("a b", None)]
    );
}

#[test]
fn error_messages_do_not_leak_wire_data() {
    let error = encode_request("秘密\n", Encoding::Utf8).unwrap_err();
    assert!(!error.to_string().contains("秘密"));

    for frame in [&b"secret"[..], &b"1/s\x00ecret/\n"[..]] {
        let error = decode_response(frame, Encoding::Utf8).unwrap_err();
        assert!(!error.to_string().contains("secret"), "frame {frame:?}");
    }
}

#[test]
fn error_is_std_error() {
    fn assert_std_error<E: std::error::Error>() {}
    assert_std_error::<Error>();
}
