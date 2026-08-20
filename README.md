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

## It does not update itself

Deliberate, decided 2026-08-20. Euterpe's file-distribution API
(`GET /api/distribution/{bucket}`) requires a signed-in user, and there is no
public read path a version check could use: `/api/public/version` reports the
*server's* version and `/api/public/desktop-download` is bound to the desktop
app's Velopack installers. Making a charter sign in so that a local video tool
can look up its own version number is a worse trade than shipping without the
feature, so Press ships without it.

Velopack was considered and rejected for the same reason plus a second one:
`velopack_service.PACK_ID` is hard-coded to `"Euterpe"`, so a second product
would mean changing Euterpe.Web's table, its routes and the RID allowlist in its
nginx config — a great deal of machinery for one single-file tool.

If a public read path ever exists, the check belongs in one place: compare with
a real SemVer parser and offer an update only when the server's version is
**strictly greater**. `?latest` on that API means *most recently uploaded*, not
highest version (`ORDER BY created_at DESC`), so treating "different" as "newer"
would let one back-fill downgrade every install.

## One file, encoder included

A release carries its own ffmpeg, gzipped into the executable and unpacked on
first use to `%TEMP%\Euterpe.Press\ffmpeg-{stamp}.exe`. The name is the content's
own hash — the doctrine Euterpe.Mod already uses for its embedded natives — so a
new build cannot collide with an older instance's still-locked file, and a stale
build of the same length cannot stand in for this one.

Shipping the encoder rather than finding one is the point, not the convenience.
The recipe Press follows is the server's, and a machine's stray ffmpeg quietly
producing something slightly different is exactly the drift that copying the
recipe exists to prevent. A build with no embedded encoder falls back to `PATH`
— that is a compile-time branch, for development, and a release never reaches
it.

The bundled build is `--disable-everything` plus only what Press uses: five
decoders (including the VP9 and mpeg4 it exists to *replace*, which it must read
first), the SVT-AV1 encoder, raw frames for the viewfinder, MP4 and pipe, and
two filters. dav1d is deliberately absent: the only AV1 decoding Press does is a
640-pixel preview.

**Its size is measured in CI and reported in the run summary, not estimated
here.** Building it needs a C toolchain this repository does not assume.

## Build

```pwsh
cargo test                        # includes the agreement with the server
cargo run -- --check video.mp4    # the verdict, from a terminal
cargo run -- --fix video.mp4      # produce one the server accepts
cargo run                         # the window
pwsh tools/make_fixtures.ps1      # regenerate the fixtures (needs ffmpeg)
uv run --project ../Euterpe.Web python tools/golden.py   # re-record the answers
```

`vendor/ffmpeg.exe`, if present, is embedded. CI puts it there; a developer's
checkout has none and uses whatever is on `PATH`.
