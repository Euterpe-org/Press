# Euterpe Press

Drag a video in. It says whether euterpe-org.com will take it, and if not it
hands you one that it will.

A charter exporting a background video has five ways to get it wrong and no way
to find out except by spending an 80 MB upload on the answer. This is that
answer, locally, in about a second.

## The rule it enforces is not its own

Everything Press judges belongs to the server. The authority is
`Euterpe.Web/backend/app/utils/chart_constants.py` and
`backend/app/domain/video_probe.py`; Press is the **third** implementation of
that gate, after the server's and the browser's.

Three implementations are three chances to disagree, and a charter told "this
file is fine" by a tool and "refused" by the server has been actively misled —
worse off than with no tool at all. So the agreement is not asserted by hand:

- `tools/golden.py` imports the **server's own** gate, runs it over every
  fixture, and records what it answered into `tests/fixtures/golden.json`.
- `tests/gate.rs` asserts the Rust port produces the same facts and the same
  verdict for every one of them, field for field.
- `tools/golden.py --check` re-runs it in CI and goes red if the server has
  moved and Press has not.

Building it this way paid for itself before it shipped: the port found a real
ordering bug in the server's gate on 2026-08-20 (a 120 fps file with an audio
track cost the charter two round trips), fixed in Euterpe.Web `70a9fa8f`.

## What it does, in one paragraph

`src/gate/` parses the MP4 box tree — no ffmpeg, no decoding, a few hundred
bytes read out of the header — and produces five facts: codec, width, height,
frame rate, whether there is an audio track. Those go through `check()` in the
server's order and come out as one `Verdict`. If the verdict is `Ok` there is
nothing to do. If it is not, the planner decides the cheapest treatment that
makes it `Ok`: stripping an audio track is a **stream copy**, exact and free,
and that is the whole fix for the great majority of files; only a wrong
resolution, a wrong frame rate or an undecodable codec forces a re-encode.
AV1 is then encoded as well, and **kept only if it comes back smaller** than
the exact copy. Every output is re-read through the same `gate` that judged the
input, and nothing is written that the server would refuse.

## Why it does not use your GPU

Measured on a real chart video, 2026-08-20:

| | time | result |
|---|---|---|
| stream copy (drop the audio) | **0.3 s** | 60.6 MB |
| SVT-AV1 crf 32 preset 6 (24-core CPU) | 27.5 s | 65.4 MB — **108%** of the copy |
| av1_nvenc (RTX 5080) | 7.9 s | 75.1 MB — **124%** of the copy |

Both AV1 attempts came back *bigger* than a free, lossless remux, so both were
discarded and the 0.3-second file shipped. On a file that genuinely must be
re-encoded (1080p at 120 fps), NVENC is four times faster and **37% larger**
than SVT-AV1 — and on this product bytes *are* the quality metric: video is 43%
of chart storage and 40% of egress, and every replacement ships to every holder.
The "AV1 must beat an exact copy" rule therefore discards a hardware encode more
often than a software one, which makes the GPU time worse than wasted. Hardware
*decoding* was measured too, at zero benefit: the encoder is the bottleneck.

Press is fast because it usually does not encode at all.

## Build

```pwsh
cargo test                        # includes the agreement with the server
cargo run -- video.mp4            # the verdict, from a terminal
pwsh tools/make_fixtures.ps1      # regenerate the fixtures (needs ffmpeg)
uv run --project ../Euterpe.Web python tools/golden.py   # re-record the answers
```
