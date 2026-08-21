# Euterpe Press

Drag a video in. It says whether euterpe-org.com will take it, and if not it
hands you one that it will.

A charter exporting a background video has five ways to get it wrong and no way
to find out except by spending an 80 MB upload on the answer. This is that
answer, locally, before the upload — the verdict in about a second, and the
file that fixes it in as long as an encode takes.

## Get it

[**Releases**](../../releases) — one `.exe`, nothing to install, no runtime to
put on the machine first. The encoder is inside it. A `.sha256` sits beside each
download because the executable is unsigned and you should be able to check you
got what CI built.

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

## The verdict, and then the work

`src/gate/` parses the MP4 box tree — no ffmpeg, no decoding, a few hundred
bytes out of the header — and produces five facts: codec, width, height, frame
rate, whether there is an audio track. Those go through `check()` in the
server's order and come out as one `Verdict`. That is the second.

Fixing it is two phases, split on a question that has nothing to do with
tidiness: **is the answer knowable in advance?**

**Phase A is everything that is decided.** Resolution, frame rate, codec, audio,
length — each follows from the file and the limits, with no judgement and no
search. It *executes* only what a stream copy can do, which is dropping the
audio track and cutting the tail: exact, lossless, and seconds even on a 379 MB
source. That is the whole fix for most files.

**Phase B is the one thing that is not.** How much quality a video needs to fit
in 80 MB depends on how compressible that particular footage turns out to be,
and nothing short of encoding it will say. So it gets its own phase, its own
screen, and it may run more than once. Measured on our own recipe, every +6 CRF
multiplies the output by a steady 0.709 — so one measurement predicts the rest
of the curve, and the search usually needs a quarter of one encode plus one real
one. Everything that rewrites the picture happens here, in a single pass, because
Press ships one encoder and a second AV1 generation costs ~190% for nothing.

What the encode is *for* decides what it may cost, and there are four answers:
rescue a file that is over the limit (the allowance is a budget, spend it),
repair one that is broken but small (house quality, no budget), apply something
the charter asked for (drop the request rather than gain weight), or beat the
file on size (ship only if smaller). **Under all four the result is never
heavier than what went in.**

The charter decides two things and Press decides nothing on their behalf: where
the video starts and ends, and — when the picture is neither 16:9 nor 16:10,
which is what players are actually on — whether to crop to one. Both are offers.
The gate has no opinion about shape, and moving somebody's opening against their
music is damage nobody asked for.

Every output is re-read through the same `gate` that judged the input, and
nothing is written that the server would refuse.

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

## Releasing

Tag it. `v1.2.3` builds, tests against the shipped encoder, proves the binary
still agrees with the gate, and publishes the exe and its checksum to Releases.

The tag has to equal `Cargo.toml`'s version and CI refuses the release if it
does not — before building, so a mismatch costs seconds rather than a wrong
download. The binary reports that version in its header and quotes it in the
crash dialog, so a release that misnames itself misnames itself to the one
person who most needs it to be right.
