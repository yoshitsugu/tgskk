import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('performance', Path(__file__).parents[1] / 'check-performance.py')
PERF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERF)
METRIC = 'input_ai_many_candidates_keys_p50_us'
TAIL = 'input_ai_many_candidates_keys_p99_us'


class PerformanceComparison(unittest.TestCase):
    def setUp(self):
        self.baseline = [{name: 100.0 for name in PERF.INPUT_METRICS | PERF.SCORING_METRICS}
                         for _ in range(7)]
        self.candidate = copy.deepcopy(self.baseline)

    def compare(self):
        return PERF.compare(self.baseline, self.candidate)

    def test_identical_results_pass_and_report_the_pair_ratio(self):
        results = self.compare()
        self.assertTrue(all(not result['regressed'] for result in results.values()))
        self.assertTrue(all(result['median_ratio'] == 1 for result in results.values()))

    def test_detects_repeated_slowdown_in_the_input_path_and_scorer(self):
        for name in (METRIC, 'worker_eight_p50_us'):
            for row in self.candidate[:5]:
                row[name] = 126.0
            self.assertTrue(self.compare()[name]['regressed'])

    def test_transient_outliers_and_shared_runner_slowdowns_do_not_fail(self):
        for index, row in enumerate(self.baseline):
            row[METRIC] *= index + 1
            self.candidate[index][METRIC] = row[METRIC]
        for row in self.candidate[:4]:
            row[METRIC] *= 10
        self.assertFalse(self.compare()[METRIC]['regressed'])

    def test_tail_threshold_and_tiny_absolute_changes_are_distinct(self):
        for old, new in zip(self.baseline, self.candidate):
            old[METRIC], new[METRIC] = 0.5, 0.9
            new[TAIL] = 149.0
        self.assertFalse(self.compare()[METRIC]['regressed'])
        self.assertFalse(self.compare()[TAIL]['regressed'])
        for new in self.candidate:
            new[TAIL] = 151.0
        self.assertTrue(self.compare()[TAIL]['regressed'])

    def test_rejects_missing_rounds_metrics_nonfinite_and_zero_samples(self):
        with self.assertRaises(ValueError):
            PERF.compare(self.baseline[:6], self.candidate[:6])
        for invalid in (0, -1, float('nan'), float('inf'), True, '100'):
            changed = copy.deepcopy(self.candidate)
            changed[0][METRIC] = invalid
            with self.assertRaises(ValueError):
                PERF.compare(self.baseline, changed)
        changed = copy.deepcopy(self.candidate)
        del changed[0][METRIC]
        with self.assertRaises(ValueError):
            PERF.compare(self.baseline, changed)
        with self.assertRaises(ValueError):
            PERF.compare(self.baseline, self.candidate[:-1])


if __name__ == '__main__':
    unittest.main()
