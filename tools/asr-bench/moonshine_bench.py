"""Offline benchmark of the actual Moonshine medium streaming model, in batch mode."""
import argparse
import json
import time
from pathlib import Path

import numpy as np
from moonshine_voice import ModelArch, Transcriber

parser = argparse.ArgumentParser()
parser.add_argument("model")
parser.add_argument("manifest", type=Path)
parser.add_argument("--repeats", type=int, default=3)
parser.add_argument("--stream", action="store_true", help="Pace real audio at capture speed; measure end-of-file to final result")
args = parser.parse_args()
clips = json.loads(args.manifest.read_text())
audio = [np.fromfile(c["path"], dtype="<f4").tolist() for c in clips]
start = time.perf_counter()
transcriber = Transcriber(args.model, ModelArch.MEDIUM_STREAMING)
print(json.dumps({"event": "load", "model": "moonshine-v2-medium", "seconds": time.perf_counter() - start}), flush=True)
for repeat in range(args.repeats + 1):
    for clip, samples in list(zip(clips, audio))[:1 if repeat == 0 else len(clips)]:
        start = time.perf_counter()
        extra = {}
        if args.stream:
            stream = transcriber.create_stream(update_interval=0.5)
            stream.start()
            capture_start = time.perf_counter()
            busy = 0.0
            for offset in range(0, len(samples), 1600):
                chunk = samples[offset:offset + 1600]
                deadline = capture_start + (offset + len(chunk)) / 16000
                time.sleep(max(0, deadline - time.perf_counter()))
                work_start = time.perf_counter()
                stream.add_audio(chunk, 16000)
                busy += time.perf_counter() - work_start
            final_start = time.perf_counter()
            result = stream.stop()
            end = time.perf_counter()
            stream.close()
            if result is None:
                raise RuntimeError("stream finalization failed")
            elapsed = end - final_start
            extra = {"release_to_final_seconds": max(0, end - capture_start - len(samples) / 16000),
                     "stream_compute_seconds": busy + elapsed}
        else:
            result = transcriber.transcribe_without_streaming(samples, 16000)
            elapsed = time.perf_counter() - start
        text = " ".join(line.text for line in result.lines)
        print(json.dumps({"event": "transcribe", "model": "moonshine-v2-medium", "id": clip["id"],
                          "pass": repeat, "seconds": elapsed, "audio_seconds": clip["seconds"], "text": text,
                          "mode": "paced-stream" if args.stream else "batch", **extra}), flush=True)
