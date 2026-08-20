"""Record what the server's transcode recipe does, once, as a snapshot.

`tools/golden.py` runs against `backend/app/domain/`, which is the gate itself
and is unambiguously an interface: three implementations of it exist and the
server is the authority. The recipe is a different animal. It lives in
`Euterpe.Web/scripts/video_transcode.py`, an operator's script run by hand a few
times a year, and whether its owner wants it pinned as a downstream contract is
their call, not mine.

So this is deliberately NOT wired into CI. It writes a dated SNAPSHOT of what
the recipe did on one day, `tests/fixtures/recipe.json`, and `tests/plan.rs`
holds the port to it. Re-run it by hand when there is reason to think the recipe
moved; the alternative -- a build that goes red because somebody edited an ops
script -- is a cost nobody agreed to pay.

    uv run --project ../Euterpe.Web python tools/recipe_snapshot.py
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
OUT = REPO / "tests" / "fixtures" / "recipe.json"
DEFAULT_WEB = REPO.parent / "Euterpe.Web"

web = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_WEB
if not (web / "scripts" / "video_transcode.py").exists():
    sys.exit(f"no Euterpe.Web scripts/ at {web}")

sys.path.insert(0, str(web / "backend"))
sys.path.insert(0, str(web / "scripts"))

# `ffmpeg()` builds an argument list and then runs it. The list is the thing
# worth recording, so the run is intercepted -- nothing is encoded here.
captured: list[list[str]] = []
real_run = subprocess.run
subprocess.run = lambda cmd, **kw: captured.append([str(a) for a in cmd])  # type: ignore[assignment]

import video_transcode as vt  # noqa: E402
from app.domain import video_probe  # noqa: E402
from app.domain.video_probe import VideoFacts  # noqa: E402


def facts(fourcc: str, width: int, height: int, fps: float, duration: float,
          has_audio: bool) -> VideoFacts:
    """Build facts the way a real file produces them: from a FOURCC.

    The codec is derived through the probe's own table rather than named, so a
    scenario cannot stand on an input no file yields. A VP9 track has no entry
    there and arrives as `(None, "vp09")`; writing `codec="vp9"` by hand pins the
    planner against fiction while reporting coverage it does not have. Both this
    snapshot and Euterpe.Web's own tests had exactly that bug, and both were
    fixed by removing the entry point rather than the wrong line.
    """
    return VideoFacts(
        codec=video_probe._CODEC_BY_FOURCC.get(fourcc.encode()),
        raw_fourcc=fourcc,
        width=width, height=height, fps=fps, duration=duration, has_audio=has_audio,
    )

# Each scenario is a decision the planner has to get right, not a sample of
# real files -- those are covered by tests/fixtures/*.mp4 and the gate.
SCENARIOS: list[tuple[str, VideoFacts, float | None]] = [
    # Conforms already: nothing forced, AV1 offered and must earn its place.
    ("conforming h264", facts("avc1", 1280, 720, 30.0, 238.6, True), None),
    # Already AV1: never re-encoded, a second generation of loss measured ~190%.
    ("conforming av1", facts("av01", 1280, 720, 30.0, 100.0, False), None),
    # H.265 is accepted too, and its other fourcc spelling must resolve.
    ("conforming h265", facts("hvc1", 1280, 720, 30.0, 100.0, False), None),
    # Scaling: 4K lands exactly on the budget; the +1 case proves the rounding
    # goes DOWN to even rather than up and over.
    ("4K", facts("avc1", 3840, 2160, 30.0, 60.0, False), None),
    ("odd 4K+1", facts("avc1", 3841, 2161, 30.0, 60.0, False), None),
    # Portrait is scaled by area like anything else -- no special case.
    ("portrait over", facts("avc1", 1440, 2564, 30.0, 60.0, False), None),
    ("120fps", facts("avc1", 1920, 1080, 120.032, 17.0, True), None),
    # The half-frame grace: 60.4 is inside it, 60.6 is not.
    ("60.4fps (grace)", facts("avc1", 1920, 1080, 60.4, 17.0, False), None),
    ("60.6fps (over)", facts("avc1", 1920, 1080, 60.6, 17.0, False), None),
    # Codecs the gate refuses: re-encoding is the only way out, at any size.
    ("vp9", facts("vp09", 1280, 720, 30.0, 60.0, False), None),
    ("mp4v", facts("mp4v", 1280, 720, 30.0, 60.0, False), None),
    # The tail. A trim alone leaves re-encoding merely offered, never forced.
    ("trim needed", facts("avc1", 1280, 720, 30.0, 220.7, False), 198.4),
    ("trim not needed", facts("avc1", 1280, 720, 30.0, 190.0, False), 198.4),
    # Everything at once, which is where an argument-order mistake shows.
    ("4K + trim", facts("avc1", 3840, 2160, 90.0, 300.0, True), 120.5),
]

rows = []
for name, facts, allowance in SCENARIOS:
    target = vt.plan(facts, allowance)
    commands = {}
    for label, copy, crf in [("copy", True, None), ("av1", False, vt.CRF), ("av1_44", False, 44)]:
        captured.clear()
        kwargs = {"copy": copy, "target": target}
        if crf is not None:
            kwargs["crf"] = crf
        vt.ffmpeg(pathlib.Path("IN.mp4"), pathlib.Path("OUT.mp4"), **kwargs)
        commands[label] = captured[0]

    rows.append({
        "name": name,
        "facts": {
            # The FOURCC is what is recorded; the reader derives the codec
            # through its own copy of the probe's table, so neither side can
            # name a codec no file yields. `codec` rides along for a human
            # reading the file and is not what the test consumes.
            "raw_fourcc": facts.raw_fourcc,
            "codec": facts.codec,
            "width": facts.width, "height": facts.height,
            "fps": facts.fps, "duration": facts.duration, "has_audio": facts.has_audio,
        },
        "allowance": allowance,
        "target": {
            "size": list(target.size) if target.size else None,
            "fps": target.fps,
            "trim_to": target.trim_to,
            "must_reencode": target.must_reencode,
            "may_try_av1": target.may_try_av1,
            "is_noop": target.is_noop,
        },
        # Recorded for reference, deliberately NOT asserted: CRF, preset and
        # scaler are tuning knobs (they moved twice in one week), and a test that
        # goes red on somebody's legitimate adjustment is a test that gets
        # ignored. tests/plan.rs pins the INVARIANTS of these instead -- no -ss
        # anywhere, -an always, -t only when trimming and always after -i, the
        # copy path free of filters, the re-encode path clamping what was asked.
        "commands": commands,
    })

subprocess.run = real_run

OUT.write_text(json.dumps({
    "_source": "Euterpe.Web/scripts/video_transcode.py, captured by hand",
    "_note": "A snapshot, not a live dependency. See this file's module docstring.",
    "constants": {
        "CRF": vt.CRF,
        "PRESET": vt.PRESET,
        "CRF_ESCALATION": list(vt.CRF_ESCALATION),
        "LOSSLESS_GROWTH_MARGIN": vt.LOSSLESS_GROWTH_MARGIN,
        "TRIM_TOLERANCE": vt.TRIM_TOLERANCE,
    },
    "scenarios": rows,
}, indent=2) + "\n", encoding="utf-8")

for row in rows:
    t = row["target"]
    print(f"{row['name']:<20} size={str(t['size']):<14} fps={str(t['fps']):<5} "
          f"trim={str(t['trim_to']):<7} must={int(t['must_reencode'])} "
          f"av1?={int(t['may_try_av1'])}")
print(f"\nwrote {OUT}")
