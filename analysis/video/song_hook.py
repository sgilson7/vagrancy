#!/usr/bin/env python3
"""Find the punchiest place to start a song under a short clip (Sam,
2026-10-07: "find the most punchy part and most viral part to use, lined up
with the first cut of the footage").

The track is decoded (ffmpeg, from imageio-ffmpeg) and measured, not listened
to: its loudness and its onsets (rises in the spectrum) frame by frame, the
tempo from how the onsets repeat, the beat and bar grid from where they fall.
Each bar start is scored by how loud and how busy the next twelve seconds
are, and by how much louder its first two seconds are than the two before
(a drop, or a chorus coming in). The best bar start is where the clip's
music begins, at the first cut.

    .venv-test/bin/python analysis/video/song_hook.py FILE [FILE ...]
Prints, per file: its length, its tempo, and the chosen start with the two
runners-up, in seconds.
"""
import json, subprocess, sys

import imageio_ffmpeg
import numpy as np

RATE, HOP, WIN = 22050, 512, 2048


def decode(path):
    raw = subprocess.run([imageio_ffmpeg.get_ffmpeg_exe(), "-loglevel", "error", "-i", str(path), "-ac", "1", "-ar", str(RATE), "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(raw, dtype=np.float32)


def analyse(path):
    x = decode(path)
    n = 1 + (len(x) - WIN) // HOP
    frames = np.lib.stride_tricks.as_strided(x, shape=(n, WIN), strides=(x.strides[0] * HOP, x.strides[0]))
    rms = np.sqrt((frames ** 2).mean(axis=1))
    spec = np.abs(np.fft.rfft(frames * np.hanning(WIN), axis=1))
    flux = np.maximum(0, np.diff(np.log1p(spec), axis=0)).sum(axis=1)
    flux = np.concatenate([[0], flux])
    flux = (flux - flux.mean()) / (flux.std() + 1e-9)
    fps = RATE / HOP
    # The tempo: the lag, between 60 and 180 beats a minute, at which the
    # onsets best match themselves.
    lags = np.arange(int(fps * 60 / 180), int(fps * 60 / 60))
    ac = np.array([np.dot(flux[:-l], flux[l:]) for l in lags])
    period = lags[int(np.argmax(ac))]
    # The beat grid's phase, then the bar's: where the onsets, and then the
    # loudness, fall most strongly every beat and every fourth beat.
    phase = max(range(period), key=lambda p: flux[p::period].sum())
    beats = np.arange(phase, n, period)
    bar_phase = max(range(4), key=lambda k: rms[beats[k::4]].sum())
    bars = beats[bar_phase::4]
    loud = rms / (rms.max() + 1e-9)
    busy = np.clip(flux, 0, None)
    span, edge = int(12 * fps), int(2 * fps)
    scored = []
    for b in bars:
        if b < 4 * fps or b + span >= n:
            continue
        ahead = loud[b:b + span].mean()
        jump = loud[b:b + edge].mean() - loud[b - edge:b].mean()
        density = busy[b:b + int(6 * fps)].mean()
        scored.append((ahead + 1.5 * jump + 0.15 * density, b / fps))
    scored.sort(reverse=True)
    return {"file": str(path), "length": round(len(x) / RATE, 1), "bpm": round(60 * fps / period, 1),
            "start": round(scored[0][1], 2), "runners_up": [round(s[1], 2) for s in scored[1:3]]}


if __name__ == "__main__":
    for f in sys.argv[1:]:
        print(json.dumps(analyse(f)))
