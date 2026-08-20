//! Cropping a video to the shape a screen actually is.
//!
//! **This is not a gate rule and Press must never present it as one.** The
//! server checks a pixel COUNT and has no opinion about shape; a 4:3 background
//! video is accepted and will play. What it will not do is fill the screen, and
//! the charter is the only person who can decide whether losing a quarter of the
//! picture is a fair trade for that.
//!
//! So this sits with the trim, in the same category and worded the same way: an
//! offer, taken by a deliberate act, never applied on Press's own initiative.
//!
//! # Two shapes, because there are two shapes
//!
//! 16:9 is what most players are on. 16:10 is what a large minority of laptops
//! are, and it is meaningfully cheaper to reach from a squarer source -- a 4:3
//! video gives up 26% of its picture to become 16:9 and 17% to become 16:10.
//! That difference is the whole decision, so both are offered and both say what
//! they cost.
//!
//! # Crop, never pad
//!
//! Padding to a shape means encoding black bars into the file: bytes spent on
//! nothing, against an 80 MB budget, to produce letterboxing the game would
//! have done for free.

/// The shapes worth offering.
///
/// Not a list to be extended. Two entries exist because two screen shapes are
/// common; a third would be a preference nobody has expressed, and every one
/// added is another decision put in front of somebody who wanted a background
/// video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Wide,
    Tall,
}

impl Shape {
    pub const ALL: [Shape; 2] = [Shape::Wide, Shape::Tall];

    /// Width and height, as the label states them.
    pub const fn ratio(self) -> (u32, u32) {
        match self {
            Self::Wide => (16, 9),
            Self::Tall => (16, 10),
        }
    }

    fn value(self) -> f64 {
        let (w, h) = self.ratio();
        f64::from(w) / f64::from(h)
    }
}

/// The charter's decision: a shape, and where its window sits along the one
/// axis that is free to move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Choice {
    pub shape: Shape,
    /// 0.0 to 1.0 along the free axis. 0.5 is centred, which is where every
    /// choice starts.
    pub offset: f32,
}

impl Choice {
    pub fn centred(shape: Shape) -> Self {
        Self { shape, offset: 0.5 }
    }

    /// The rectangle this choice keeps, or `None` when the source is already
    /// this shape and there is nothing to cut.
    pub fn window(self, from: (u32, u32)) -> Option<Window> {
        window(from, self.shape, self.offset)
    }

    pub fn along(self, from: (u32, u32)) -> Along {
        along(from, self.shape)
    }
}

/// A rectangle to keep, in source pixels. `ffmpeg`'s `crop=w:h:x:y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
}

impl Window {
    /// How much of the picture this throws away, 0.0 to 1.0.
    pub fn lost(&self, from: (u32, u32)) -> f64 {
        let whole = u64::from(from.0) * u64::from(from.1);
        if whole == 0 {
            return 0.0;
        }
        let kept = u64::from(self.width) * u64::from(self.height);
        1.0 - kept as f64 / whole as f64
    }
}

/// Which way a crop can be slid.
///
/// There is only ever one free axis: reaching a wider shape takes the sides,
/// reaching a taller one takes the top and bottom, and the other dimension is
/// already exactly right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Along {
    Horizontal,
    Vertical,
}

/// A source already this shape, near enough.
///
/// 1% of the ratio. 1920x1080 and 2560x1440 are exactly 16:9 and 1280x800 is
/// exactly 16:10, so this is not about them -- it is about an export that came
/// out 1918x1080 and would otherwise be offered a crop that removes two pixels
/// and calls it a decision.
const CLOSE_ENOUGH: f64 = 0.01;

/// Whether `shape` is worth offering for a source of these dimensions.
pub fn worth_offering(from: (u32, u32), shape: Shape) -> bool {
    let (w, h) = from;
    if w == 0 || h == 0 {
        return false;
    }
    let source = f64::from(w) / f64::from(h);
    (source - shape.value()).abs() / shape.value() > CLOSE_ENOUGH
}

/// True when the source is neither of the shapes players are usually on.
pub fn is_unusual(from: (u32, u32)) -> bool {
    Shape::ALL.iter().all(|shape| worth_offering(from, *shape))
}

/// The crop window for a shape, positioned by `offset`.
///
/// `offset` runs 0.0 to 1.0 along the free axis -- left to right, or top to
/// bottom -- and 0.5 is centred. `None` when the source is already this shape.
pub fn window(from: (u32, u32), shape: Shape, offset: f32) -> Option<Window> {
    if !worth_offering(from, shape) {
        return None;
    }
    let (w, h) = from;
    let wanted = shape.value();
    let source = f64::from(w) / f64::from(h);
    let offset = f64::from(offset.clamp(0.0, 1.0));

    // EVEN, both of them, and the origin too. yuv420p subsamples chroma by two,
    // so an odd width or an odd x is a crop ffmpeg has to correct on its own --
    // and it corrects by shifting, which lands the window somewhere nobody
    // chose.
    // The untouched axis is rounded down too. A 1081x817 source keeps "all" of
    // its height, and all of 817 is an odd number -- which ffmpeg then corrects
    // by itself, landing the window somewhere nobody chose.
    let (keep_w, keep_h) = if source > wanted {
        (
            even_size((f64::from(h) * wanted) as u32, w),
            even_size(h, h),
        )
    } else {
        (
            even_size(w, w),
            even_size((f64::from(w) / wanted) as u32, h),
        )
    };

    let slack_x = w.saturating_sub(keep_w);
    let slack_y = h.saturating_sub(keep_h);
    Some(Window {
        width: keep_w,
        height: keep_h,
        x: even_offset((f64::from(slack_x) * offset) as u32, slack_x),
        y: even_offset((f64::from(slack_y) * offset) as u32, slack_y),
    })
}

