"""Download pinned real speech and export identical 16 kHz float PCM for all engines."""
import argparse
import io
import json
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq
import soundfile as sf
from huggingface_hub import hf_hub_download

parser = argparse.ArgumentParser()
parser.add_argument("directory", type=Path)
parser.add_argument("--count", type=int, default=24)
args = parser.parse_args()
args.directory.mkdir(parents=True, exist_ok=True)
revision = "5be91486e11a2d616f4ec5db8d3fd248585ac07a"
file = hf_hub_download("hf-internal-testing/librispeech_asr_dummy",
                       "clean/validation-00000-of-00001.parquet", repo_type="dataset", revision=revision)
rows = pq.read_table(file).to_pylist()
if not 1 <= args.count <= len(rows):
    parser.error(f"count must be between 1 and {len(rows)}")
clips = []
for index in np.linspace(0, len(rows) - 1, args.count, dtype=int):
    row = rows[index]
    samples, rate = sf.read(io.BytesIO(row["audio"]["bytes"]), dtype="float32")
    if rate != 16000 or samples.ndim != 1:
        raise ValueError("Expected mono 16 kHz corpus")
    path = (args.directory / (row["id"] + ".f32")).resolve()
    samples.astype("<f4").tofile(path)
    clips.append({"id": row["id"], "path": str(path), "text": row["text"],
                  "seconds": len(samples) / rate, "speaker_id": row["speaker_id"]})
(args.directory / "manifest.json").write_text(json.dumps(clips, indent=2) + "\n")
print(f"Exported {len(clips)} clips. This small clean-speech subset is not a dictation quality benchmark.")
