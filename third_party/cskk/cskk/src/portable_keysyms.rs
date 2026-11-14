pub use xkeysym::Keysym;
include!("portable_keysyms_data.rs");
#[cfg(windows)]
pub const KEYSYM_NO_FLAGS: u32 = 0;

pub fn keysym_from_name(name: &str, _flags: u32) -> Keysym {
    if let Ok(index) = NAMES.binary_search_by_key(&name, |entry| entry.0) {
        return Keysym::from(NAMES[index].1);
    }
    if let Some(hex) = name.strip_prefix('U') {
        if (4..=8).contains(&hex.len()) {
            if let Ok(value) = u32::from_str_radix(hex, 16) {
                if let Some(ch) = char::from_u32(value) {
                    return if ch == '\0' {
                        Keysym::NoSymbol
                    } else if (ch as u32) < 0x100 {
                        Keysym::from_char(ch)
                    } else {
                        Keysym::from(0x01000000 | ch as u32)
                    };
                }
            }
        }
    }
    if let Some(hex) = name.strip_prefix("0x") {
        if let Ok(value) = u32::from_str_radix(hex, 16) {
            return Keysym::from(value);
        }
    }
    Keysym::NoSymbol
}
pub fn keysym_get_name(key: Keysym) -> String {
    if let Some((name, _)) = NAMES.iter().find(|(_, value)| *value == key.raw()) {
        return (*name).to_owned();
    }
    key.name()
        .map(|name| {
            name.strip_prefix("XK_")
                .map(str::to_owned)
                .unwrap_or_else(|| name.replace("XF86XK_", "XF86"))
        })
        .unwrap_or_else(|| match key.key_char() {
            Some(ch) if !ch.is_control() => format!("U{:04X}", ch as u32),
            _ => format!("0x{:08x}", key.raw()),
        })
}
pub fn keysym_to_utf8(key: Keysym) -> String {
    match key.raw() {
        keysyms::KEY_XF86Numeric0..=keysyms::KEY_XF86Numeric9 => {
            ((b'0' + (key.raw() - keysyms::KEY_XF86Numeric0) as u8) as char).to_string()
        }
        keysyms::KEY_XF86NumericStar => "*".into(),
        keysyms::KEY_XF86NumericPound => "#".into(),
        _ => key.key_char().map(|ch| ch.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod portable_contract_tests {
    use super::*;

    #[test]
    fn protocol_names_and_unicode_values_are_portable() {
        for (name, value, text) in [
            ("Return", 0xff0d, "\r"),
            ("a", 0x61, "a"),
            ("Greek_alpha", 0x07e1, "α"),
            ("U03B1", 0x010003b1, "α"),
            ("U3042", 0x01003042, "あ"),
            ("U0001F600", 0x0101f600, "😀"),
            ("XF86Numeric1", 0x10081201, "1"),
        ] {
            let key = keysym_from_name(name, 0);
            assert_eq!(key.raw(), value, "{name}");
            assert_eq!(keysym_to_utf8(key), text, "{name}");
            assert_eq!(keysym_from_name(&keysym_get_name(key), 0), key, "{name}");
        }
        for name in ["nonexistent-key", "U00110000", "U0000"] {
            assert_eq!(keysym_from_name(name, 0), Keysym::NoSymbol, "{name}");
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn names_and_characters_match_native_keysyms() {
        let mut differences = Vec::new();
        let mut newer_names = Vec::new();
        for &(name, value) in NAMES {
            let native = xkbcommon::xkb::keysym_from_name(name, xkbcommon::xkb::KEYSYM_NO_FLAGS);
            let key = Keysym::from(value);
            assert_eq!(keysym_from_name(name, 0), key, "{name}");
            if native == Keysym::NoSymbol && key != Keysym::NoSymbol {
                newer_names.push(name);
            } else {
                assert_eq!(key, native, "{name}");
            }
            let native_text = xkbcommon::xkb::keysym_to_utf8(key);
            let old_numeric_mapping = native_text.is_empty()
                && (keysyms::KEY_XF86Numeric0..=keysyms::KEY_XF86NumericPound).contains(&value);
            if keysym_to_utf8(key) != native_text && !old_numeric_mapping {
                differences.push((name, value, native_text));
            }
            assert_eq!(keysym_from_name(&keysym_get_name(key), 0), key, "{name}");
        }
        if !newer_names.is_empty() {
            eprintln!("Names newer than the installed libxkbcommon: {newer_names:?}");
        }
        assert!(differences.is_empty(), "{:?}", differences);
        for (offset, expected) in "0123456789*#".chars().enumerate() {
            assert_eq!(
                keysym_to_utf8(Keysym::from(keysyms::KEY_XF86Numeric0 + offset as u32)),
                expected.to_string()
            );
        }
        for value in 0..=255 {
            let name = format!("U{:04X}", value);
            let portable = keysym_from_name(&name, 0);
            let expected = match value {
                0 => 0,
                0x08..=0x0b | 0x0d | 0x1b => 0xff00 | value,
                0x7f => 0xffff,
                0x20..=0x7e | 0xa0..=0xff => value,
                _ => 0x01000000 | value,
            };
            assert_eq!(portable.raw(), expected, "{name}");
            let native = xkbcommon::xkb::keysym_from_name(&name, 0);
            // Older releases rejected Unicode names for control characters.
            if native != Keysym::NoSymbol || value == 0 {
                assert_eq!(portable, native, "{name}");
            }
        }
        for name in [
            "U0041",
            "U00E9",
            "U3042",
            "U03B1",
            "U0100",
            "U20AC",
            "U0001F600",
            "U00110000",
            "0xff0d",
            "nonexistent-key",
        ] {
            assert_eq!(
                keysym_from_name(name, 0),
                xkbcommon::xkb::keysym_from_name(name, 0),
                "{name}"
            );
        }
    }
}
