//! Background-video facts, read from the MP4 header alone.
//!
//! A faithful port of `Euterpe.Web/backend/app/domain/video_probe.py`. Every
//! quirk below is one the live library taught that file, and porting the
//! CONCLUSIONS instead of the semantics would quietly reintroduce each one --
//! so this mirrors its control flow box for box, and `tests/gate.rs` asserts the
//! two agree on every fixture rather than on my reading of them.
//!
//! Two shapes have to be handled:
//!
//!   * **plain mp4** -- `stbl/stts` lists (count, delta) runs, so sample count
//!     and duration fall straight out. 314 of the 349 live videos are this.
//!   * **fragmented mp4** -- `moov` describes the tracks but carries an EMPTY
//!     `stts`; samples live in `moof/traf/trun` spread through the file and the
//!     movie header's duration is typically 0. The other 35 are this, several
//!     common exporters emit it, and the mod plays it fine -- refusing it would
//!     be refusing a valid file.
//!
//! Bytes in, facts out. No I/O, no allocation beyond the track list, and it
//! never panics on bad input: an unreadable file is `None` and the caller
//! decides what that means.

use super::limits;

/// Sample-entry fourcc -> the codec name the gate and the UI speak.
///
/// Public because it is the ONLY way a `Facts` should ever acquire a codec.
/// Naming one directly lets a test stand on an input no file produces: a VP9
/// track has no entry here, so it arrives as `(None, "vp09")`, and a scenario
/// written as `codec: "vp9"` would be pinning the planner against fiction while
/// reporting coverage it does not have. Both this port and the server made that
/// mistake within an hour of each other, and both fixed it the same way -- by
/// removing the entry point rather than the one wrong line.
pub fn codec_of(fourcc: &[u8; 4]) -> Option<&'static str> {
    match fourcc {
        b"avc1" | b"avc3" => Some("h264"),
        b"hev1" | b"hvc1" => Some("h265"),
        b"av01" => Some("av1"),
        _ => None,
    }
}

/// Sample entries that identify a track as VIDEO even though the codec will not
/// be accepted. Used only when `mdia/hdlr` is missing or odd -- naming them is
/// what turns "we found no video" into "this file is VP9", which is a sentence
/// the charter can act on.
///
/// `jpeg` is deliberately absent: a JPEG track in an MP4 is a poster frame, not
/// the background video, and treating it as one would refuse a good file for the
/// codec of its thumbnail.
const KNOWN_VIDEO: [&[u8; 4]; 12] = [
    b"avc1", b"avc3", b"hev1", b"hvc1", b"av01",
    b"vp08", b"vp09", b"mp4v", b"dvh1", b"dvhe", b"vc-1", b"s263",
];

/// Sample entries that mark a track as AUDIO when its handler is missing or odd.
/// The handler (`soun`) is the primary test; this is the same second witness the
/// video side gets.
const KNOWN_AUDIO: [&[u8; 4]; 11] = [
    b"mp4a", b"ac-3", b"ec-3", b"alac", b"Opus", b"fLaC", b".mp3",
    b"samr", b"sowt", b"twos", b"lpcm",
];

/// Containers worth descending into. Anything else is skipped whole.
const CONTAINERS: [&[u8; 4]; 8] =
    [b"moov", b"trak", b"mdia", b"minf", b"stbl", b"mvex", b"moof", b"traf"];

/// A malformed file must cost a bounded amount of work, not an unbounded one.
const MAX_DEPTH: u32 = 8;
const MAX_BOXES: u64 = 500_000;

/// Half a frame of grace at the limit. A POLICY choice, not a float tolerance:
/// what it forgives is a header whose numbers do not divide cleanly, and the
/// price is that a genuine 60.4 fps upload is let through. That price is
/// nothing -- across the live library every 60 fps file computes to exactly 60.0
/// (one reads 59.94) and the next value that exists at all is 90.0, so the range
/// this opens is empty. Refusing a real 60 fps video for a rounding hair would
/// not be.
const FPS_EPSILON: f64 = 0.5;

