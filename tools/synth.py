"""Hand-built MP4 headers, for the boundaries no muxer will produce.

`tools/make_fixtures.ps1` covers every shape ffmpeg can be asked for. It cannot
cover a file whose media header CONTRADICTS its own sample table, because a
muxer that wrote one would be broken -- and that contradiction is exactly what
`_MAX_HEADER_STRETCH` exists to arbitrate. So those three cases are assembled
byte by byte here.

Only the boxes the probe actually reads are emitted; there is no `mdat` at all,
because the gate never looks at a single sample's bytes. Standard library only.

    uv run --no-project python tools/synth.py
"""

from __future__ import annotations

import pathlib
import struct

OUT = pathlib.Path(__file__).resolve().parents[1] / "tests" / "fixtures"


def box(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I", 8 + len(payload)) + kind + payload


def ftyp() -> bytes:
    return box(b"ftyp", b"isom" + struct.pack(">I", 512) + b"isomiso2avc1mp41")


def tkhd(track_id: int) -> bytes:
    # version 0: the probe reads track_id at body+12, which is where the spec
    # puts it after creation_time and modification_time.
    return box(b"tkhd", struct.pack(">IIII", 0, 0, 0, track_id) + b"\0" * 68)


def mdhd(timescale: int, duration: int) -> bytes:
    """The header's own opinion of how long the track runs.

    This is the number `_MAX_HEADER_STRETCH` arbitrates: it may CORRECT a sample
    table that does not divide cleanly, but never overrule one -- a header
    claiming twice its own samples' duration would halve the computed frame rate
    and wave a 120 fps file straight through.
    """
    return box(b"mdhd", struct.pack(">IIIII", 0, 0, 0, timescale, duration) + b"\0" * 4)


def hdlr(handler: bytes) -> bytes:
    return box(b"hdlr", struct.pack(">II", 0, 0) + handler + b"\0" * 12 + b"\0")


def stsd(fourcc: bytes, width: int, height: int) -> bytes:
    """One VisualSampleEntry. The probe reads the CODED size from here rather
    than from `tkhd`, because `tkhd` states the display size after any transform
    and what is rationed is pixels actually decoded."""
    entry = (
        fourcc
        + b"\0" * 6                      # reserved
        + struct.pack(">H", 1)           # data_reference_index
        + b"\0" * 16                     # pre_defined + reserved + pre_defined
        + struct.pack(">HH", width, height)
        + struct.pack(">II", 0x0048_0000, 0x0048_0000)  # 72 dpi, both axes
        + b"\0" * 4                      # reserved
        + struct.pack(">H", 1)           # frame_count
        + b"\0" * 32                     # compressorname
        + struct.pack(">H", 0x0018)      # depth
        + b"\xff\xff"                    # pre_defined = -1
    )
    entry = struct.pack(">I", 8 + len(entry)) + entry
    return box(b"stsd", struct.pack(">II", 0, 1) + entry)


def stts(runs: list[tuple[int, int]]) -> bytes:
    payload = struct.pack(">II", 0, len(runs))
    for count, delta in runs:
        payload += struct.pack(">II", count, delta)
    return box(b"stts", payload)


def movie(*, timescale: int, header_duration: int, runs: list[tuple[int, int]],
          fourcc: bytes = b"avc1", width: int = 320, height: int = 180) -> bytes:
    stbl = box(b"stbl", stsd(fourcc, width, height) + stts(runs))
    minf = box(b"minf", stbl)
    mdia = box(b"mdia", mdhd(timescale, header_duration) + hdlr(b"vide") + minf)
    trak = box(b"trak", tkhd(1) + mdia)
    return ftyp() + box(b"moov", trak)


# ── the three cases ──────────────────────────────────────────────────────
#
# All three share one sample table shape so the only variable is the header.
#
# 306 samples x 33 ticks = 10098 ticks, and 10098 % 4 == 2 -- which is what puts
# `ticks * 1.25` exactly on .5 (12622.5) and makes the rounding rule observable
# at all. Half-UP gives 12623 and half-to-even gives 12622: a 0.0019 difference
# in the reported frame rate, six orders of magnitude above any float tolerance,
# so a regression here fails loudly instead of drifting.
CASES = {
    # The header stretches the table onto an exact half tick. Cap = 12623,
    # fps = 306000 / 12623 = 24.241464.
    "synth_stretch_half.mp4": dict(timescale=1000, header_duration=20_000,
                                   runs=[(306, 33)]),

    # The header is a useful SECOND OPINION, within the 1.25x allowance, so it
    # wins outright: this is live chart 233, which reads 30.31 fps from its
    # sample table alone and 30.01 once the header is consulted. A 1% error that
    # at 60 fps would refuse a legitimate upload.
    "synth_stretch_corrects.mp4": dict(timescale=1000, header_duration=10_098 + 200,
                                       runs=[(306, 33)]),

    # The header claims roughly three times its own samples' duration. Believing
    # it would divide the frame rate and wave the file through, so it is capped
    # at 1.25x and the cap is what gets used.
    "synth_stretch_lies.mp4": dict(timescale=1000, header_duration=30_000,
                                   runs=[(306, 33)]),
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, spec in CASES.items():
        data = movie(**spec)
        (OUT / name).write_bytes(data)
        samples = sum(c for c, _ in spec["runs"])
        table = sum(c * d for c, d in spec["runs"])
        print(f"{name:<28} {len(data):>5} B   "
              f"table={table} ticks / {samples} samples   header={spec['header_duration']}")


if __name__ == "__main__":
    main()
