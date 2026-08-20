"""Record what the SERVER's gate says about every fixture, field for field.

Press carries a third implementation of the upload gate (the first two being
``Euterpe.Web/backend/app/domain/video_probe.py`` and its browser mirror
``frontend/src/utils/videoProbe.ts``). A third implementation is a third chance
to disagree, and a charter who is told "this file is fine" by a tool and
"refused" by the server has been actively misled -- worse than no tool at all.

So the expectations are not hand-written here. This runs the SERVER's own code
over the fixtures and writes down what it answered; `tests/gate.rs` then asserts
the Rust port answers identically. Regenerate it whenever Euterpe.Web moves:

    uv run --project ../Euterpe.Web python tools/golden.py

`--check` re-runs it and fails instead of writing, which is what CI does: if the
server's behaviour has moved and Press has not, the build goes red rather than
shipping a tool that lies. Also pins the four thresholds, so a change to
`chart_constants.py` is caught even when no fixture's verdict happens to move.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
FIXTURES = REPO / "tests" / "fixtures"
GOLDEN = FIXTURES / "golden.json"

# Where the authority lives. Overridable because a checkout is not always a
# sibling -- CI clones it wherever it likes.
DEFAULT_WEB = REPO.parent / "Euterpe.Web"


def load_gate(web: pathlib.Path):
    backend = web / "backend"
    if not (backend / "app" / "domain" / "video_probe.py").exists():
        sys.exit(
            f"no Euterpe.Web backend at {backend}\n"
            "pass --web <path>, or clone it beside this repo."
        )
    sys.path.insert(0, str(backend))
    from app.domain import video_probe
    from app.utils import chart_constants
    return video_probe, chart_constants


def facts_of(video_probe, data: bytes) -> dict | None:
    facts = video_probe.probe(data)
    if facts is None:
        return None
    return {
        "codec": facts.codec,
        "raw_fourcc": facts.raw_fourcc,
        "width": facts.width,
        "height": facts.height,
        # Six places: enough that a real parsing difference shows, loose enough
        # that IEEE-754 division ordering between Python and Rust does not.
        "fps": round(facts.fps, 6),
        "duration": round(facts.duration, 6),
        "has_audio": facts.has_audio,
    }


def build(video_probe, chart_constants) -> dict:
    rows = []
    for path in sorted(FIXTURES.iterdir()):
        if path.name == GOLDEN.name or path.is_dir():
            continue
        data = path.read_bytes()
        facts = facts_of(video_probe, data)
        verdict = video_probe.check(len(data), video_probe.probe(data))
        rows.append({
            "file": path.name,
            "size": len(data),
            "facts": facts,
            "verdict": None if verdict is None else {"code": verdict[0], "params": verdict[1]},
        })

    return {
        # Pinned separately from the fixtures: a threshold can move without any
        # fixture crossing it, and that still has to fail the build.
        "constants": {
            "VIDEO_MAX_BYTES": chart_constants.VIDEO_MAX_BYTES,
            "VIDEO_MAX_WIDTH": chart_constants.VIDEO_MAX_WIDTH,
            "VIDEO_MAX_HEIGHT": chart_constants.VIDEO_MAX_HEIGHT,
            "VIDEO_MAX_PIXELS": chart_constants.VIDEO_MAX_PIXELS,
            "VIDEO_MAX_FPS": chart_constants.VIDEO_MAX_FPS,
            "VIDEO_ALLOWED_CODECS": sorted(chart_constants.VIDEO_ALLOWED_CODECS),
            "VIDEO_OVERRUN_ALLOWANCE_SECONDS": chart_constants.VIDEO_OVERRUN_ALLOWANCE_SECONDS,
        },
        "fixtures": rows,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--web", type=pathlib.Path, default=DEFAULT_WEB,
                    help="path to the Euterpe.Web checkout (default: beside this repo)")
    ap.add_argument("--check", action="store_true",
                    help="fail if the recorded answers no longer match; write nothing")
    args = ap.parse_args()

    video_probe, chart_constants = load_gate(args.web)
    fresh = build(video_probe, chart_constants)

    if args.check:
        if not GOLDEN.exists():
            sys.exit(f"{GOLDEN} does not exist - run without --check first")
        recorded = json.loads(GOLDEN.read_text(encoding="utf-8"))
        if recorded == fresh:
            print(f"gate unchanged - {len(fresh['fixtures'])} fixtures agree")
            return 0
        print("THE SERVER'S GATE HAS MOVED.\n", file=sys.stderr)
        if recorded.get("constants") != fresh["constants"]:
            print(f"  constants: {recorded.get('constants')}\n"
                  f"          -> {fresh['constants']}\n", file=sys.stderr)
        was = {r["file"]: r for r in recorded.get("fixtures", [])}
        for row in fresh["fixtures"]:
            if was.get(row["file"]) != row:
                print(f"  {row['file']}:\n"
                      f"    was {json.dumps(was.get(row['file']), ensure_ascii=False)}\n"
                      f"    now {json.dumps(row, ensure_ascii=False)}", file=sys.stderr)
        print("\nPort the change into src/gate/, then re-run without --check.", file=sys.stderr)
        return 1

    GOLDEN.write_text(json.dumps(fresh, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    for row in fresh["fixtures"]:
        f = row["facts"]
        shape = "unreadable" if f is None else (
            f"{f['codec'] or f['raw_fourcc'] or '?'} {f['width']}x{f['height']} "
            f"{f['fps']:.2f}fps {f['duration']:.2f}s audio={f['has_audio']}")
        print(f"{row['file']:<24} {row['size'] // 1024:>5} KB  {shape:<44} "
              f"-> {row['verdict']['code'] if row['verdict'] else 'PASS'}")
    print(f"\nwrote {GOLDEN}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
