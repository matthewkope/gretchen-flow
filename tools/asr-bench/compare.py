"""Paired clip bootstrap: repeated timings are not independent accuracy examples."""
import argparse
import json
from pathlib import Path

import numpy as np
from summarize import distance, normalize_english

parser = argparse.ArgumentParser()
parser.add_argument("manifest", type=Path)
parser.add_argument("baseline", type=Path)
parser.add_argument("candidates", nargs="+", type=Path)
args = parser.parse_args()
clips = json.loads(args.manifest.read_text())
ids = [c["id"] for c in clips]
refs = {c["id"]: normalize_english(c["text"]).split() for c in clips}


def load(path):
    rows = [json.loads(l) for l in path.read_text().splitlines() if l.startswith("{")]
    # One transcript per unique clip avoids pretending repeats add accuracy evidence.
    result = {r["id"]: r for r in rows if r.get("pass") == 1}
    if set(result) != set(ids):
        raise ValueError("Incomplete pass")
    return result


base = load(args.baseline)
base_errors = np.array([distance(refs[i], normalize_english(base[i]["text"]).split()) for i in ids])
lengths = np.array([len(refs[i]) for i in ids])
draws = np.random.default_rng(20260918).integers(0, len(ids), size=(10000, len(ids)))
output = {}
for path in args.candidates:
    candidate = load(path)
    errors = np.array([distance(refs[i], normalize_english(candidate[i]["text"]).split()) for i in ids])
    delta = errors - base_errors
    bootstrap = delta[draws].sum(axis=1) / lengths[draws].sum(axis=1) * 100
    output[path.stem] = {"wer_delta_percentage_points": float(delta.sum() / lengths.sum() * 100),
                         "paired_clip_bootstrap_95_interval": np.percentile(bootstrap, [2.5, 97.5]).tolist(),
                         "better_clips": int((delta < 0).sum()), "worse_clips": int((delta > 0).sum()),
                         "equal_clips": int((delta == 0).sum()), "unique_clips": len(ids)}
print(json.dumps(output, indent=2))
