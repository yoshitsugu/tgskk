use std::{hint::black_box, time::Instant};
use tgskk_core::{Dictionary, Engine};

fn measure(name: &str, mut lookup: impl FnMut()) {
    for _ in 0..10_000 {
        lookup();
    }
    let mut samples = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let start = Instant::now();
        for _ in 0..100 {
            lookup();
        }
        samples.push(start.elapsed().as_nanos() as f64 / 100.0);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{name}: batch-mean p50={:.1} ns/op p95={:.1} ns/op p99={:.1} ns/op",
        samples[500], samples[950], samples[990]
    );
}

fn main() {
    let source: String = (0..10_000)
        .map(|i| format!("よみ{i} /候補{i}/別候補{i}/\n"))
        .collect();
    let start = Instant::now();
    let dictionary = Dictionary::parse_utf8(&source).unwrap();
    println!("synthetic 10,000 entries, parse={:?}", start.elapsed());
    measure("dictionary hit", || {
        black_box(dictionary.lookup(black_box("よみ1234")));
    });
    let mut engine = Engine::new(dictionary, 256);
    measure("cached hit", || {
        black_box(engine.lookup(black_box("よみ1234")));
    });
    measure("cached miss", || {
        black_box(engine.lookup(black_box("未登録")));
    });
}
