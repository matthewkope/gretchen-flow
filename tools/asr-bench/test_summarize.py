import json
import tempfile
import unittest
from pathlib import Path

from summarize import distance, normalize_english, summarize, words


class ScoringTests(unittest.TestCase):
    def test_lexical_normalization(self):
        self.assertEqual(words("Hello, I'M here!"), ["hello", "i'm", "here"])

    def test_insert_delete_substitute(self):
        self.assertEqual(distance(["a", "b"], ["a", "x", "b"]), 1)
        self.assertEqual(distance(["a", "b"], ["a"]), 1)
        self.assertEqual(distance(["a", "b"], ["a", "x"]), 1)

    def test_equivalent_spoken_and_written_forms(self):
        self.assertEqual(normalize_english("Mister Quilter"), normalize_english("Mr. Quilter"))
        self.assertEqual(normalize_english("ten seconds"), normalize_english("10 seconds"))
        self.assertEqual(normalize_english("there is nothing"), normalize_english("there's nothing"))

    def test_warm_repeats_and_stream_release_metric(self):
        manifest = [{"id": "one", "text": "a b"}]
        rows = [{"event": "transcribe", "id": "one", "pass": i,
                 "text": "a x", "seconds": 0.01, "release_to_final_seconds": 0.03}
                for i in range(3)]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "result.jsonl"
            path.write_text("\n".join(json.dumps(r) for r in rows))
            result = summarize(manifest, [path])["result"]
            self.assertEqual(result["warm_runs"], 2)
            self.assertEqual(result["wer_percent"], 50)
            self.assertEqual(result["p50_ms"], 30)
            self.assertEqual(result["word_errors"], 2)

    def test_missing_clips_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "result.jsonl"
            path.write_text("")
            with self.assertRaises(ValueError):
                summarize([{"id": "one", "text": "a b"}], [path])


if __name__ == "__main__":
    unittest.main()
