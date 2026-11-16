use std::path::Path;
use std::time::Instant;
use tgskk_zenz::{Evaluator, mixed};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 6 {
        return Err(
            "Usage: tgskk-zenz-eval WORKER MODEL LEFT_CONTEXT RIGHT_CONTEXT KATAKANA CANDIDATE..."
                .into(),
        );
    }
    let mut evaluator = Evaluator::open(Path::new(&args[0]), Path::new(&args[1]))?;
    let candidates = &args[5..];
    evaluator.score(&args[2], &args[3], "", &[])?;
    let start = Instant::now();
    let scores = evaluator.score(&args[2], &args[3], &args[4], candidates)?;
    eprintln!("scoring_ms={:.3}", start.elapsed().as_secs_f64() * 1000.0);
    println!("base\tright\tmix\tcandidate");
    for (candidate, score) in candidates.iter().zip(scores) {
        println!(
            "{:.6}\t{:.6}\t{:.6}\t{candidate}",
            score.0,
            score.1,
            mixed(score)
        );
    }
    Ok(())
}
