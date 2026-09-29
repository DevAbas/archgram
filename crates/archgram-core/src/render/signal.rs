//! The flows' animation as SMIL (DESIGN.md, Motion and Components: Signal):
//! a signal per hop, in the style the spec names, following its edge's own
//! path, with its edge's label over it in the signal's colour; and, while a
//! signal is at a card, the card lit and its logo in its brand's colour. SMIL runs where CSS and scripts do not, in an `<img>`
//! and on GitHub. Every element starts invisible, so a reader that runs no
//! animation shows the still diagram.

use crate::geometry::Rect;
use std::fmt::Write as _;

use crate::motion::{Hop, Lit, Timeline, fade};
use crate::render::edge::{Drawn, edge_label};
use crate::render::scene::{GroupOf, Item};
use crate::render::svg::{inline, num};
use crate::spec::SignalStyle;
use crate::tokens::{
    MOTION_EASE, MOTION_HOP_GAP_MS, SIGNAL_BLUR, SIGNAL_BOLT, SIGNAL_BOLT_WIDTH, SIGNAL_COMET,
    SIGNAL_CORE, SIGNAL_DASH, SIGNAL_DASH_PERIOD_MS, SIGNAL_DOT, SIGNAL_FLICKER_FAST_MS,
    SIGNAL_FLICKER_MS, SIGNAL_HALO, SIGNAL_RING, SIGNAL_RIPPLE, SIGNAL_TRAIL,
};

/// The keyframes of one animation: times in whole milliseconds within the
/// cycle, each with its value. The only way keyTimes are written, so the
/// rules SMIL sets on them hold for every animation (ARCHITECTURE.md,
/// Invariants): the first time is 0, the last is the whole cycle, times
/// never go back, and there is one value per time.
#[derive(Debug)]
pub struct KeyTrack {
    period: u32,
    keys: Vec<(u32, String)>,
}

impl KeyTrack {
    #[must_use]
    pub fn new(period: u32) -> Self {
        KeyTrack {
            period,
            keys: Vec::new(),
        }
    }

    /// Adds a key at `t`, held to the cycle and to the key before it.
    pub fn key(&mut self, t: u32, value: &str) -> &mut Self {
        let floor = self.keys.last().map_or(0, |k| k.0);
        debug_assert!(
            t >= floor && t <= self.period,
            "key at {t} after {floor} in {}",
            self.period
        );
        self.keys
            .push((t.clamp(floor, self.period), value.to_owned()));
        self
    }

    /// `values` (or `keyPoints` for a motion) and `keyTimes`, the cycle's
    /// length and its repeat.
    ///
    /// # Panics
    ///
    /// When the track does not start at 0 and end at the cycle's end.
    #[must_use]
    pub fn attributes(&self, values: &str) -> String {
        assert!(
            self.keys.first().is_some_and(|k| k.0 == 0)
                && self.keys.last().is_some_and(|k| k.0 == self.period),
            "a track runs the whole cycle"
        );
        let vals: Vec<&str> = self.keys.iter().map(|k| k.1.as_str()).collect();
        let times: Vec<String> = self
            .keys
            .iter()
            .map(|k| fraction(k.0, self.period))
            .collect();
        format!(
            r#"dur="{}ms" repeatCount="indefinite" {values}="{}" keyTimes="{}""#,
            self.period,
            vals.join(";"),
            times.join(";")
        )
    }