/// How far the media header may stretch the sample table before it stops being a
/// correction and starts being a contradiction. Live chart 233 reads 30.31 fps
/// from its sample table alone and 30.01 once the header is consulted -- a 1%
/// error that at 60 fps would refuse a legitimate upload. But a header claiming
/// the track runs twice as long as its own samples is not rounding, and
/// believing it would halve the computed frame rate and wave a 120 fps file
/// straight through.
const MAX_HEADER_STRETCH: f64 = 1.25;

/// What the header says.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    /// h264 / h265 / av1, or `None` for a video track we do not accept.
    pub codec: Option<&'static str>,
    /// What was actually there, for the rejection message.
    pub raw_fourcc: String,
    pub width: u32,
    pub height: u32,
    /// The track average. Variable-rate footage has no single number, and the
    /// average is the honest one to gate on.
    pub fps: f64,
    pub duration: f64,
    /// A background video's audio is never played: the game plays `music.ogg`.
    pub has_audio: bool,
}

#[derive(Default, Clone)]
struct Track {
    id: Option<u32>,
    kind: Option<[u8; 4]>,
    fourcc: Option<[u8; 4]>,
    timescale: Option<u64>,
    ticks: Option<u128>,
    width: Option<u32>,
    height: Option<u32>,
    stts_samples: u128,
    stts_ticks: u128,
}

struct Walker<'a> {
    d: &'a [u8],
    boxes: u64,
    /// Every track ever opened. `registry` decides which are addressable and in
    /// what order; a `trak` with no `tkhd` stays here and is never seen, exactly
    /// as the Python dict never receives it.
    slots: Vec<Track>,
    /// (track_id, slot). A Vec rather than a map because Python iterates its
    /// `tracks` dict in INSERTION order and re-assigning an existing id keeps
    /// that id's original position -- which decides which track a file with two
    /// video tracks is judged on.
    registry: Vec<(u32, usize)>,
    cur: Option<usize>,
    /// track_id -> default_sample_duration, from `mvex/trex`.
    trex: Vec<(u32, u32)>,
    /// track_id -> (samples, ticks) accumulated across every fragment.
    frag: Vec<(u32, u128, u128)>,
    traf_id: Option<u32>,
    traf_default: Option<u32>,
    saw_ftyp: bool,
}

impl<'a> Walker<'a> {
    fn new(d: &'a [u8]) -> Self {
        Self {
            d,
            boxes: 0,
            slots: Vec::new(),
            registry: Vec::new(),
            cur: None,
            trex: Vec::new(),
            frag: Vec::new(),
            traf_id: None,
            traf_default: None,
            saw_ftyp: false,
        }
    }

    // ── primitives ───────────────────────────────────────────────────────
    // Out-of-range reads yield 0 rather than panicking. Every caller already
    // bounds-checks the way the Python does; this is the backstop that keeps a
    // malformed file from taking the process down with it.
    fn u16(&self, o: usize) -> u32 {
        match self.d.get(o..o + 2) {
            Some(b) => u32::from(u16::from_be_bytes([b[0], b[1]])),
            None => 0,
        }
    }

    fn u32(&self, o: usize) -> u32 {
        match self.d.get(o..o + 4) {
            Some(b) => u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            None => 0,
        }
    }

    fn u64(&self, o: usize) -> u64 {
        match self.d.get(o..o + 8) {
            Some(b) => u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
            None => 0,
        }
    }

    fn fourcc(&self, o: usize) -> [u8; 4] {
        match self.d.get(o..o + 4) {
            Some(b) => [b[0], b[1], b[2], b[3]],
            None => [0; 4],
        }
    }

    fn track(&mut self) -> Option<&mut Track> {
        self.cur.map(|i| &mut self.slots[i])
    }

