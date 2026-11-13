use std::{env, error::Error, fs, process::ExitCode};
use tgskk_core::input::{CskkKeyEvent, InputSession};
use tgskk_core::{Dictionary, Engine};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if let [command, dictionary_path, encoding, events] = args.as_slice()
        && command == "keys"
    {
        let mut input = InputSession::open(dictionary_path, encoding)?;
        for name in events.split_whitespace() {
            let outcome = input.process_key(&name.parse::<CskkKeyEvent>()?);
            println!(
                "{name}: preedit={:?} commit={:?} mode={:?}",
                outcome.preedit,
                outcome.committed,
                outcome.mode_change.map(|change| change.label())
            );
            if let Some(error) = outcome.conversion_error {
                return Err(error.into());
            }
        }
        return Ok(());
    }
    let [dictionary_path, reading] = args.as_slice() else {
        return Err("usage: tgskk <UTF-8 dictionary> <reading>\n       tgskk keys <dictionary> <encoding> '<space-separated key names>'".into());
    };
    let dictionary = Dictionary::parse_utf8(&fs::read_to_string(dictionary_path)?)?;
    let mut engine = Engine::new(dictionary, 0);
    for candidate in engine.lookup(reading).iter() {
        match &candidate.annotation {
            Some(annotation) => println!("{}\t{}", candidate.text, annotation),
            None => println!("{}", candidate.text),
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
