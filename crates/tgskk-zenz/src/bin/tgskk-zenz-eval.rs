use std::path::Path;
use std::time::Instant;
use tgskk_zenz::Evaluator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 5 {
        return Err(
            "Usage: tgskk-zenz-eval WORKER MODEL LEFT_CONTEXT KATAKANA CANDIDATE...".into(),
        );
    }
    let mut evaluator = Evaluator::open(Path::new(&args[0]), Path::new(&args[1]))?;
    let candidates = &args[4..];
    evaluator.score(&args[2], "", &[])?;
    let start = Instant::now();
    let scores = evaluator.score(&args[2], &args[3], candidates)?;
    eprintln!("scoring_ms={:.3}", start.elapsed().as_secs_f64() * 1000.0);
    for (candidate, score) in candidates.iter().zip(scores) {
        println!("{score:.6}\t{candidate}");
    }
    Ok(())
}
