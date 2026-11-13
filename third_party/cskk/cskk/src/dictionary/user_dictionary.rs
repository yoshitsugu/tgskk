use crate::dictionary::candidate::Candidate;
use crate::dictionary::file_dictionary::{load_dictionary, DictionaryEntries, FileDictionary};
use crate::dictionary::{CompositeKey, DictEntry, Dictionary};
use crate::error::CskkError;
use crate::error::CskkError::Error;
use encoding_rs::{Encoder, EncoderResult, Encoding};
use lru_ordered_map::LruOrderedMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tempfile::NamedTempFile;

///
/// User dictionary that can load from file and save entries to file.
///
///
#[derive(Debug)]
pub(crate) struct UserDictionary {
    file_path: String,
    encode: String,
    // Midashi -> DictEntry map
    okuri_ari_dictionary: LruOrderedMap<String, DictEntry>,
    okuri_nashi_dictionary: LruOrderedMap<String, DictEntry>,
    // Just bool, because we know this is under mutex.
    has_change: bool,
}

const BUF_SIZE: usize = 1024;

impl UserDictionary {
    pub(crate) fn new(file_path: &str, encode: &str) -> Result<Self, CskkError> {
        let dictionary = load_dictionary(file_path, encode.as_bytes())?;

        Ok(UserDictionary {
            file_path: String::from(file_path),
            encode: encode.to_string(),
            okuri_ari_dictionary: dictionary.okuri_ari,
            okuri_nashi_dictionary: dictionary.okuri_nashi,
            has_change: false,
        })
    }
}

impl Dictionary for UserDictionary {
    fn lookup(&self, composite_key: &CompositeKey) -> Option<&DictEntry> {
        if composite_key.has_okuri() {
            self.okuri_ari_dictionary
                .peek(&composite_key.get_dict_key())
        } else {
            self.okuri_nashi_dictionary
                .peek(&composite_key.get_dict_key())
        }
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn complete<'a>(
        &'a self,
        midashi_head: &'a CompositeKey,
    ) -> Box<dyn Iterator<Item = &'a DictEntry> + 'a> {
        FileDictionary::complete(self, midashi_head)
    }