    // ── the walk ─────────────────────────────────────────────────────────
    fn walk(&mut self, start: usize, end: usize, depth: u32, parent: [u8; 4]) {
        if depth > MAX_DEPTH {
            return;
        }
        let mut o = start;
        while o + 8 <= end {
            self.boxes += 1;
            if self.boxes > MAX_BOXES {
                return;
            }
            let mut size = u64::from(self.u32(o));
            let typ = self.fourcc(o + 4);
            let mut body = o + 8;
            if size == 1 {
                if o + 16 > end {
                    return;
                }
                size = self.u64(o + 8);
                body = o + 16;
            } else if size == 0 {
                size = (end - o) as u64;
            }
            // Truncated or lying -- stop, and keep whatever was read so far.
            if size < 8 || o as u64 + size > end as u64 {
                return;
            }
            let stop = o + size as usize;
            self.handle(typ, body, stop, depth, parent);
            o = stop;
        }
    }

    fn handle(&mut self, typ: [u8; 4], body: usize, stop: usize, depth: u32, parent: [u8; 4]) {
        if CONTAINERS.contains(&&typ) {
            match &typ {
                b"trak" => {
                    self.slots.push(Track::default());
                    self.cur = Some(self.slots.len() - 1);
                }
                // The fragments describe no tracks. Leaving `cur` pointing at the
                // last `trak` would let a stray box inside a `moof` write into
                // that track's facts -- and the browser, which walks each `moof`
                // with no current track at all, would not. Same file, two
                // verdicts.
                b"moof" => self.cur = None,
                b"traf" => {
                    self.traf_id = None;
                    self.traf_default = None;
                }
                _ => {}
            }
            self.walk(body, stop, depth + 1, typ);
            return;
        }

        match &typ {
            // Top level only. A `ftyp` buried inside `moov` is not a file-type
            // declaration, and counting it would accept a file the browser's
            // top-level scan refuses.
            b"ftyp" if depth == 0 => self.saw_ftyp = true,
            b"tkhd" => self.tkhd(body, stop),
            b"mdhd" => self.mdhd(body, stop),
            // ONLY the media handler, which is a direct child of `mdia`. A
            // QuickTime-derived file hangs a second `hdlr` straight off `minf`
            // naming its DATA handler (`alis`), and taking that one instead made
            // 13 perfectly good videos in the live library look like they had no
            // video track at all.
            b"hdlr" if parent == *b"mdia" => {
                if body + 12 <= stop {
                    let kind = self.fourcc(body + 8);
                    if let Some(t) = self.track() {
                        t.kind = Some(kind);
                    }
                }
            }
            b"stsd" => self.stsd(body, stop),
            b"stts" => self.stts(body, stop),
            b"trex" => {
                if body + 16 <= stop {
                    let (id, dur) = (self.u32(body + 4), self.u32(body + 12));
                    self.trex.retain(|(t, _)| *t != id);
                    self.trex.push((id, dur));
                }
            }
            b"tfhd" => self.tfhd(body, stop),
            b"trun" => self.trun(body, stop),
            _ => {}
        }
    }

    fn tkhd(&mut self, body: usize, stop: usize) {
        if body + 4 > stop || self.cur.is_none() {
            return;
        }
        let ver = self.d.get(body).copied().unwrap_or(0);
        let o = body + 4 + if ver == 1 { 16 } else { 8 };
        if o + 4 > stop {
            return;
        }
        let id = self.u32(o);
        let slot = self.cur.expect("checked above");
        self.slots[slot].id = Some(id);
        // Python's `self.tracks[tid] = self.cur`: an existing id is REPLACED and
        // keeps its original iteration position.
        match self.registry.iter_mut().find(|(t, _)| *t == id) {
            Some(entry) => entry.1 = slot,
            None => self.registry.push((id, slot)),
        }
    }

