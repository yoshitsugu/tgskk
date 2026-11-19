use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tgskk_core::input::{CskkDictionary, CskkKeyEvent, InputSession};
use tgskk_zenz::{Assistance, Evaluator, Score, Status, Worker};

fn percentile(times: &mut [f64], percent: usize) -> f64 {
    times.sort_by(f64::total_cmp);
    times[((times.len() - 1) * percent).div_ceil(100)]
}
#[allow(clippy::cast_precision_loss)]
fn rate(part: usize, whole: usize) -> String {
    if whole == 0 {
        "n/a".to_owned()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}
fn base_argmax(scores: &[Score]) -> usize {
    scores
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.0.total_cmp(&b.0))
        .map_or(0, |(index, _)| index)
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
    let mut times = Vec::new();
    let mut dictionary_correct = 0;
    let mut left_correct = 0;
    let mut both_correct = 0;
    let mut scorer_correct = 0;
    let mut left_breaks = 0;
    let mut both_breaks_dictionary = 0;
    let mut both_breaks_left = 0;
    let mut left_changes = 0;
    let mut both_changes = 0;
    let mut right_changes = 0;
    let mut total = 0;
    for line in cases
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 && fields.len() != 5 {
            return Err("Invalid case row".into());
        }
        let (left, reading, expected) = (fields[0], fields[1], fields[2]);
        let right = fields.get(4).copied().unwrap_or("");
        let candidates: Vec<_> = fields[3].split('|').map(String::from).collect();
        evaluator.score(left, "", "", &[])?;
        let start = Instant::now();
        let left_scores = evaluator.score(left, "", reading, &candidates)?;
        times.push(start.elapsed().as_secs_f64() * 1000.);
        let left_best = Evaluator::first_candidate(&left_scores);
        let best = if right.is_empty() {
            left_best
        } else {
            evaluator.score(left, right, "", &[])?;
            let scores = evaluator.score(left, right, reading, &candidates)?;
            Evaluator::first_candidate(&scores)
        };
        let dictionary_first = candidates[0].as_str();
        let left_first = candidates[left_best].as_str();
        let both_first = candidates[best].as_str();
        let dictionary_right = dictionary_first == expected;
        let left_right = left_first == expected;
        let both_right = both_first == expected;
        dictionary_correct += usize::from(dictionary_right);
        left_correct += usize::from(left_right);
        both_correct += usize::from(both_right);
        scorer_correct += usize::from(candidates[base_argmax(&left_scores)] == expected);
        left_breaks += usize::from(dictionary_right && !left_right);
        both_breaks_dictionary += usize::from(dictionary_right && !both_right);
        both_breaks_left += usize::from(left_right && !both_right);
        left_changes += usize::from(left_first != dictionary_first);
        both_changes += usize::from(both_first != dictionary_first);
        right_changes += usize::from(both_first != left_first);
        total += 1;
        println!(
            "{left} + {reading} [right={right}] => dictionary={dictionary_first}, left={left_first}, left_right={both_first} (expected {expected}){}",
            if both_first == left_first { "" } else { " *" }
        );
    }
    if total == 0 {
        return Err(format!("No evaluation cases in {}", args[2]).into());
    }
    println!(
        "first_candidate: dictionary={dictionary_correct}/{total} ({}), left={left_correct}/{total} ({}), left_right={both_correct}/{total} ({})",
        rate(dictionary_correct, total),
        rate(left_correct, total),
        rate(both_correct, total)
    );
    println!(
        "candidate_changes: left={left_changes}/{total} ({}), left_right={both_changes}/{total} ({}), right_added={right_changes}/{total} ({})",
        rate(left_changes, total),
        rate(both_changes, total),
        rate(right_changes, total)
    );
    println!(
        "broken_correct: left_vs_dictionary={left_breaks}/{dictionary_correct} ({}), left_right_vs_dictionary={both_breaks_dictionary}/{dictionary_correct} ({}), left_right_vs_left={both_breaks_left}/{left_correct} ({})",
        rate(left_breaks, dictionary_correct),
        rate(both_breaks_dictionary, dictionary_correct),
        rate(both_breaks_left, left_correct)
    );
    println!(
        "scorer_accuracy={scorer_correct}/{total}, scoring_p50_ms={:.3}, scoring_p95_ms={:.3}",
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
