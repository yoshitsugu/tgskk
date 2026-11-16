use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tgskk_core::input::{CskkDictionary, CskkKeyEvent, InputSession};
use tgskk_zenz::{Assistance, Evaluator, Status, Worker};

fn percentile(times: &mut [f64], percent: usize) -> f64 {
    times.sort_by(f64::total_cmp);
    times[((times.len() - 1) * percent).div_ceil(100)]
}
#[allow(clippy::arc_with_non_send_sync)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 && args.len() != 6 {
        return Err(
            "Usage: evaluate WORKER MODEL CASES.tsv INTERKEY_MS [DICTIONARY ENCODING]".into(),
        );
    }
    let mut evaluator = Evaluator::open(Path::new(&args[0]), Path::new(&args[1]))?;
    let cases = std::fs::read_to_string(&args[2])?;
    let mut correct = 0;
    let mut total = 0;
    let mut times = Vec::new();
    for line in cases
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 {
            return Err("Invalid case row".into());
        }
        let candidates: Vec<_> = fields[3].split('|').map(String::from).collect();
        evaluator.score(fields[0], "", "", &[])?;
        let start = Instant::now();
        let scores: Vec<_> = evaluator
            .score(fields[0], "", fields[1], &candidates)?
            .into_iter()
            .map(|(base, _)| base)
            .collect();
        times.push(start.elapsed().as_secs_f64() * 1000.);
        let best = scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .unwrap()
            .0;
        total += 1;
        correct += usize::from(candidates[best] == fields[2]);
        println!(
            "{} + {} => {} (expected {}), scores={scores:?}",
            fields[0], fields[1], candidates[best], fields[2]
        );
    }
    println!(
        "scorer_accuracy={correct}/{total}, scoring_p50_ms={:.3}, scoring_p95_ms={:.3}",
        percentile(&mut times, 50),
        percentile(&mut times, 95)
    );
    drop(evaluator);
    let worker = Worker::start(args[0].clone().into(), args[1].clone().into())?;
    let deadline = Instant::now() + Duration::from_secs(6);
    while worker.status() == Status::Loading && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    if worker.status() != Status::Ready {
        return Err("Model unavailable".into());
    }
    let delay: u64 = args[3].parse()?;
    let default_dictionary = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/SKK-JISYO.zenz.utf8"
    );
    let dictionary = args.get(4).map_or(default_dictionary, String::as_str);
    let encoding = args.get(5).map_or("utf-8", String::as_str);
    let dictionary = Arc::new(CskkDictionary::new_static_dict(dictionary, encoding, true)?);
    let mut spaces = Vec::new();
    let mut keys = Vec::new();
    let (mut hits, mut misses, mut promoted, mut stable) = (0, 0, 0, 0);
    for _ in 0..10 {
        for (context, expected, trace) in [
            ("駅で", "汽車", "K i s h a space"),
            ("新聞の", "記者", "K i s h a space"),
            ("会社に", "帰社", "K i s h a space"),
            ("取引先である", "貴社", "K i s h a space"),
            ("大学教授の", "講演", "K o u e n space"),
        ] {
            let mut input = InputSession::with_dictionaries(vec![dictionary.clone()])?;
            let assistance = Assistance::attach(&mut input, worker.clone());
            assistance.context(context, "", true);
            for name in trace.split_whitespace() {
                let event: CskkKeyEvent = name.parse()?;
                let start = Instant::now();
                let _ = input.process_key(&event);
                assistance.prefetch(&input);
                let elapsed = start.elapsed().as_secs_f64() * 1000.;
                if name == "space" {
                    spaces.push(elapsed);
                } else {
                    keys.push(elapsed);
                }
                if delay > 0 {
                    std::thread::sleep(Duration::from_millis(delay));
                }
            }
            let first = input.candidates()[0].text.clone();
            promoted += usize::from(first == expected);
            // Late inference may populate the cache, but must not change a
            // candidate list already exposed to the user.
            std::thread::sleep(Duration::from_millis(80));
            stable += usize::from(input.candidates()[0].text == first);
            let (h, m) = assistance.counts();
            hits += h;
            misses += m;
        }
    }
    println!(
        "interkey_ms={delay}, ready_hits={hits}, fallback={misses}, expected_first={promoted}/50, frozen_lists={stable}/50"
    );
    println!(
        "space_p50_ms={:.3}, space_p99_ms={:.3}, key_p99_ms={:.3}",
        percentile(&mut spaces, 50),
        percentile(&mut spaces, 99),
        percentile(&mut keys, 99)
    );
    if stable != 50 {
        return Err("An exposed candidate list changed after inference completed".into());
    }
    Ok(())
}