    fn mdhd(&mut self, body: usize, stop: usize) {
        if body + 4 > stop || self.cur.is_none() {
            return;
        }
        let ver = self.d.get(body).copied().unwrap_or(0);
        let o = body + 4 + if ver == 1 { 16 } else { 8 };
        let need = if ver == 1 { 12 } else { 8 };
        if o + need > stop {
            return;
        }
        let timescale = u64::from(self.u32(o));
        let ticks = if ver == 1 {
            u128::from(self.u64(o + 4))
        } else {
            u128::from(self.u32(o + 4))
        };
        if let Some(t) = self.track() {
            t.timescale = Some(timescale);
            t.ticks = Some(ticks);
        }
    }

    /// First sample entry: fourcc, then (for a visual entry) coded size.
    ///
    /// The CODED size is read here rather than `tkhd`'s, because `tkhd` states
    /// the DISPLAY size after any transform -- a rotated phone clip reports its
    /// display box, and what is being rationed is pixels actually decoded.
    fn stsd(&mut self, body: usize, stop: usize) {
        let o = body + 8; // version/flags + entry_count
        if o + 16 > stop || self.cur.is_none() {
            return;
        }
        let fourcc = self.fourcc(o + 4);
        let vis = o + 8 + 8 + 16; // sample-entry header + pre_defined/reserved
        let size = if vis + 4 <= stop {
            Some((self.u16(vis), self.u16(vis + 2)))
        } else {
            None
        };
        if let Some(t) = self.track() {
            t.fourcc = Some(fourcc);
            if let Some((w, h)) = size {
                t.width = Some(w);
                t.height = Some(h);
            }
        }
    }

    fn stts(&mut self, body: usize, stop: usize) {
        if body + 8 > stop || self.cur.is_none() {
            return;
        }
        let declared = self.u32(body + 4) as usize;
        let o = body + 8;
        let n = declared.min(stop.saturating_sub(o) / 8);
        let (mut samples, mut ticks) = (0u128, 0u128);
        for i in 0..n {
            let at = o + i * 8;
            let (cnt, delta) = (u128::from(self.u32(at)), u128::from(self.u32(at + 4)));
            samples += cnt;
            ticks += cnt * delta;
        }
        if let Some(t) = self.track() {
            t.stts_samples = samples;
            t.stts_ticks = ticks;
        }
    }

    fn tfhd(&mut self, body: usize, stop: usize) {
        if body + 8 > stop {
            return;
        }
        let flags = self.u32(body) & 0x00FF_FFFF;
        self.traf_id = Some(self.u32(body + 4));
        let mut o = body + 8;
        if flags & 0x00_0001 != 0 {
            o += 8; // base_data_offset
        }
        if flags & 0x00_0002 != 0 {
            o += 4; // sample_description_index
        }
        self.traf_default = if flags & 0x00_0008 != 0 && o + 4 <= stop {
            Some(self.u32(o))
        } else {
            None
        };
    }

    /// Sample count, and duration from the best source this fragment has: its own
    /// per-sample list, else the traf default, else the trex default.
    fn trun(&mut self, body: usize, stop: usize) {
        if body + 8 > stop {
            return;
        }
        let Some(track_id) = self.traf_id else {
            return;
        };
        let flags = self.u32(body) & 0x00FF_FFFF;
        let mut count = self.u32(body + 4) as usize;
        let mut o = body + 8;
        if flags & 0x00_0001 != 0 {
            o += 4; // data_offset
        }
        if flags & 0x00_0004 != 0 {
            o += 4; // first_sample_flags
        }
        let has_duration = flags & 0x00_0100 != 0;
        let present = [0x00_0100, 0x00_0200, 0x00_0400, 0x00_0800]
            .iter()
            .filter(|f| flags & **f != 0)
            .count();
        // A stride of zero means the run declares no per-sample fields at all,
        // so there is nothing to read and nothing to clamp against -- the count
        // stands as declared and the durations come from a default below.
        let stride = 4 * present;
        if let Some(readable) = stop.saturating_sub(o).checked_div(stride) {
            count = count.min(readable);
        }

        let mut added_ticks = 0u128;
        if has_duration {
            for i in 0..count {
                added_ticks += u128::from(self.u32(o + i * stride));
            }
        } else {
            let step = self
                .traf_default
                .or_else(|| self.trex.iter().find(|(t, _)| *t == track_id).map(|(_, d)| *d));
            // Python: `if step:` -- a default duration of zero is no duration.
            if let Some(step) = step.filter(|s| *s != 0) {
                added_ticks = count as u128 * u128::from(step);
            }
        }

        match self.frag.iter_mut().find(|(t, _, _)| *t == track_id) {
            Some(slot) => {
                slot.1 += count as u128;
                slot.2 += added_ticks;
            }
            None => self.frag.push((track_id, count as u128, added_ticks)),
        }
    }
}

