"""Summarize matched clips, warm processing latency, and lexical word error rate."""
import argparse
import json
import re
from pathlib import Path

import numpy as np
from whisper_normalizer.english import EnglishTextNormalizer

normalize_english = EnglishTextNormalizer()


def words(text):
    # Case/punctuation normalization only; no number/abbreviation expansion.
    return re.findall(r"[a-z0-9]+(?:'[a-z0-9]+)*", text.lower().replace("’", "'"))


def distance(reference, hypothesis):
    previous = list(range(len(hypothesis) + 1))
    for i, word in enumerate(reference, 1):
        current = [i]
        for j, other in enumerate(hypothesis, 1):
            current.append(min(current[-1] + 1, previous[j] + 1, previous[j - 1] + (word != other)))
        previous = current
    return previous[-1]


def summarize(manifest, paths):
    references = {c["id"]: words(c["text"]) for c in manifest}
    output = {}
    for path in paths:
        rows = [json.loads(line) for line in path.read_text().splitlines() if line.startswith("{")]
        warm = [r for r in rows if r.get("event") == "transcribe" and r["pass"] > 0]
        ids = {r["id"] for r in warm}
        if ids != set(references):
            raise ValueError(f"{path}: incomplete or mismatched corpus")
        by_pass = {}
        for row in warm:
            by_pass.setdefault(row["pass"], []).append(row)
        for records in by_pass.values():
            if len(records) != len(references) or {r['id'] for r in records} != set(references):
                raise ValueError(f"{path}: incomplete or duplicate pass")
        latencies = [r.get("release_to_final_seconds", r["seconds"]) for r in warm]
        errors = sum(distance(references[r["id"]], words(r["text"])) for r in warm)
        total_words = sum(len(references[r["id"]]) for r in warm)
        normalized_refs = {c["id"]: normalize_english(c["text"]).split() for c in manifest}
        normalized_errors = sum(distance(normalized_refs[r["id"]], normalize_english(r["text"]).split()) for r in warm)
        normalized_total = sum(len(normalized_refs[r["id"]]) for r in warm)
        first = [r for r in rows if r.get("event") == "transcribe" and r["pass"] == 0]
        output[path.stem] = {
            "clips": len(ids), "warm_runs": len(warm), "passes": len(by_pass),
            "word_errors": errors, "reference_words": total_words, "wer_percent": errors / total_words * 100,
            "normalized_word_errors": normalized_errors, "normalized_reference_words": normalized_total,
            "normalized_wer_percent": normalized_errors / normalized_total * 100,
            "p50_ms": float(np.percentile(latencies, 50) * 1000),
            "p95_ms": float(np.percentile(latencies, 95) * 1000),
            "cold_first_call_ms": first[0]["seconds"] * 1000 if first else None,
            "load_seconds": next((r["seconds"] for r in rows if r.get("event") == "load"), None),
            "latency_kind": "simulated release to final" if "release_to_final_seconds" in warm[0] else "batch engine call",
            "transcript_variants": sum(len({r["text"] for r in warm if r["id"] == clip}) > 1 for clip in ids),
            "errors": [{"id": r["id"], "reference": next(c['text'] for c in manifest if c['id'] == r['id']),
                        "hypothesis": r['text'], "distance": distance(references[r['id']], words(r['text']))}
                       for r in by_pass[min(by_pass)] if distance(references[r['id']], words(r['text']))],
        }
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("results", nargs="+", type=Path)
    args = parser.parse_args()
    print(json.dumps(summarize(json.loads(args.manifest.read_text()), args.results), indent=2))
