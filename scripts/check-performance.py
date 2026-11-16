#!/usr/bin/env python3
"""Compare paired release-build input and real-model scoring latencies on Linux."""
import argparse
import contextlib
import json
import math
import os
import platform
from pathlib import Path
import select
import shutil
import statistics
import struct
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
INPUT_METRICS = {
    f'input_{mode}_{case}_{category}_p{percent}_us'
    for mode in ('plain', 'ai')
    for case in ('kana', 'conversion', 'many_candidates')
    for category in (('keys', 'commit') if case == 'kana' else ('keys', 'space', 'commit'))
    for percent in (50, 99)
}
SCORING_METRICS = {
    f'worker_{case}_p{percent}_us'
    for case in ('four', 'eight') for percent in (50, 99)
}


def validate_metrics(values, expected):
    if set(values) != expected:
        raise ValueError(f'Unexpected metrics: missing={expected - set(values)}, extra={set(values) - expected}')
    if any(isinstance(value, bool) or not isinstance(value, (int, float))
           or not math.isfinite(value) or value <= 0 for value in values.values()):
        raise ValueError('Latency samples must be finite and positive')


def compare(baseline, candidate):
    if len(baseline) != len(candidate) or len(baseline) < 7:
        raise ValueError('At least seven complete paired rounds are required')
    expected = INPUT_METRICS | SCORING_METRICS
    for values in [*baseline, *candidate]:
        validate_metrics(values, expected)
    metrics = {}
    for name in sorted(expected):
        before = [row[name] for row in baseline]
        after = [row[name] for row in candidate]
        ratios = [new / old for old, new in zip(before, after)]
        threshold = 1.50 if '_p99_' in name else 1.25
        slower = sum(new > old * threshold and new - old > 1.0
                     for old, new in zip(before, after))
        metrics[name] = {
            'baseline_us': before, 'candidate_us': after,
            'median_ratio': statistics.median(ratios), 'threshold': threshold,
            'slower_pairs': slower,
            'regressed': slower >= math.ceil(len(before) * 0.7),
        }
    return metrics


def read_exact(stream, size):
    deadline = time.monotonic() + 15
    result = b''
    while len(result) < size:
        if not select.select([stream], [], [], max(0, deadline - time.monotonic()))[0]:
            raise TimeoutError('AI worker response deadline exceeded')
        chunk = os.read(stream.fileno(), size - len(result))
        if not chunk:
            raise EOFError('AI worker closed the response stream')
        result += chunk
    return result