/// The sample entry's name, but only if it is something a person can read.
///
/// A track can be identified as video by its handler while carrying garbage
/// where its codec name should be, and that garbage would otherwise be
/// interpolated straight into the rejection sentence -- control bytes and all.
/// An empty string here becomes "unknown" in the message.
///
/// Only ASCII SPACES are trimmed, and printability is judged BEFORE any
/// trimming: Python's `str.strip()` also eats U+001C..U+001F where JavaScript's
/// `trim()` does not, so the three sides would otherwise print different codec
/// names for the same file.
fn printable_fourcc(fourcc: &[u8; 4]) -> String {
    if fourcc.iter().any(|b| *b < 0x20 || *b > 0x7E) {
        return String::new();
    }
    String::from_utf8_lossy(fourcc).trim_matches(' ').to_owned()
}

/// The track to judge: the first whose media handler says `vide`, and failing
/// that the first whose sample entry names a video codec.
///
/// The fallback is not redundant. A track that omits `mdia/hdlr` -- or writes
/// something unusual in it -- is malformed but perfectly playable, and with only
/// the handler test it would come back as "no video track" and be refused as
/// unreadable. The sample entry is the second, independent witness that this is
/// video, and it lets an unaccepted codec be REPORTED as such rather than
/// disappearing into the generic verdict.
///
/// Among several video tracks, one we can actually PLAY wins: a file whose first
/// video track is a legacy or preview stream and whose second is the real H.264
/// is refused for the wrong track's codec otherwise. Known limitation,
/// deliberately not chased further -- with two playable video tracks only the
/// first is judged, so a decoy could hide a larger stream behind a small one.
/// What that buys is bounded by the size limit, which counts the whole file.
fn video_track(w: &Walker) -> Option<usize> {
    let ordered: Vec<usize> = w.registry.iter().map(|(_, slot)| *slot).collect();

    let mut candidates: Vec<usize> = ordered
        .iter()
        .copied()
        .filter(|i| w.slots[*i].kind == Some(*b"vide"))
        .collect();
    if candidates.is_empty() {
        candidates = ordered
            .iter()
            .copied()
            .filter(|i| {
                w.slots[*i]
                    .fourcc
                    .is_some_and(|f| KNOWN_VIDEO.iter().any(|k| **k == f))
            })
            .collect();
    }
    if candidates.is_empty() {
        return None;
    }
    candidates
        .iter()
        .copied()
        .find(|i| {
            let codec = w.slots[*i].fourcc.and_then(|f| codec_of(&f));
            limits::codec_allowed(codec)
        })
        .or_else(|| candidates.first().copied())
}

