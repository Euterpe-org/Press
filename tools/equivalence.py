"""Does Press produce the same file the server's round would have produced?

That claim is the reason Press mirrors `video_transcode.py` instead of inventing
a recipe, and it is the reason a chart processed here never needs the periodic
round to touch it again — so it is worth proving rather than asserting.

This runs BOTH pipelines over the same fixtures with the same ffmpeg and
compares the results byte for byte:

    uv run --project ../Euterpe.Web python tools/equivalence.py

It is not part of CI. Byte equality holds for one ffmpeg and one SVT-AV1 build,
and Press ships its own while the server's operator uses theirs, so a mismatch
across versions is expected and is not a defect. What this checks is that the
two implementations make the same DECISIONS and issue the same commands — run it
when either side's recipe changes.
"""

from __future__ import annotations

import argparse
import hashlib
import pathlib
import shutil
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[1]
FIXTURES = REPO / "tests" / "fixtures"
DEFAULT_WEB = REPO.parent / "Euterpe.Web"

# One from each population, plus the case where AV1 competes and loses.
CASES = [
    "audio_h264.mp4",   # audio only -> stream copy, AV1 offered
    "plain_h264.mp4",   # conforming -> nothing forced, AV1 offered
    "fps120.mp4",       # frame rate -> re-encode forced
    "vp9.mp4",          # codec -> re-encode forced
]


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def press_binary() -> pathlib.Path:
    exe = REPO / "target" / "release" / "euterpe-press.exe"
    if not exe.exists():
        print("building Press...", flush=True)
        subprocess.run(["cargo", "build", "--release"], cwd=REPO, check=True)
    return exe


def run_server(web: pathlib.Path, sources: pathlib.Path, out: pathlib.Path) -> None:
    """The server's own script, unmodified, over a directory of sources."""
    script = web / "scripts" / "video_transcode.py"
    if not script.exists():
        sys.exit(f"no video_transcode.py at {script}")
    subprocess.run(
        [sys.executable, str(script), str(sources), str(out)],
        cwd=web / "backend",
        check=False,  # a FAILED chart is a result to compare, not a crash
    )


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--web", type=pathlib.Path, default=DEFAULT_WEB)
    args = ap.parse_args()

    if shutil.which("ffmpeg") is None:
        sys.exit("ffmpeg must be on PATH: both sides have to use the SAME one")

    press = press_binary()
    work = pathlib.Path(tempfile.mkdtemp(prefix="press-equivalence-"))
    sources, server_out = work / "in", work / "server"
    sources.mkdir()
    server_out.mkdir()

    # The server's script keys everything on `{cid}.mp4`, so the fixtures are
    # renamed to numbers on the way in and mapped back for the report.
    naming = {}
    for index, fixture in enumerate(CASES, start=1):
        cid = f"{index}"
        shutil.copy(FIXTURES / fixture, sources / f"{cid}.mp4")
        naming[cid] = fixture

    print("running the server's recipe...", flush=True)
    run_server(args.web, sources, server_out)

    print("running Press...", flush=True)
    rows = []
    for cid, fixture in naming.items():
        own = work / cid
        own.mkdir()
        source = own / fixture
        shutil.copy(FIXTURES / fixture, source)
        subprocess.run([str(press), "--fix", str(source)], check=False)

        # Named after the source by `job::result_name`, not a fixed name.
        ours = own / f"{source.stem}.press.mp4"
        theirs = server_out / f"{cid}.mp4"
        rows.append((fixture, ours, theirs))

    print()
    identical = differing = absent = 0
    for fixture, ours, theirs in rows:
        if not ours.exists() and not theirs.exists():
            print(f"{fixture:<22} both wrote nothing (already standard, or failed alike)")
            absent += 1
            continue
        if ours.exists() != theirs.exists():
            who = "Press" if ours.exists() else "the server"
            print(f"{fixture:<22} MISMATCH — only {who} produced a file")
            differing += 1
            continue

        a, b = sha256(ours), sha256(theirs)
        if a == b:
            print(f"{fixture:<22} identical   {ours.stat().st_size:>8} B   {a[:16]}")
            identical += 1
        else:
            print(
                f"{fixture:<22} DIFFERENT   press {ours.stat().st_size} B {a[:16]}"
                f" / server {theirs.stat().st_size} B {b[:16]}"
            )
            differing += 1

    print()
    print(f"identical {identical}   differing {differing}   no output {absent}")
    if differing:
        print(
            "\nA difference means the two implementations decided differently or "
            "issued different commands. Compare src/plan.rs against "
            "scripts/video_transcode.py before assuming it is an encoder version.",
            file=sys.stderr,
        )
    else:
        print("\nPress produces exactly what the round would have produced.")

    shutil.rmtree(work, ignore_errors=True)
    return 1 if differing else 0


if __name__ == "__main__":
    raise SystemExit(main())
