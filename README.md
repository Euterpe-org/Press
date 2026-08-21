# Euterpe Press

Drag a video in. It tells you whether euterpe-org.com will take it, and if not,
hands you one that it will.

Exporting a background video gives you five ways to get it wrong — wrong codec,
too many pixels, too many frames per second, an audio track nobody will hear,
or just too big — and no way to find out except spending an 80 MB upload on the
answer. This gives you the answer locally, in about a second, and then does the
work if there is any.

## Get it

[Releases](../../releases). One `.exe`, and there is genuinely nothing to
install: no runtime, no Visual C++ redistributable, and the video encoder is
inside the file. Windows 10 or later, 64-bit.

It isn't code-signed, so SmartScreen will stop you the first time. More info →
Run anyway. Each release has a `.sha256` next to it if you want to check what
you downloaded is what CI built.

## What it does to your computer

Not much, deliberately:

- Your video is never touched. The result goes wherever you tell it to.
- Nothing is written to the folder you run it from.
- Two folders get used. `%TEMP%\Euterpe.Press` holds working files and is
  emptied when the run finishes. `%LOCALAPPDATA%\Euterpe.Press` holds the
  encoder once it's unpacked, so it doesn't have to unpack every time.
- No network — there is no networking code in it at all. No registry, no
  administrator, no service, no auto-update.

Delete the exe and those two folders and it's gone.

## Using it

Drop a video on the window. It reads a few hundred bytes of the MP4 header —
no decoding — and tells you what's wrong with it. Most files need a stream copy
to drop the audio track, which is exact and takes about a second.

You get two decisions of your own. The scrubber sets where the video starts and
ends. And if the picture is neither 16:9 nor 16:10 — which is what people's
screens actually are — it offers to crop to one, and tells you how much of the
picture that costs. Both are offers; it won't do either on its own.

There's also a terminal mode, which is how you check a folder full of them:

```
euterpe-press --check video.mp4     # what would the server say
euterpe-press --fix video.mp4       # write one it accepts, next to the original
```

## The rules aren't ours

Everything Press judges belongs to the server. It's the third implementation of
that gate, after the server's and the browser's, and three implementations are
three chances to disagree — being told "this is fine" by a tool and "refused"
by the server is worse than having no tool.

So the agreement isn't asserted by hand. `tools/golden.py` imports the server's
own gate, runs it over every fixture, and records the answers;
`tests/gate.rs` holds the Rust port to them field for field, and CI fails if the
server moves and Press doesn't. Same for the transcode plan: thirteen scenarios,
recorded from the server, checked on every build.

It paid for itself before it shipped. Porting the gate found a real ordering bug
in the server's own version — a 120 fps file with an audio track cost the
uploader two round trips instead of one.

## Why it doesn't use your GPU

Because on this particular job it's slower where it counts. Measured on a real
chart video:

| | time | result |
|---|---|---|
| stream copy (drop the audio) | 0.3 s | 60.6 MB |
| SVT-AV1 crf 32, 24-core CPU | 27.5 s | 65.4 MB — 108% of the copy |
| av1_nvenc, RTX 5080 | 7.9 s | 75.1 MB — 124% of the copy |

Both AV1 attempts came back *bigger* than a free, lossless remux, so both were
thrown away and the 0.3-second file shipped. On a file that genuinely has to be
re-encoded, NVENC is four times faster and 37% larger — and bytes are the point
here, since every replacement ships to everyone who plays the chart.

Press is fast because most of the time it doesn't encode at all.

## Building

```pwsh
cargo test                        # includes the agreement with the server
cargo run                         # the window
cargo run -- --check video.mp4
```

A checkout has no `vendor/ffmpeg.exe`, so a local build falls back to whatever
`ffmpeg` is on your PATH and says so plainly in the window. CI builds a minimal
one from source — `--disable-everything` plus the five decoders, one encoder and
two filters this actually uses — and embeds it. That's the difference between a
20 MB release and a 90 MB one.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).

The bundled ffmpeg is built without `--enable-gpl`, so it's LGPL-2.1+, and the
one external library in it is SVT-AV1 (BSD-3). The exact configure line that
produced it is in `.github/workflows/build.yml`.