    /// The number of keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// `t` over `period` to four decimals, by integer arithmetic: a tenth of a
/// millisecond per second of cycle, finer than a frame.
fn fraction(t: u32, period: u32) -> String {
    let k = (u64::from(t) * 10_000 + u64::from(period) / 2) / u64::from(period.max(1));
    match k {
        0 => "0".into(),
        10_000 => "1".into(),
        _ => {
            let s = format!("0.{k:04}");
            s.trim_end_matches('0').to_owned()
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn ms(v: f64) -> u32 {
    v.round().max(0.0) as u32
}

/// The easing of a hop, between two holds.
fn splines() -> String {
    let e = MOTION_EASE.map(num);
    format!(
        r#"calcMode="spline" keySplines="0 0 1 1;{} {} {} {};0 0 1 1""#,
        e[0], e[1], e[2], e[3]
    )
}

/// A hop's progress: still at its start until `start`, eased to its end by
/// `end`, then still. Shared by the moving mark and a line drawn along.
fn progress(hop: &Hop, period: u32, from: &str, to: &str) -> String {
    let mut t = KeyTrack::new(period);
    t.key(0, from)
        .key(hop.start, from)
        .key(hop.end, to)
        .key(period, to);
    format!("{} {}", t.attributes("values"), splines())
}

/// Seen from `start` to `end`, fading in and out over `motion.fade`.
fn seen(start: u32, end: u32, period: u32) -> String {
    let f = fade().min(end.saturating_sub(start) / 2);
    let mut t = KeyTrack::new(period);
    t.key(0, "0")
        .key(start, "0")
        .key(start + f, "1")
        .key(end - f, "1")
        .key(end, "0")
        .key(period, "0");
    format!(
        r#"<animate attributeName="opacity" {}/>"#,
        t.attributes("values")
    )
}

/// The lit times of one card as an opacity animation: fully on from each
/// start to each end, fading over `motion.fade` either side.
#[must_use]
pub fn lit_animation(lit: &[Lit], period: u32) -> String {
    let f = fade();
    let mut t = KeyTrack::new(period);
    t.key(0, "0");
    for l in lit {
        t.key(l.start.saturating_sub(f), "0")
            .key(l.start, "1")
            .key(l.end, "1")
            .key((l.end + f).min(period), "0");
    }
    t.key(period, "0");
    format!(
        r#"<animate attributeName="opacity" {}/>"#,
        t.attributes("values")
    )
}

/// A flicker: the opacity wavering on its own short loop.
fn flicker(period_ms: f64, values: &str) -> String {
    format!(
        r#"<animate attributeName="opacity" dur="{}ms" repeatCount="indefinite" values="{values}"/>"#,
        ms(period_ms)
    )
}

/// The mark moving along edge `id`.
fn mover(id: &str, hop: &Hop, period: u32) -> String {
    let mut t = KeyTrack::new(period);
    t.key(0, "0")
        .key(hop.start, "0")
        .key(hop.end, "1")
        .key(period, "1");
    format!(
        r##"<animateMotion {} {}><mpath href="#{id}"/></animateMotion>"##,
        t.attributes("keyPoints"),
        splines()
    )
}

/// A stretch `px` long of the edge's line, its front at the moving mark.
fn streak(
    d: &str,
    length: f64,
    px: f64,
    class: &str,
    style: &str,
    hop: &Hop,
    period: u32,
) -> String {
    let t = 1000.0 * px / length.max(1.0);
    let (on, gone) = (num(t), num(t - 1000.0));
    format!(
        r#"<path class="{class}"{style} d="{d}" pathLength="1000" stroke-dasharray="{on} {}"><animate attributeName="stroke-dashoffset" {}/></path>"#,
        num(1000.0 + t),
        progress(hop, period, &on, &gone)
    )
}

/// Every signal, drawn above the cards. `labels` holds each edge's label
/// and its box, by the edge's index. On a labelled edge the signal's line
/// and glow fade out round the label (`label_gap`), with no patch to box
/// the text, and a copy of the label's text above them takes the signal's
/// colour for as long as the signal is seen.
pub fn signals(
    style: SignalStyle,
    timeline: &Timeline,
    drawn: &[Drawn],
    labels: &[Option<(&str, Rect)>],
    hue: impl Fn(usize) -> &'static str,
) -> Item {
    let mut items = Vec::new();
    let period = timeline.period;
    for hop in &timeline.hops {
        let id = edge_id(hop.edge);
        let d = drawn[hop.edge].d();
        let length = drawn[hop.edge].length();
        let body = mark(style, hop, period, (&id, &d, length));
        // A filled line stays while the card it reached is lit, then fades
        // with it; every other mark goes as it arrives.
        let shown = if style == SignalStyle::Wire {
            let end = (hop.end + ms(MOTION_HOP_GAP_MS) + fade()).min(period);
            seen(hop.start, end, period)
        } else {
            seen(hop.start, hop.end, period)
        };
        let (body, label) = match labels.get(hop.edge).copied().flatten() {
            Some((text, at)) => {
                let [_, copy] = edge_label(text, at, ("label-patch", "sub lit-text"));
                (
                    format!(r#"<g mask="url(#{GAP})">{body}</g>"#),
                    inline(&[copy]),
                )
            }
            None => (body, String::new()),
        };
        items.push(Item::Motion(format!(
            r#"<g class="signal {}" opacity="0">{shown}{body}{label}</g>"#,
            hue(hop.from)
        )));
    }
    Item::Group {
        of: GroupOf::Class("signals"),
        items,
    }
}

/// The mask a signal on a labelled edge is drawn through.
const GAP: &str = "label-gap";

/// The mask that keeps signals off every label in `boxes`, in a drawing
/// `width` by `height`: clear everywhere but over each label, where a black
/// box softened by `signal.blur` fades the line and its glow out before the
/// text, as the still line stops at the label's patch.
#[must_use]
pub fn label_gap(boxes: &[Rect], width: f64, height: f64) -> Option<String> {
    if boxes.is_empty() {
        return None;
    }
    let area = format!(
        r#"x="0" y="0" width="{}" height="{}""#,
        num(width),
        num(height)
    );
    let mut holes = String::new();
    for r in boxes {
        let _ = write!(
            holes,
            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#000" filter="url(#{GAP}-soft)"/>"##,
            num(r.x),
            num(r.y),
            num(r.w),
            num(r.h)
        );
    }
    Some(format!(
        r##"<defs><filter id="{GAP}-soft" filterUnits="userSpaceOnUse" {area}><feGaussianBlur stdDeviation="{}"/></filter><mask id="{GAP}" maskUnits="userSpaceOnUse" {area}><rect {area} fill="#fff"/>{holes}</mask></defs>"##,
        num(SIGNAL_BLUR)
    ))
}

/// The mark one hop draws in `style`: along edge `id`, whose path is `d`
/// and `length` long.
fn mark(style: SignalStyle, hop: &Hop, period: u32, (id, d, length): (&str, &str, f64)) -> String {
    let head = |inner: &str| format!("<g>{inner}{}</g>", mover(id, hop, period));
    match style {
        SignalStyle::Wire => {
            let fill = |class: &str| {
                format!(
                    r#"<path class="{class}" d="{d}" pathLength="1000" stroke-dasharray="1000 1000"><animate attributeName="stroke-dashoffset" {}/></path>"#,
                    progress(hop, period, "1000", "0")
                )
            };
            format!("{}{}", fill("fill glowing"), fill("fill"))
        }
        SignalStyle::Spark => format!(
            "{}{}{}{}",
            streak(d, length, SIGNAL_TRAIL, "trail glowing", "", hop, period),
            streak(d, length, SIGNAL_TRAIL, "trail", "", hop, period),
            streak(d, length, SIGNAL_BOLT, "bolt", "", hop, period),
            head(&format!(
                r#"<circle class="halo" r="{}">{}</circle><circle class="dot" r="{}"/><circle class="core" r="{}"/>"#,
                num(SIGNAL_HALO),
                flicker(SIGNAL_FLICKER_MS, "0.55;1;0.7;0.95;0.6;1;0.55"),
                num(SIGNAL_DOT),
                num(SIGNAL_CORE)
            ))
        ),
        SignalStyle::Arc => format!(
            r#"<path class="wire-glow" d="{d}">{}</path>{}{}"#,
            flicker(SIGNAL_FLICKER_FAST_MS, "0.25;0.6;0.35;0.7;0.3;0.55;0.25"),
            streak(d, length, SIGNAL_BOLT, "bolt", "", hop, period),
            head(&format!(
                r#"<circle class="halo" r="{}">{}</circle><circle class="dot" r="{}"/><circle class="core" r="{}"/>"#,
                num(SIGNAL_HALO),
                flicker(SIGNAL_FLICKER_FAST_MS, "0.6;1;0.5;0.9;0.7;1;0.6"),
                num(SIGNAL_DOT),
                num(SIGNAL_CORE)
            ))
        ),
        SignalStyle::Comet => {
            // Five layers, longest faintest and thinnest: the tail tapers
            // from the head's width to a fifth of it.
            let tail: String = (1..=5u8)
                .map(|i| {
                    let k = f64::from(i) / 5.0;
                    let px = SIGNAL_COMET * (1.2 - k);
                    let style = format!(
                        r#" style="stroke-width:{};opacity:{}""#,
                        num(SIGNAL_BOLT_WIDTH * k),
                        num(k)
                    );
                    streak(d, length, px, "tail", &style, hop, period)
                })
                .collect();
            format!(
                "{tail}{}",
                head(&format!(r#"<circle class="dot" r="{}"/>"#, num(SIGNAL_DOT)))
            )
        }
        SignalStyle::Dot => head(&format!(
            r#"<circle class="ring" r="{}"/><circle class="dot" r="{}"/>"#,
            num(SIGNAL_RING),
            num(SIGNAL_DOT)
        )),
        SignalStyle::Pulse => {
            let ripple = |begin: u32| {
                let timing = format!(
                    r#"dur="{}ms"{} repeatCount="indefinite""#,
                    ms(crate::tokens::SIGNAL_RIPPLE_PERIOD_MS),
                    if begin == 0 {
                        String::new()
                    } else {
                        format!(r#" begin="{begin}ms""#)
                    }
                );
                format!(
                    r#"<circle class="ripple" r="{dot}"><animate attributeName="r" values="{dot};{}" {timing}/><animate attributeName="opacity" values="0.6;0" {timing}/></circle>"#,
                    num(SIGNAL_RIPPLE),
                    dot = num(SIGNAL_DOT)
                )
            };
            head(&format!(
                r#"{}{}<circle class="dot" r="{}"/><circle class="core" r="{}"/>"#,
                ripple(0),
                ripple(ms(crate::tokens::SIGNAL_RIPPLE_PERIOD_MS / 2.0)),
                num(SIGNAL_DOT),
                num(SIGNAL_CORE)
            ))
        }
        SignalStyle::Current => {
            let cycle: f64 = SIGNAL_DASH.iter().sum();
            format!(
                r#"<path class="flowing" d="{d}"><animate attributeName="stroke-dashoffset" values="0;{}" dur="{}ms" repeatCount="indefinite"/></path>{}"#,
                num(-cycle),
                ms(SIGNAL_DASH_PERIOD_MS),
                head(&format!(r#"<circle class="dot" r="{}"/>"#, num(SIGNAL_DOT)))
            )
        }
    }
}

/// Whether a style draws a glow, so the drawing needs the blur filter.
#[must_use]
pub fn glows(style: SignalStyle) -> bool {
    matches!(
        style,
        SignalStyle::Wire | SignalStyle::Spark | SignalStyle::Arc
    )
}

/// The id a flow's edge carries, for a signal to follow.
#[must_use]
pub fn edge_id(edge: usize) -> String {
    format!("edge-{edge}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions_run_from_0_to_1_in_four_decimals() {
        assert_eq!(fraction(0, 3000), "0");
        assert_eq!(fraction(3000, 3000), "1");
        assert_eq!(fraction(1500, 3000), "0.5");
        assert_eq!(fraction(1, 3000), "0.0003");
        assert_eq!(fraction(2999, 3000), "0.9997");
    }

    #[test]
    fn a_track_writes_one_time_per_value() {
        let mut t = KeyTrack::new(1000);
        t.key(0, "0").key(250, "1").key(1000, "0");
        assert_eq!(
            t.attributes("values"),
            r#"dur="1000ms" repeatCount="indefinite" values="0;1;0" keyTimes="0;0.25;1""#
        );
    }

    #[test]
    #[should_panic(expected = "a track runs the whole cycle")]
    fn a_track_that_stops_short_is_refused() {
        let mut t = KeyTrack::new(1000);
        t.key(0, "0").key(500, "1");
        let _ = t.attributes("values");
    }
}
