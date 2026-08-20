# Generates the tiny MP4s the probe is tested against.
#
# Every shape here is one the live library actually contains (see
# Euterpe.Web/.claude/rules/video.md) or one the gate has a named verdict for.
# They are kept small enough to live in the repo -- a few KB each -- because a
# parser test that needs a 60 MB download is a test nobody runs.
#
# Requires ffmpeg on PATH. Run from the repo root:
#   pwsh tools/make_fixtures.ps1

$ErrorActionPreference = "Stop"
$out = Join-Path $PSScriptRoot "..\tests\fixtures"
New-Item -ItemType Directory -Force $out | Out-Null

# A deterministic 1-second source. `testsrc2` has motion and detail, so an
# encoder cannot collapse it to nothing and leave a file with no samples.
$src = @("-f", "lavfi", "-i", "testsrc2=size=320x180:rate=30:duration=1")
$srcT = @("-f", "lavfi", "-i", "testsrc2=size=180x320:rate=30:duration=1")
# The pixel limit is a boundary, so it is tested as one rather than with a 4K
# clip that is obviously over: 2560x1440 is exactly the budget and must pass,
# 2560x1444 is 10,240 pixels past it and must not. A pair four rows apart also
# keeps both fixtures small enough to live in the repo, which a 4K frame is not.
# Flat colour, not `testsrc2`, for the two large frames: these fixtures exist to
# state a WIDTH and a HEIGHT, and detail at 2560 px costs half a megabyte of
# repository for a number the probe reads out of `stsd`. The small fixtures keep
# real content, where it is cheap and where sample timing actually matters.
$srcAt = @("-f", "lavfi", "-i", "color=c=0x334455:size=2560x1440:rate=30:duration=1")
$srcOver = @("-f", "lavfi", "-i", "color=c=0x334455:size=2560x1444:rate=30:duration=1")
$src120 = @("-f", "lavfi", "-i", "testsrc2=size=320x180:rate=120:duration=1")
$sine = @("-f", "lavfi", "-i", "sine=frequency=440:duration=1")

# `$args` is a PowerShell automatic variable and is silently overwritten inside
# a function, so the parameter carries a name of its own.
function Make($name, $ffargs) {
    $path = Join-Path $out $name
    & ffmpeg -y -v error @ffargs $path
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed for $name" }
    $kb = [math]::Round((Get-Item $path).Length / 1KB, 1)
    "{0,-24} {1,8} KB" -f $name, $kb
}

$h264 = @("-c:v", "libx264", "-preset", "veryfast", "-crf", "30", "-pix_fmt", "yuv420p")

# ── the shapes ───────────────────────────────────────────────────────────
# plain: the ordinary case -- 314 of the live library's 349 are this shape.
Make "plain_h264.mp4"      ($src + $h264 + @("-an"))

# audio: the single most common violation -- 85% of the live library carries one.
Make "audio_h264.mp4"      ($src + $sine + $h264 + @("-c:a", "aac", "-shortest"))

# fragmented: 35 of 349. Empty stts, duration 0, samples live in moof/trun.
Make "fragmented.mp4"      ($src + $h264 + @("-an", "-movflags", "frag_keyframe+empty_moov+default_base_moof"))

# portrait: judged on PIXEL COUNT, so 180x320 passes exactly as 320x180 does.
Make "portrait.mp4"        ($srcT + $h264 + @("-an"))

# over the frame-rate limit -- can only be fixed by re-encoding.
Make "fps120.mp4"          ($src120 + $h264 + @("-an"))

# the pixel boundary, from both sides.
$fast = @("-c:v", "libx264", "-preset", "ultrafast", "-crf", "40", "-pix_fmt", "yuv420p")
Make "pixels_at_limit.mp4" ($srcAt + $fast + @("-an"))
Make "pixels_over.mp4"     ($srcOver + $fast + @("-an"))

# codecs the gate accepts, and two it refuses.
Make "hevc.mp4"            ($src + @("-c:v", "libx265", "-preset", "veryfast", "-crf", "30", "-pix_fmt", "yuv420p", "-an", "-tag:v", "hvc1"))
Make "av1.mp4"             ($src + @("-c:v", "libsvtav1", "-crf", "40", "-preset", "10", "-pix_fmt", "yuv420p", "-an"))
Make "vp9.mp4"             ($src + @("-c:v", "libvpx-vp9", "-crf", "50", "-b:v", "0", "-pix_fmt", "yuv420p", "-an"))
Make "mpeg4.mp4"           ($src + @("-c:v", "mpeg4", "-q:v", "10", "-pix_fmt", "yuv420p", "-an"))

# ── structural problem PLUS an audio track ───────────────────────────────
# The gate reports the structural problem FIRST, because stripping the audio
# cannot fix any of these three -- they all need a fresh export. Getting that
# order wrong costs the charter a wasted round trip: they follow the audio
# advice, re-upload, and are refused again for the thing that actually mattered.
#
# It was wrong. `check()` had the audio test above the frame-rate test, so a
# 120 fps file with audio cost exactly that round trip; found 2026-08-20 while
# building this port against real files, fixed in Euterpe.Web `70a9fa8f`. These
# three fixtures exist so it cannot come back: one per structural class, each
# carrying the violation that would shadow it.
Make "audio_fps120.mp4"    ($src120 + $sine + $h264 + @("-c:a", "aac", "-shortest"))
Make "audio_pixels_over.mp4" ($srcOver + $sine + $fast + @("-c:a", "aac", "-shortest"))
Make "audio_vp9.mp4"       ($src + $sine + @("-c:v", "libvpx-vp9", "-crf", "50", "-b:v", "0", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"))

# QuickTime-derived: a `qt  ` brand and a second hdlr under minf naming the DATA
# handler (`alis`). Reading THAT hdlr made 13 good videos look like they had no
# video track. Named .mp4 on purpose -- that is how they arrive.
Make "quicktime_brand.mp4" ($src + $h264 + @("-an", "-f", "mov"))

# a leading `free` box ahead of ftyp: present, but not first.
Make "faststart.mp4"       ($src + $h264 + @("-an", "-movflags", "+faststart"))

# ── things that are not videos ───────────────────────────────────────────
# Not an MP4 at all -> INVALID_VIDEO, the same verdict as every other
# unreadable shape. The gate deliberately does not distinguish them.
[System.IO.File]::WriteAllBytes((Join-Path $out "not_an_mp4.bin"),
    [byte[]](0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09))
"{0,-24} {1,8} KB" -f "not_an_mp4.bin", 0

# Truncated: a real header cut off mid-mdat. Walks fine, then runs out.
$plain = [System.IO.File]::ReadAllBytes((Join-Path $out "plain_h264.mp4"))
[System.IO.File]::WriteAllBytes((Join-Path $out "truncated.mp4"), $plain[0..([math]::Min(2047, $plain.Length - 1))])
"{0,-24} {1,8} KB" -f "truncated.mp4", 2

# ── the boundaries no muxer will produce ─────────────────────────────────
# A header that contradicts its own sample table has to be assembled by hand;
# see tools/synth.py. Stdlib only, so it needs no project.
& uv run --no-project python (Join-Path $PSScriptRoot "synth.py")
if ($LASTEXITCODE -ne 0) { throw "tools/synth.py failed" }

Write-Output ""
Write-Output "fixtures in $out"
Write-Output "now run:  uv run --project <Euterpe.Web> python tools/golden.py"