    /// 書き込みを完了してから元のファイルを置き換え、直前の内容を.BAKに残す。
    /// 辞書ファイルのフォーマットは SKK 16.2 user manual 5.10.7 辞書の書式 に依る
    /// userdictなので送りありエントリも送りなしエントリも最近使用した順に並ぶ。
    fn save_dictionary(&mut self) -> Result<bool, CskkError> {
        if self.has_change {
            let parent = Path::new(&self.file_path)
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let mut temporary = NamedTempFile::new_in(parent)?;
            let mut stream = BufWriter::new(temporary.as_file_mut());
            let mut enc = Encoding::for_label(self.encode.as_bytes())
                .expect("It should be same as encoding name succeeded when loading file.")
                .new_encoder();

            // Not using. Can't compile on mac.
            // let encoded = encode_string(
            //     &mut enc,
            //     &format!(
            //         ";; Save on {} \n",
            //         chrono::offset::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
            //     ),
            // )?;
            // stream.write_all(encoded.as_slice())?;

            let encoded = encode_string(&mut enc, ";; okuri-ari entries.\n")?;
            stream.write_all(encoded.as_slice())?;
            for (_, dictentry) in self.okuri_ari_dictionary.iter_lru() {
                // midashi is ignored here because dictentry holds the same.
                if let Some(dict_entry) = dictentry {
                    let mut dict_entry_string = dict_entry.to_string();
                    dict_entry_string += "\n";
                    let encoded = encode_string(&mut enc, dict_entry_string.as_mut_str())?;
                    stream.write_all(encoded.as_slice())?;
                }
            }
            let encoded = encode_string(&mut enc, ";; okuri-nasi entries.\n")?;
            stream.write_all(encoded.as_slice())?;
            for (_, dictentry) in self.okuri_nashi_dictionary.iter_lru() {
                if let Some(dict_entry) = dictentry {
                    let mut dict_entry_string = dict_entry.to_string();
                    dict_entry_string += "\n";
                    let encoded = encode_string(&mut enc, dict_entry_string.as_mut_str())?;
                    stream.write_all(encoded.as_slice())?;
                }
            }
            stream.flush()?;
            drop(stream);
            temporary.as_file().sync_all()?;
            // Keep the live file in place until the atomic replacement is ready.
            // A missing live file is recreated from the in-memory dictionary.
            match File::open(&self.file_path) {
                Ok(mut original) => {
                    let mut backup = NamedTempFile::new_in(parent)?;
                    io::copy(&mut original, backup.as_file_mut())?;
                    backup.as_file().sync_all()?;
                    backup
                        .persist(format!("{}.BAK", self.file_path))
                        .map_err(|error| error.error)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            temporary
                .persist(&self.file_path)
                .map_err(|error| error.error)?;
            self.has_change = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn select_candidate(&mut self, candidate: &Candidate) -> Result<bool, CskkError> {
        let midashi = &candidate.midashi;
        log::debug!("Select midashi: {:?}", midashi);
        let dictionary = if candidate.okuri {
            &mut self.okuri_ari_dictionary
        } else {
            &mut self.okuri_nashi_dictionary
        };

        let entry = dictionary.get_mut(midashi);
        match entry {
            Some(dict_entry) => {
                dict_entry.prioritize_candidate(candidate);
            }
            None => {
                dictionary.push(
                    candidate.midashi.to_owned(),
                    DictEntry::new(&candidate.midashi, candidate),
                );
            }
        }
        self.has_change = true;
        Ok(true)
    }

    fn purge_candidate(
        &mut self,
        composite_key: &CompositeKey,
        candidate: &Candidate,
    ) -> Result<bool, CskkError> {
        let dictionary = if candidate.okuri {
            &mut self.okuri_ari_dictionary
        } else {
            &mut self.okuri_nashi_dictionary
        };
        let midashi = &candidate.midashi;
        let entry = dictionary.get_mut(midashi);
        if let Some(dict_entry) = entry {
            dict_entry.remove_matching_candidate(composite_key, candidate);
        }
        self.has_change = true;
        Ok(true)
    }

    fn reload(&mut self) -> Result<(), CskkError> {
        FileDictionary::reload(self)
    }
}

impl FileDictionary for UserDictionary {
    fn file_path(&self) -> &str {
        &self.file_path
    }

    fn encode(&self) -> &str {
        &self.encode
    }

    fn set_dictionary(&mut self, dictionary: DictionaryEntries) {
        self.okuri_ari_dictionary = dictionary.okuri_ari;
        self.okuri_nashi_dictionary = dictionary.okuri_nashi;
    }

    fn get_okuri_nashi_dictionary(&self) -> &LruOrderedMap<String, DictEntry> {
        &self.okuri_nashi_dictionary
    }

    fn get_okuri_ari_dictionary(&self) -> &LruOrderedMap<String, DictEntry> {
        &self.okuri_ari_dictionary
    }
}

fn encode_string(encoder: &mut Encoder, to_encode: &str) -> Result<Vec<u8>, CskkError> {
    let mut encoded_vec = Vec::with_capacity(BUF_SIZE);
    let mut source = to_encode;
    let mut tmp_buf = Vec::with_capacity(BUF_SIZE);
    loop {
        let (result, read) =
            encoder.encode_from_utf8_to_vec_without_replacement(source, &mut tmp_buf, true);
        if read == 0 {
            return Err(Error(
                "Cannot read on encoding. Give up whole string.".to_string(),
            ));
        }
        match result {
            EncoderResult::Unmappable(_char) => {
                return Err(Error("Encoding failed. Give up whole string.".to_string()));
            }
            EncoderResult::InputEmpty => {
                encoded_vec.append(&mut tmp_buf);
                break;
            }
            EncoderResult::OutputFull => {
                encoded_vec.append(&mut tmp_buf);
                source = &source[read..];
            }
        }
    }
    Ok(encoded_vec)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unencodable_candidate_does_not_replace_or_truncate_the_live_dictionary() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("user.dict");
        let original = b";; okuri-ari entries.\n;; okuri-nasi entries.\n";
        std::fs::write(&path, original).unwrap();
        let mut dictionary = UserDictionary::new(path.to_str().unwrap(), "euc-jp").unwrap();
        let candidate =
            Candidate::new("えもじ".into(), None, false, "😀".into(), None, "😀".into());
        dictionary.select_candidate(&candidate).unwrap();
        assert!(dictionary.save_dictionary().is_err());
        assert!(dictionary.has_change);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!directory.path().join("user.dict.BAK").exists());
    }
    use encoding_rs_io::DecodeReaderBytesBuilder;
    use std::io::{BufRead, BufReader};
    use tempfile::NamedTempFile;

    #[test]
    fn userdict() -> Result<(), CskkError> {
        let file = NamedTempFile::new()?;
        let filename = file.path().to_str().unwrap();
        let mut user_dictionary = UserDictionary::new(filename, "utf-8")?;
        let candidate = Candidate::new(
            "あああ".to_string(),
            None,
            false,
            "アアア".to_string(),
            Some("wow".to_string()),
            "アアア".to_string(),
        );
        let composite_key = CompositeKey::new("あああ", None);
        user_dictionary.select_candidate(&candidate)?;
        user_dictionary.save_dictionary()?;
        user_dictionary.purge_candidate(&composite_key, &candidate)?;
        user_dictionary.save_dictionary()?;
        Ok(())
    }

    /// Recent select_candidate の順序になっているか
    #[test]
    fn userdict_ordering() -> Result<(), CskkError> {
        let file = NamedTempFile::new()?;
        let filename = file.path().to_str().unwrap();
        let mut user_dictionary = UserDictionary::new(filename, "utf-8")?;
        let candidate = Candidate::new(
            "あ".to_string(),
            None,
            false,
            "候補".to_string(),
            None,
            "候補".to_string(),
        );
        user_dictionary.select_candidate(&candidate)?;
        let candidate = Candidate::new(
            "い".to_string(),
            None,
            false,
            "候補".to_string(),
            None,
            "候補".to_string(),
        );
        user_dictionary.select_candidate(&candidate)?;

        let ab_candidate = Candidate::new(
            "あb".to_string(),
            Some("ば".to_string()),
            true,
            "候補".to_string(),
            None,
            "候補".to_string(),
        );
        user_dictionary.select_candidate(&ab_candidate)?;

        let ib_candidate = Candidate::new(
            "いb".to_string(),
            Some("ば".to_string()),
            true,
            "候補".to_string(),
            None,
            "候補".to_string(),
        );
        user_dictionary.select_candidate(&ib_candidate)?;
        user_dictionary.select_candidate(&ab_candidate)?;
        user_dictionary.save_dictionary()?;

        let saved_file = File::open(filename)?;
        let enc = Encoding::for_label_no_replacement("utf-8".as_bytes());
        let decoder = DecodeReaderBytesBuilder::new()
            .encoding(enc)
            .build(saved_file);
        let reader = BufReader::new(decoder);
        for (i, line) in reader.lines().enumerate() {
            if let Ok(line) = line {
                match i {
                    0 => {
                        assert_eq!(line, ";; okuri-ari entries.");
                    }
                    1 => {
                        assert_eq!(line, "あb /候補/[ば/候補/]/");
                    }
                    2 => {
                        assert_eq!(line, "いb /候補/[ば/候補/]/");
                    }
                    3 => {
                        assert_eq!(line, ";; okuri-nasi entries.");
                    }
                    4 => {
                        assert_eq!(line, "い /候補/");
                    }
                    5 => {
                        assert_eq!(line, "あ /候補/");
                    }
                    _ => {}
                }
            }
        }
        user_dictionary.save_dictionary()?;
        Ok(())
    }
}
