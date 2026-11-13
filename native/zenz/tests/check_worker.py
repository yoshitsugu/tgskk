#!/usr/bin/env python3
"""Exercise the real static worker, framing, and dynamic-backend isolation."""
import math
import os
from pathlib import Path
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import unittest

WORKER, MODEL, PROBE = (Path(arg).resolve() for arg in sys.argv[1:4])
sys.argv = sys.argv[:1]
MAGIC = 0x325A4754
REFUSED = 0xFFFFFFFF


class Worker:
    def __init__(self, cwd=None, env=None):
        self.process = subprocess.Popen([str(WORKER), str(MODEL)], cwd=cwd, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            if self.word() != MAGIC:
                raise AssertionError('Worker protocol version mismatch')
        except BaseException:
            self.close()
            raise

    def read(self, size):
        deadline = time.monotonic() + 10
        data = b''
        while len(data) < size:
            if not select.select([self.process.stdout], [], [], max(0, deadline - time.monotonic()))[0]:
                raise TimeoutError('Worker response timeout')
            chunk = os.read(self.process.stdout.fileno(), size - len(data))
            if not chunk:
                raise EOFError('Worker closed its output')
            data += chunk
        return data

    def word(self):
        return struct.unpack('<I', self.read(4))[0]

    def score(self, context, reading, candidates):
        frame = struct.pack('<I', len(candidates))
        for text in [context, reading, *candidates]:
            encoded = text.encode()
            frame += struct.pack('<I', len(encoded)) + encoded
        self.process.stdin.write(frame)
        self.process.stdin.flush()
        count = self.word()
        if count == REFUSED:
            return None
        if count != len(candidates):
            raise AssertionError('Invalid response count')
        scores = list(struct.unpack('<' + 'd' * count, self.read(count * 8)))
        if not all(math.isfinite(value) for value in scores):
            raise AssertionError('Non-finite candidate score')
        return scores

    def close(self):
        try:
            self.process.stdin.close()
        except (BrokenPipeError, OSError):
            pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.process.stdout.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


class NativeWorkerTests(unittest.TestCase):
    def test_context_changes_first_candidate(self):
        candidates = ['記者', '汽車', '帰社', '貴社']
        with Worker() as worker:
            for context, expected in [('駅で', '汽車'), ('新聞の', '記者')]:
                scores = worker.score(context, 'キシャ', candidates)
                self.assertEqual(candidates[max(range(4), key=scores.__getitem__)], expected)

    def test_rejected_request_does_not_kill_worker(self):
        with Worker() as worker:
            pid = worker.process.pid
            self.assertIsNone(worker.score('駅で', 'キシャ', ['']))
            self.assertEqual(len(worker.score('駅で', 'キシャ', ['汽車'])), 1)
            self.assertEqual(worker.process.pid, pid)
            self.assertIsNone(worker.process.poll())

    def test_warmup_and_eight_candidates(self):
        with Worker() as worker:
            self.assertEqual(worker.score('駅で', '', []), [])
            scores = worker.score('駅で', 'キシャ', ['汽車'] * 8)
            self.assertEqual(len(scores), 8)
            self.assertLess(max(scores) - min(scores), 1e-5)
            with self.assertRaises(EOFError):
                worker.score('駅で', 'キシャ', ['汽車'] * 9)
            self.assertEqual(worker.process.wait(timeout=5), 1)

    def test_neither_cwd_nor_backend_path_loads_plugins(self):
        for location in ['cwd', 'environment']:
            with self.subTest(location=location), tempfile.TemporaryDirectory(prefix='tgskk-backend-test-') as scratch:
                root = Path(scratch)
                plugin_dir = root / 'plugins'
                plugin_dir.mkdir()
                plugin = plugin_dir / PROBE.name
                shutil.copyfile(PROBE, plugin)
                marker = root / 'loaded'
                env = os.environ.copy()
                env['TGSKK_PROBE_MARKER'] = str(marker)
                env.pop('GGML_BACKEND_PATH', None)
                cwd = root
                if location == 'cwd':
                    cwd = plugin_dir
                else:
                    env['GGML_BACKEND_PATH'] = str(plugin)
                with Worker(cwd=cwd, env=env) as worker:
                    self.assertEqual(len(worker.score('駅で', 'キシャ', ['汽車'])), 1)
                    self.assertFalse(marker.exists(), 'Worker executed an untrusted dynamic backend')


if __name__ == '__main__':
    unittest.main()
