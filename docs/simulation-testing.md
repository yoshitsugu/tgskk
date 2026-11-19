# Deterministic scoring simulation

Run the bounded seed corpus:

```sh
cargo test -p tgskk-zenz --lib simulation --locked
```

Replay a failing seed (PowerShell: set `$env:TGSKK_DST_SEED = '42'` first):

```sh
TGSKK_DST_SEED=42 cargo test -p tgskk-zenz --lib deterministic_scoring_schedules --locked -- --nocapture
```

The simulator uses a virtual integer clock, a seeded PRNG and one event queue.
Inference completion can occur before, after, or at the same virtual time as a
context change, reset, privacy change, prefetch, or candidate display.
Successful scoring, timeouts and rejected requests are injected without a model,
real threads, sockets or sleeps. Equal-time events use seeded priorities.
Failures report the seed and event trace; replay requires the same code version.

Tests exercise the production worker's dequeue eligibility and completion
functions, the real pending/cache storage, Assistance and CSKK input session.
They check that obsolete work is discarded, stale completions are not cached or
retried, failed work does not replace newer requests, valid results are kept,
cached jobs are not rescored, refusals leave pending work unchanged, timeouts
requeue live work when the pending slot is empty, and dictionary conversion
uses the available matching score or dictionary order.
The default 256-seed corpus asserts that key race scenarios were actually reached.
The Rust quality workflow runs this corpus on Linux and Windows.

This follows the approach of [FoundationDB simulation testing](https://apple.github.io/foundationdb/engineering.html)
at the scoring-service boundary. It does not simulate OS scheduling, mutex
interleavings, process supervision/backoff, real inference, file I/O or Windows
TSF/COM. Native integration and thread-based tests remain necessary for those
boundaries. A separate fixed-order test displays dictionary-order candidates
with Space, selects a non-first candidate, then completes the queued production
job while the list is shown. It checks that the saved list and cursor are
unchanged, that Return still commits the saved string, and that the late score
would have reordered the list if applied earlier. Arbitrary key sequences on an
already displayed list remain uncovered.
More seeds cover more schedules; this is not exhaustive verification.