/// Facts for the first video track, or `None` if this is not a readable MP4.
///
/// `None` covers everything the gate cannot judge: not an MP4, no video track,
/// truncated, or a header that carries no usable timing. All of them mean the
/// same thing to a charter -- re-export as a standard MP4 -- so they are one
/// outcome rather than a taxonomy nobody can act on differently.
pub fn probe(data: &[u8]) -> Option<Facts> {
    if data.len() < 8 {
        return None;
    }

    let mut w = Walker::new(data);
    w.walk(0, data.len(), 0, [0; 4]);

    // `ftyp` must be PRESENT, not first. The spec only asks for it as early as
    // possible, and some muxers reserve a leading `free`/`wide` box ahead of it
    // -- requiring byte offset 4 would refuse a file the old check accepted,
    // which is the expensive direction to be wrong in. Requiring it at all is
    // what still refuses a bare QuickTime `.mov` renamed to `.mp4`.
    if !w.saw_ftyp {
        return None;
    }

    let vid = video_track(&w)?;
    let track = w.slots[vid].clone();
    let fourcc = track.fourcc?;

    let timescale = track.timescale.unwrap_or(0);

    // SUM, not either-or. A plain file has no fragments and a fragmented one has
    // an empty sample table, so for both this is just the half that exists --
    // but a HYBRID carries both, and taking only the table would report the
    // first ten seconds of a file whose fragments run at a different rate
    // entirely, in whichever direction happens to hurt.
    let (frag_samples, frag_ticks) = track
        .id
        .and_then(|id| w.frag.iter().find(|(t, _, _)| *t == id))
        .map_or((0, 0), |(_, s, t)| (*s, *t));
    let samples = track.stts_samples + frag_samples;
    let mut ticks = track.stts_ticks + frag_ticks;

    let header_ticks = track.ticks.unwrap_or(0);
    if ticks == 0 {
        ticks = header_ticks;
    } else if header_ticks > ticks {
        // Half-UP, which is what `f64::round()` already does. The server spells
        // it `floor(x + 0.5)` because Python's `round()` sends halves to EVEN
        // (Euterpe.Web `d5eeba36`, which unified on half-up precisely so that
        // every other implementation of this gate could use its own language's
        // rounding instead of carrying a helper that has to be kept correct).
        //
        // `.round()` and `floor(x + 0.5)` are not interchangeable in general --
        // that equivalence is what gave Java's `Math.round(0.49999999999999994)`
        // its famous wrong answer -- but they are here: `ticks` is integral and
        // 1.25 is exactly representable, so the product's fraction is always
        // one of 0, .25, .5, .75 and never the pathological value.
        let stretched = (ticks as f64 * MAX_HEADER_STRETCH).round() as u128;
        ticks = header_ticks.min(stretched);
    }

    let (width, height) = (track.width.unwrap_or(0), track.height.unwrap_or(0));

    // Dimensions we could not read are NOT dimensions of zero: 0 x 0 passes a
    // pixel-count limit that the real frame might not. Every one of the live
    // library's 349 videos states them, so a file that does not is one we cannot
    // judge rather than one we may wave through.
    if timescale == 0 || ticks == 0 || samples == 0 || width == 0 || height == 0 {
        return None;
    }

    let duration = ticks as f64 / timescale as f64;
    let has_audio = w.registry.iter().any(|(_, slot)| {
        *slot != vid
            && (w.slots[*slot].kind == Some(*b"soun")
                || w.slots[*slot]
                    .fourcc
                    .is_some_and(|f| KNOWN_AUDIO.iter().any(|k| **k == f)))
    });

    Some(Facts {
        codec: codec_of(&fourcc),
        raw_fourcc: printable_fourcc(&fourcc),
        width,
        height,
        fps: samples as f64 / duration,
        duration,
        has_audio,
    })
}

/// The frame rate as the rejection message states it.
///
/// `floor(x + 0.5)`, not `round()`: Python rounds halves to even and JavaScript
/// rounds them up, so a file computing exactly 62.5 fps would otherwise be
/// refused as "62 fps" by one implementation and "63 fps" by another.
pub fn reported_fps(fps: f64) -> i64 {
    (fps + 0.5).floor() as i64
}

pub fn over_fps_limit(fps: f64) -> bool {
    fps > f64::from(limits::MAX_FPS) + FPS_EPSILON
}