@contextlib.contextmanager
def worker_process(executable, model):
    process = subprocess.Popen([str(executable), str(model)], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    try:
        # The baseline worker comes from the previous revision and may still
        # speak the older protocol without right context.
        version, = struct.unpack('<I', read_exact(process.stdout, 4))
        if version not in (0x325A4754, 0x335A4754):
            raise ValueError('AI worker protocol mismatch')
        process.right_context = version == 0x335A4754
        yield process
    finally:
        with contextlib.suppress(BrokenPipeError, OSError):
            process.stdin.close()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        process.stdout.close()


def score(process, context, reading, candidates):
    frame = struct.pack('<I', len(candidates))
    for text in [context, *([''] if process.right_context else []), reading, *candidates]:
        data = text.encode('utf-8')
        frame += struct.pack('<I', len(data)) + data
    process.stdin.write(frame)
    process.stdin.flush()
    count, = struct.unpack('<I', read_exact(process.stdout, 4))
    if count != len(candidates):
        raise ValueError(f'Worker rejected scoring workload: count={count}')
    values = count * (2 if process.right_context else 1)
    scores = struct.unpack('<' + 'd' * values, read_exact(process.stdout, values * 8))
    if not all(math.isfinite(value) for value in scores):
        raise ValueError('AI worker returned a non-finite score')


def scoring_metrics(executable, model):
    result = {}
    with worker_process(executable, model) as worker:
        for name, context, reading, candidates in [
            ('four', '駅に入ってきた', 'キシャ', ['記者', '汽車', '帰社', '貴社']),
            ('eight', '駅に入ってきた汽車と紅葉の写真を撮る老人を取材した新聞の', 'キシャ',
             ['記者', '汽車', '帰社', '貴社', '騎射', '喜捨', '寄車', '希釈']),
        ]:
            times = []
            for iteration in range(20):
                # Match speculative context preparation before the final reading arrives.
                score(worker, context, '', [])
                start = time.perf_counter_ns()
                score(worker, context, reading, candidates)
                elapsed = (time.perf_counter_ns() - start) / 1000
                if iteration >= 4:
                    times.append(elapsed)
            times.sort()
            for percent in (50, 99):
                result[f'worker_{name}_p{percent}_us'] = times[math.ceil((len(times) - 1) * percent / 100)]
    validate_metrics(result, SCORING_METRICS)
    return result


def fixtures(directory):
    dictionary = directory / 'dictionary.utf8'
    entries = [f'よみ{i} /候補{i}/別候補{i}/' for i in range(10000)]
    entries.extend(['きしゃ /記者/汽車/帰社/貴社/',
                    'こう /' + '/'.join(f'候補{i}' for i in range(512)) + '/'])
    dictionary.write_text(';; okuri-ari entries.\n;; okuri-nasi entries.\n'
                          + '\n'.join(entries) + '\n', encoding='utf-8')
    data = directory / 'data'
    shutil.copytree(ROOT / 'third_party/cskk/assets', data / 'libcskk')
    probe = directory / 'server-probe'
    probe.write_text('#!/bin/sh\ntouch "$TGSKK_PERFORMANCE_SERVER_MARKER"\nexit 1\n', encoding='utf-8')
    probe.chmod(0o700)
    env = {
        **os.environ,
        'TGSKK_PERFORMANCE_SERVER_MARKER': str(directory / 'server-called'),
        'XDG_DATA_HOME': str(data), 'XDG_DATA_DIRS': str(data),
        'XDG_CONFIG_HOME': str(directory / 'config'),
    }
    return dictionary, probe, env


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-input', type=Path, required=True)
    parser.add_argument('--candidate-input', type=Path, required=True)
    parser.add_argument('--baseline-worker', type=Path, required=True)
    parser.add_argument('--candidate-worker', type=Path, required=True)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--baseline-revision', required=True)
    parser.add_argument('--candidate-revision', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    samples = {'baseline': [], 'candidate': []}
    with tempfile.TemporaryDirectory(prefix='tgskk-performance-') as temporary:
        dictionary, probe, env = fixtures(Path(temporary))
        for round_index in range(7):
            # Alternating order limits bias from warm-up, CPU frequency and thermal drift.
            order = ('baseline', 'candidate') if round_index % 2 == 0 else ('candidate', 'baseline')
            for variant in order:
                executable = getattr(args, f'{variant}_input').resolve()
                worker = getattr(args, f'{variant}_worker').resolve()
                output = subprocess.check_output([str(executable), str(dictionary), str(worker),
                                                  str(args.model.resolve()), str(probe)], env=env, timeout=90)
                if (Path(temporary) / 'server-called').exists():
                    raise RuntimeError('Local dictionary hits unexpectedly started a SKK server request')
                values = json.loads(output)
                validate_metrics(values, INPUT_METRICS)
                values.update(scoring_metrics(worker, args.model.resolve()))
                samples[variant].append(values)
                print(f'Round {round_index + 1}/7: {variant} complete', flush=True)
    metrics = compare(samples['baseline'], samples['candidate'])
    regressions = [name for name, result in metrics.items() if result['regressed']]
    report = {'baseline_revision': args.baseline_revision,
              'candidate_revision': args.candidate_revision,
              'metrics': metrics, 'regressions': regressions,
              'environment': {'cpu_count': os.cpu_count(), 'machine': platform.machine(),
                              'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
                              'g++': subprocess.check_output(['g++', '--version'], text=True).splitlines()[0]}}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    summary = ['| Metric | Candidate / baseline | Slower pairs |', '| --- | ---: | ---: |']
    for name, result in metrics.items():
        summary.append(f'| {name} | {result["median_ratio"]:.2f}x | {result["slower_pairs"]}/7 |')
    summary.append('\nRegressions: ' + (', '.join(regressions) or 'none'))
    text = '\n'.join(summary) + '\n'
    print(text)
    if os.environ.get('GITHUB_STEP_SUMMARY'):
        with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as stream:
            stream.write(text)
    if regressions:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
