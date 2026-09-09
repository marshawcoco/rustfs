import unittest

from durable_capacity_smoke import latency_summary


class LatencySummaryTests(unittest.TestCase):
    def test_nearest_rank_percentiles_keep_sample_count(self):
        summary = latency_summary([value / 100 for value in reversed(range(1, 101))])
        self.assertEqual(summary, {"samples": 100, "p95_seconds": 0.95, "p99_seconds": 0.99})

    def test_small_baseline_does_not_interpolate_beyond_observed_samples(self):
        self.assertEqual(latency_summary([0.3]), {"samples": 1, "p95_seconds": 0.3, "p99_seconds": 0.3})

    def test_invalid_samples_are_not_passing_evidence(self):
        for samples in [[], [-1], [float("nan")], [float("inf")]]:
            with self.subTest(samples=samples), self.assertRaises(ValueError):
                latency_summary(samples)


if __name__ == "__main__":
    unittest.main()
