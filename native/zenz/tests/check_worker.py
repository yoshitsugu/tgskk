#!/usr/bin/env python3
"""Exercise the real static worker, framing, and dynamic-backend isolation."""
import math
import os
from pathlib import Path
import queue
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import time
import unittest

WORKER, MODEL = (Path(arg).resolve() for arg in sys.argv[1:3])
PROBE = Path(sys.argv[3]).resolve() if len(sys.argv) > 3 else None
sys.argv = sys.argv[:1]
MAGIC = 0x325A4754
REFUSED = 0xFFFFFFFF


class Worker:
    def __init__(self, cwd=None, env=None, model=MODEL):
        self.process = subprocess.Popen([str(WORKER), str(model)], cwd=cwd, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.reader = None
        if os.name == 'nt':
            self.incoming = queue.Queue()
            self.buffer = b''
            self.reader = threading.Thread(target=self.read_pipe, daemon=True)
            self.reader.start()
        try:
            if self.word() != MAGIC:
                raise AssertionError('Worker protocol version mismatch')
        except BaseException:
            self.close()
            raise

    def read(self, size):
        if self.reader is not None:
            deadline = time.monotonic() + 10
            while len(self.buffer) < size:
                try:
                    chunk = self.incoming.get(timeout=max(0, deadline - time.monotonic()))
                except queue.Empty:
                    raise TimeoutError('Worker response timeout') from None
                if isinstance(chunk, Exception):
                    raise chunk
                if not chunk:
                    raise EOFError('Worker closed its output')
                self.buffer += chunk
            result, self.buffer = self.buffer[:size], self.buffer[size:]
            return result
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

    def read_pipe(self):
        try:
            while True:
                chunk = os.read(self.process.stdout.fileno(), 1024)
                self.incoming.put(chunk)
                if not chunk:
                    return
        except OSError as error:
            self.incoming.put(error)

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
        if self.reader is not None:
            self.reader.join(timeout=5)
            if self.reader.is_alive():
                raise TimeoutError('Worker stdout reader did not stop')
        self.process.stdout.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


class NativeWorkerTests(unittest.TestCase):
    def test_unicode_model_path(self):
        with tempfile.TemporaryDirectory(prefix='tgskk-モデル-') as scratch:
            model = Path(scratch) / '学習モデル.gguf'
            shutil.copyfile(MODEL, model)
            with Worker(model=model) as worker:
                scores = worker.score('駅で', 'キシャ', ['記者', '汽車'])
                self.assertGreater(scores[1], scores[0])

    def test_scores_match_pinned_serial_reference(self):
        # Rare kanji make a 40-character context consume many model tokens.
        rare = ('𠮷𩸽𠀋𡈽𡌛𡑮𡢽𠮟𡚴𡸴𣇄𣗄𣜿𣝣𣳾𤟱𥒎𥔎𥝱𥧄'
                '𥶡𦫿𦹀𧃴𧚄𨉷𨏍𪆐𠂉𠂢𠂤𠆢𠈓𠌫𠎁𠍱𠏹𠑊𠔉')
        rare = (rare * 2)[:40]
        cases = [
            ('駅で', 'キシャ', ['記者', '汽車'], [-2.5865285336484294, -0.2760772154230728]),
            ('新聞の', 'キシャ', ['記者', '汽車'], [-0.0005491340728444249, -8.84067115625013]),
            (rare, 'キシャ' + 'ア' * 61, [rare[:23], '記者'], [-397.45471864680763, -3.1781633930180933]),
            ('あ' * 240, 'キシャ', ['漢' * 96, '汽車'], [-473.0221857021271, -3.096456488005224]),
        ]
        for context, reading, candidates, expected in cases:
            with self.subTest(context=context, reading=reading), Worker() as worker:
                actual = worker.score(context, reading, candidates)
                self.assertIsNotNone(actual)
                for got, reference in zip(actual, expected, strict=True):
                    # Serial references use the pinned model; allow libm/compiler rounding.
                    self.assertAlmostEqual(got, reference, delta=1e-6)

    def test_prompt_token_boundary_and_rejection_recovery(self):
        # With the pinned tokenizer these prompts have 252 and 253 tokens.
        context = 'あ' * 240
        candidates = ['漢' * 96, '汽車']
        with Worker() as worker:
            expected = worker.score(context, 'キシャ', candidates)
            self.assertIsNotNone(expected)
            single = [worker.score(context, 'キシャ', [candidate])[0]
                      for candidate in candidates]
            self.assertEqual(expected, single)
            self.assertIsNone(worker.score(context + 'あ', 'キシャ', candidates))
            self.assertEqual(worker.score(context, 'キシャ', candidates), expected)

    def test_context_changes_first_candidate(self):
        candidates = ['記者', '汽車', '帰社', '貴社']
        with Worker() as worker:
            for context, expected in [('駅で', '汽車'), ('新聞の', '記者')]:
                scores = worker.score(context, 'キシャ', candidates)
                self.assertEqual(candidates[max(range(4), key=scores.__getitem__)], expected)

    def test_rejected_request_does_not_kill_worker(self):
        with Worker() as worker:
            pid = worker.process.pid
            expected = worker.score('駅で', 'キシャ', ['汽車', '記者'])
            for rejected in [[''], ['汽車', '記者', '漢' * 97]]:
                with self.subTest(rejected_length=len(rejected[-1])):
                    self.assertIsNone(worker.score('別の文脈', 'キシャ', rejected))
                    self.assertEqual(worker.score('駅で', 'キシャ', ['汽車', '記者']), expected)
                    self.assertEqual(worker.process.pid, pid)
                    self.assertIsNone(worker.process.poll())

    def test_batched_scores_equal_single_candidate_requests(self):
        cases = [
            ('キシャ', ['汽車'] * 8),
            ('キシャ', ['記者', '汽車', '帰社', '貴社', '騎射', '喜捨', '稀釈', '汽車']),
            ('コウエン', ['公園', '講演', '公演', '後援', '口演', '高遠', '広縁', '紅炎']),
            ('カンジ', ['漢字', '感じ', '幹事', '監事', '完治', '官事', '莞爾', '漢字']),
            ('カガク', ['科学', '化学', '価額', '火学', '下学', '花萼', '科学', '化学']),
            ('タイショウ', ['対象', '対照', '大賞', '大将', '対称', '大正', '隊商', '退将']),
            ('キシャ', ['気', '木', '汽車', '記者', '国際連合安全保障理事会', '機', '帰社', '貴社']),
            ('コクサイ', ['国際連合安全保障理事会', '情報処理推進機構', '地方自治体',
                       '東京都千代田区', '全国高等学校野球選手権大会', '国際宇宙ステーション',
                       '公共交通機関', '学術研究機関']),
            ('シンブン', ['社', '本', '紙', '新聞', '記者', '記事', '取材', '出版社']),
            ('キシャ', ['汽', '記', '帰', '貴', '社', '車', '者', '機']),
        ]
        contexts = ['', '駅で', '新聞を読んでいるときの文章です',
                    ('ここでは直前までの文章を文脈として候補の採点に使います。' * 2)[-40:]]
        with Worker() as worker:
            for context in contexts:
                for reading, candidates in cases:
                    with self.subTest(context=context, reading=reading, candidates=candidates):
                        together = worker.score(context, reading, candidates)
                        single = [worker.score(context, reading, [candidate])[0]
                                  for candidate in candidates]
                        self.assertEqual(together, single)
                        self.assertEqual(worker.score(context, reading, candidates), single)
                        self.assertEqual(worker.score(context, reading, candidates[::-1]), single[::-1])

    def test_warmup_and_eight_candidates(self):
        with Worker() as worker:
            self.assertEqual(worker.score('駅で', '', []), [])
            scores = worker.score('駅で', 'キシャ', ['汽車'] * 8)
            self.assertEqual(len(scores), 8)
            self.assertLess(max(scores) - min(scores), 1e-5)
            with self.assertRaises(EOFError):
                worker.score('駅で', 'キシャ', ['汽車'] * 9)
            self.assertEqual(worker.process.wait(timeout=5), 1)

    def test_long_candidates_leave_the_shared_prefix_usable(self):
        with Worker() as worker:
            short = ['記者', '汽車']
            expected = worker.score('駅で', 'キシャ', short)
            candidates = ['漢' * 96, '𠮷' * 24]
            together = worker.score('駅で', 'キシャ', candidates)
            single = [worker.score('駅で', 'キシャ', [candidate])[0]
                      for candidate in candidates]
            self.assertEqual(together, single)
            self.assertEqual(worker.score('駅で', 'キシャ', short), expected)

    @unittest.skipUnless(PROBE, 'Linux dynamic-backend probe is not provided')
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