/// Which way this shape's window may be slid on this source.
pub fn along(from: (u32, u32), shape: Shape) -> Along {
    let (w, h) = from;
    if h > 0 && f64::from(w) / f64::from(h) > shape.value() {
        Along::Horizontal
    } else {
        Along::Vertical
    }
}

/// A DIMENSION: even, at least two, never past `ceiling`.
fn even_size(value: u32, ceiling: u32) -> u32 {
    let even = value.min(ceiling) / 2 * 2;
    even.max(2).min(ceiling.max(2))
}

/// An OFFSET: even, never past `ceiling`, and zero is a perfectly good answer.
///
/// Separate from [`even_size`] because a dimension has a floor of two and an
/// offset does not. Sharing one helper pushed every window two pixels off the
/// left edge, including the ones with no room to move at all.
fn even_offset(value: u32, ceiling: u32) -> u32 {
    value.min(ceiling) / 2 * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shapes in the charter's own library, and what each crop costs. These
    /// are the numbers the two options are chosen between, so they are the
    /// numbers worth pinning.
    #[test]
    fn the_cost_of_each_shape_is_what_it_should_be() {
        // 4:3-ish. Cutting to 16:9 takes a quarter of the picture; 16:10 takes
        // noticeably less, which is the entire reason both are offered.
        let squarish = (1080, 816);
        let wide = window(squarish, Shape::Wide, 0.5).expect("worth offering");
        let tall = window(squarish, Shape::Tall, 0.5).expect("worth offering");
        assert_eq!(wide.width, 1080, "a squarer source keeps its width");
        assert!(
            (wide.lost(squarish) - 0.26).abs() < 0.02,
            "16:9 from 4:3 should cost about a quarter, measured {:.3}",
            wide.lost(squarish)
        );
        assert!(
            tall.lost(squarish) < wide.lost(squarish),
            "16:10 must be the cheaper of the two here, or the choice is fake"
        );

        // Ultrawide. The sides go instead, and the height is untouched.
        let ultrawide = (5120, 2160);
        let cut = window(ultrawide, Shape::Wide, 0.5).expect("worth offering");
        assert_eq!(cut.height, 2160);
        assert_eq!(cut.width, 3840, "16:9 of 2160 is 3840");
        assert_eq!(along(ultrawide, Shape::Wide), Along::Horizontal);
    }

    /// A source already that shape is not offered a crop of it.
    #[test]
    fn an_exact_shape_is_left_alone() {
        for exact in [(1920, 1080), (2560, 1440), (1280, 720)] {
            assert!(!worth_offering(exact, Shape::Wide), "{exact:?}");
            assert!(window(exact, Shape::Wide, 0.5).is_none(), "{exact:?}");
        }
        assert!(!worth_offering((1280, 800), Shape::Tall));
        assert!(!worth_offering((1920, 1200), Shape::Tall));

        // And an export that missed by two pixels is not a decision anybody
        // needs to be asked about.
        assert!(!worth_offering((1918, 1080), Shape::Wide));
    }

    /// Odd numbers anywhere in a crop make ffmpeg move the window itself.
    #[test]
    fn every_edge_lands_on_an_even_pixel() {
        for from in [(1080, 816), (5120, 2160), (1081, 817), (999, 501)] {
            for shape in Shape::ALL {
                for offset in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let Some(window) = window(from, shape, offset) else {
                        continue;
                    };
                    assert_eq!(window.width % 2, 0, "{from:?} {shape:?} {offset}");
                    assert_eq!(window.height % 2, 0, "{from:?} {shape:?} {offset}");
                    assert_eq!(window.x % 2, 0, "{from:?} {shape:?} {offset}");
                    assert_eq!(window.y % 2, 0, "{from:?} {shape:?} {offset}");
                }
            }
        }
    }

    /// A window never reaches outside the picture, at either extreme of the
    /// slider. Off the edge is a black band welded into the result.
    #[test]
    fn a_window_never_leaves_the_picture() {
        for from in [(1080, 816), (5120, 2160), (640, 480), (720, 1280)] {
            for shape in Shape::ALL {
                for offset in [0.0, 0.5, 1.0, -1.0, 2.0] {
                    let Some(window) = window(from, shape, offset) else {
                        continue;
                    };
                    assert!(
                        window.x + window.width <= from.0,
                        "{from:?} {shape:?} {offset}: {window:?} runs off the right"
                    );
                    assert!(
                        window.y + window.height <= from.1,
                        "{from:?} {shape:?} {offset}: {window:?} runs off the bottom"
                    );
                }
            }
        }
    }

    /// Sliding moves the window and nothing else. If the size changed with the
    /// offset the shape would not be the shape that was asked for.
    #[test]
    fn sliding_moves_the_window_without_resizing_it() {
        let from = (1080, 816);
        let centre = window(from, Shape::Wide, 0.5).expect("offered");
        let top = window(from, Shape::Wide, 0.0).expect("offered");
        let bottom = window(from, Shape::Wide, 1.0).expect("offered");

        assert_eq!((top.width, top.height), (centre.width, centre.height));
        assert_eq!((bottom.width, bottom.height), (centre.width, centre.height));
        assert_eq!(top.y, 0);
        assert!(bottom.y > centre.y && centre.y > top.y);
        assert_eq!(along(from, Shape::Wide), Along::Vertical);
    }
}
