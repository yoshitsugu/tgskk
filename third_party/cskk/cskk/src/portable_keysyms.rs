pub use xkeysym::Keysym;
include!("portable_keysyms_data.rs");
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn names_and_characters_match_native_keysyms() {
        let mut differences = Vec::new();
        for &(name, value) in NAMES {
            let native = xkbcommon::xkb::keysym_from_name(name, xkbcommon::xkb::KEYSYM_NO_FLAGS);
            assert_eq!(keysym_from_name(name, 0), native, "{name}");
            let key = Keysym::from(value);
            if keysym_to_utf8(key) != xkbcommon::xkb::keysym_to_utf8(key) {
                differences.push((name, value, xkbcommon::xkb::keysym_to_utf8(key)));
            }
            assert_eq!(keysym_from_name(&keysym_get_name(key), 0), key, "{name}");
        }
        assert!(differences.is_empty(), "{:?}", differences);
        for value in 0..=255 {
            let name = format!("U{:04X}", value);
            assert_eq!(
                keysym_from_name(&name, 0),
                xkbcommon::xkb::keysym_from_name(&name, 0),
                "{}",
                name
            );
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
