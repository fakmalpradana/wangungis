//! Toolbar icons. The data is the approved icon sheet on a 24x24 grid: SVG path
//! strings are kept verbatim so they can be compared 1:1. A small parser flattens
//! them to polylines, which egui strokes and fills.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, vec2};

/// Accent orange `#F27300`, same as the tool preview color.
const ACCENT: Color32 = Color32::from_rgb(242, 115, 0);
/// Guide magenta `#D9008C`.
const GUIDE: Color32 = Color32::from_rgb(217, 0, 140);
/// Stroke width in grid units.
const STROKE: f32 = 1.6;
/// Radius of a `dot`.
const DOT_R: f32 = 1.15;

#[derive(Clone, Copy)]
enum Prim {
    Path(&'static str),
    /// x, y, w, h, rx.
    Rect(f32, f32, f32, f32, f32),
    /// cx, cy, r.
    Circle(f32, f32, f32),
}

#[derive(Clone, Copy, PartialEq)]
enum Look {
    /// Theme-colored stroke.
    Theme,
    /// Accent stroke.
    Accent,
    /// Guide stroke.
    Guide,
    /// Solid accent fill, no stroke.
    Fill,
    /// Accent fill and accent stroke.
    FillStroke,
    /// Accent fill at 35% plus the theme stroke.
    Tint,
    /// Solid fill in the theme color.
    Solid,
}

#[derive(Clone, Copy)]
struct El {
    prim: Prim,
    look: Look,
    /// Dash and gap in grid units; `(0, 0)` = solid line.
    dash: (f32, f32),
    opacity: f32,
}

const fn el(prim: Prim) -> El {
    El {
        prim,
        look: Look::Theme,
        dash: (0.0, 0.0),
        opacity: 1.0,
    }
}
const fn path(d: &'static str) -> El {
    el(Prim::Path(d))
}
const fn rect(x: f32, y: f32, w: f32, h: f32) -> El {
    el(Prim::Rect(x, y, w, h, 0.0))
}
const fn rrect(x: f32, y: f32, w: f32, h: f32, rx: f32) -> El {
    el(Prim::Rect(x, y, w, h, rx))
}
const fn circle(x: f32, y: f32, r: f32) -> El {
    el(Prim::Circle(x, y, r))
}
const fn dot(x: f32, y: f32) -> El {
    circle(x, y, DOT_R).solid()
}
impl El {
    const fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }
    const fn accent(self) -> Self {
        self.look(Look::Accent)
    }
    const fn guide(self) -> Self {
        self.look(Look::Guide)
    }
    const fn fill(self) -> Self {
        self.look(Look::Fill)
    }
    const fn fill_stroke(self) -> Self {
        self.look(Look::FillStroke)
    }
    const fn tint(self) -> Self {
        self.look(Look::Tint)
    }
    const fn solid(self) -> Self {
        self.look(Look::Solid)
    }
    const fn dash(mut self, a: f32, b: f32) -> Self {
        self.dash = (a, b);
        self
    }
    const fn op(mut self, o: f32) -> Self {
        self.opacity = o;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    // Tools, toolbar order.
    Select,
    Line,
    Rectangle,
    Circle,
    Polygon,
    Arc,
    PushPull,
    Offset,
    FollowMe,
    Tape,
    Move,
    Rotate,
    Scale,
    Paint,
    CloudSlice,
    CloudMove,
    Eraser,
    // Actions.
    Undo,
    Redo,
    ClearGuides,
    Group,
    Component,
    Dissolve,
    EditContents,
    Leave,
    ZoomExtents,
    // Views.
    ViewTop,
    ViewFront,
    ViewRight,
    ViewIso,
    ViewBack,
    ViewLeft,
}

impl Icon {
    #[cfg(test)]
    pub const ALL: [Icon; 32] = [
        Icon::Select,
        Icon::Line,
        Icon::Rectangle,
        Icon::Circle,
        Icon::Polygon,
        Icon::Arc,
        Icon::PushPull,
        Icon::Offset,
        Icon::FollowMe,
        Icon::Tape,
        Icon::Move,
        Icon::Rotate,
        Icon::Scale,
        Icon::Paint,
        Icon::CloudSlice,
        Icon::CloudMove,
        Icon::Eraser,
        Icon::Undo,
        Icon::Redo,
        Icon::ClearGuides,
        Icon::Group,
        Icon::Component,
        Icon::Dissolve,
        Icon::EditContents,
        Icon::Leave,
        Icon::ZoomExtents,
        Icon::ViewTop,
        Icon::ViewFront,
        Icon::ViewRight,
        Icon::ViewIso,
        Icon::ViewBack,
        Icon::ViewLeft,
    ];

    /// Icon of a `camera::VIEWS` entry, by its catalog key.
    pub fn view(key: &str) -> Option<Icon> {
        Some(match key {
            "view.top" => Icon::ViewTop,
            "view.front" => Icon::ViewFront,
            "view.right" => Icon::ViewRight,
            "view.iso" => Icon::ViewIso,
            "view.back" => Icon::ViewBack,
            "view.left" => Icon::ViewLeft,
            _ => return None,
        })
    }

    fn elements(self) -> &'static [El] {
        match self {
            Icon::Select => const { &[path("M6 3v16l4.2-4.2 2.8 6.2 2.4-1.1-2.8-6.1H18Z")] },
            Icon::Line => {
                const {
                    &[
                        path("M5 19 19 5"),
                        circle(5.0, 19.0, 2.0).fill(),
                        circle(19.0, 5.0, 2.0).fill(),
                    ]
                }
            }
            Icon::Rectangle => {
                const {
                    &[
                        rect(4.0, 6.0, 16.0, 12.0),
                        circle(4.0, 18.0, 2.0).fill(),
                        circle(20.0, 6.0, 2.0).fill(),
                    ]
                }
            }
            Icon::Circle => {
                const {
                    &[
                        circle(12.0, 12.0, 8.0),
                        path("M12 12h8").accent(),
                        circle(12.0, 12.0, 1.8).fill(),
                    ]
                }
            }
            Icon::Polygon => {
                const {
                    &[
                        path("M20 12 16 18.9H8L4 12l4-6.9h8Z"),
                        circle(12.0, 12.0, 1.8).fill(),
                    ]
                }
            }
            Icon::Arc => {
                const {
                    &[
                        path("M4 17a8 8 0 0 1 16 0"),
                        path("M4 17h16").dash(2.0, 2.5).op(0.45),
                        circle(4.0, 17.0, 2.0).fill(),
                        circle(20.0, 17.0, 2.0).fill(),
                    ]
                }
            }
            Icon::PushPull => {
                const {
                    &[
                        path("M4 14h10v8H4z"),
                        path("M14 14l4-4v8l-4 4"),
                        path("M4 14l4-4h10l-4 4Z").tint(),
                        path("M11 12V2.5M8 5.5l3-3 3 3").accent(),
                    ]
                }
            }
            Icon::Offset => {
                const {
                    &[
                        rrect(3.0, 3.0, 18.0, 18.0, 1.0),
                        rect(7.5, 7.5, 9.0, 9.0).accent(),
                    ]
                }
            }
            Icon::FollowMe => {
                const {
                    &[
                        path("M6 18C6 10 11 6 20 6M17 3.5 20 6l-3 2.5").accent(),
                        rect(3.0, 15.0, 6.0, 6.0).tint(),
                    ]
                }
            }
            Icon::Tape => {
                const {
                    &[
                        path("M4 7v10M20 7v10"),
                        path("M5 12h14M8 9.5 5.5 12 8 14.5M16 9.5 18.5 12 16 14.5").accent(),
                    ]
                }
            }
            Icon::Move => {
                const {
                    &[
                    path("M12 3v18M3 12h18"),
                    path("M9.5 5.5 12 3l2.5 2.5M9.5 18.5 12 21l2.5-2.5M5.5 9.5 3 12l2.5 2.5M18.5 9.5 21 12l-2.5 2.5").accent(),
                ]
                }
            }
            Icon::Rotate => {
                const {
                    &[
                        path("M12 12h7"),
                        circle(12.0, 12.0, 1.8).solid(),
                        path("M19 12a7 7 0 1 1-2.05-4.95M17.2 3.2 17 7.2l-4 .2").accent(),
                    ]
                }
            }
            Icon::Scale => {
                const {
                    &[
                        rect(3.0, 3.0, 18.0, 18.0).dash(2.5, 2.5).op(0.5),
                        rect(3.0, 13.0, 8.0, 8.0),
                        path("M11 13 19 5M14 5h5v5").accent(),
                    ]
                }
            }
            Icon::Paint => {
                const {
                    &[
                        path("M3 15l6-5 7 5-6 5Z").tint(),
                        path("M18 3s-3.5 4-3.5 6.5a3.5 3.5 0 0 0 7 0C21.5 7 18 3 18 3Z")
                            .fill_stroke(),
                    ]
                }
            }
            Icon::CloudSlice => {
                const {
                    &[
                        dot(3.0, 4.0).op(0.35),
                        dot(21.0, 8.0).op(0.35),
                        dot(3.0, 19.0).op(0.35),
                        dot(20.0, 20.0).op(0.35),
                        dot(12.0, 2.5).op(0.35),
                        dot(21.5, 15.0).op(0.35),
                        dot(9.0, 10.0),
                        dot(13.0, 9.0),
                        dot(15.0, 13.0),
                        dot(10.0, 14.5),
                        dot(12.0, 12.0),
                        dot(14.5, 16.0),
                        dot(8.5, 16.5),
                        rect(6.0, 6.0, 12.0, 12.0).accent().dash(2.5, 2.0),
                        rect(10.8, 4.8, 2.4, 2.4).fill(),
                    ]
                }
            }
            Icon::CloudMove => {
                const {
                    &[
                        dot(4.0, 9.0),
                        dot(7.0, 7.0),
                        dot(6.0, 12.0),
                        dot(9.0, 10.0),
                        dot(3.5, 15.0),
                        dot(8.0, 15.0),
                        dot(10.0, 12.5),
                        dot(10.5, 17.0),
                        dot(6.0, 18.5),
                        path("M13 12h8M18 9l3 3-3 3").accent(),
                    ]
                }
            }
            Icon::Eraser => {
                const {
                    &[
                        path("M3 15 12 6l6 6-9 9Z"),
                        path("M3 15l4.5-4.5 6 6L9 21Z").tint(),
                        path("M9 21h12"),
                    ]
                }
            }
            Icon::Undo => const { &[path("M9 5 4 10l5 5"), path("M4 10h11a5 5 0 0 1 0 10h-4")] },
            Icon::Redo => const { &[path("m15 5 5 5-5 5"), path("M20 10H9a5 5 0 0 0 0 10h4")] },
            Icon::ClearGuides => {
                const {
                    &[
                        path("M3 21 21 3").guide().dash(3.0, 2.5),
                        path("M15 15l5 5m0-5-5 5"),
                    ]
                }
            }
            Icon::Group => {
                const {
                    &[
                        rrect(2.5, 2.5, 19.0, 19.0, 1.0).dash(3.0, 2.5).accent(),
                        rect(6.0, 11.0, 6.0, 7.0),
                        circle(16.0, 9.0, 3.0),
                    ]
                }
            }
            Icon::Component => {
                const {
                    &[
                        rect(3.0, 8.0, 7.0, 8.0),
                        rect(14.0, 8.0, 7.0, 8.0),
                        path("M10 12h4").accent().dash(1.5, 1.5),
                        path("M3 5.5 4.5 4 6 5.5 4.5 7ZM14 5.5 15.5 4 17 5.5 15.5 7Z").fill(),
                    ]
                }
            }
            Icon::Dissolve => {
                const {
                    &[
                        rect(7.0, 4.0, 10.0, 16.0).dash(3.0, 2.5).op(0.45),
                        rect(2.0, 9.0, 5.0, 6.0),
                        circle(19.5, 12.0, 2.5),
                        path("M11 12H8.5M13 12h2.5").accent(),
                    ]
                }
            }
            Icon::EditContents => {
                const {
                    &[
                        rrect(8.0, 3.0, 13.0, 18.0, 1.0).dash(3.0, 2.5),
                        path("M2 12h12M11 9l3 3-3 3").accent(),
                    ]
                }
            }
            Icon::Leave => {
                const {
                    &[
                        rrect(3.0, 3.0, 13.0, 18.0, 1.0).dash(3.0, 2.5),
                        path("M9 12h12M18 9l3 3-3 3").accent(),
                    ]
                }
            }
            Icon::ZoomExtents => {
                const {
                    &[
                        path("M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5"),
                        rect(8.0, 8.0, 8.0, 8.0).accent(),
                    ]
                }
            }
            Icon::ViewTop => {
                const {
                    &[
                        path("M5 9h10v10H5z"),
                        path("M5 9l4-4h10l-4 4Z").tint(),
                        path("M15 9l4-4v10l-4 4Z"),
                    ]
                }
            }
            Icon::ViewFront => {
                const {
                    &[
                        path("M5 9h10v10H5z").tint(),
                        path("M5 9l4-4h10l-4 4Z"),
                        path("M15 9l4-4v10l-4 4Z"),
                    ]
                }
            }
            Icon::ViewRight => {
                const {
                    &[
                        path("M5 9h10v10H5z"),
                        path("M5 9l4-4h10l-4 4Z"),
                        path("M15 9l4-4v10l-4 4Z").tint(),
                    ]
                }
            }
            Icon::ViewBack => {
                const {
                    &[
                        path("M9 5h10v10H9z").tint().dash(2.0, 2.0),
                        path("M5 9h10v10H5z"),
                        path("M5 9l4-4h10l-4 4Z"),
                        path("M15 9l4-4v10l-4 4Z"),
                    ]
                }
            }
            Icon::ViewLeft => {
                const {
                    &[
                        path("M5 9l4-4v10l-4 4Z").tint().dash(2.0, 2.0),
                        path("M5 9h10v10H5z"),
                        path("M5 9l4-4h10l-4 4Z"),
                        path("M15 9l4-4v10l-4 4Z"),
                    ]
                }
            }
            Icon::ViewIso => {
                const {
                    &[
                        path("M5 9h10v10H5z"),
                        path("M5 9l4-4h10l-4 4Z"),
                        path("M15 9l4-4v10l-4 4Z"),
                        path("M5 9l4-4h10v10l-4 4H5Z").accent(),
                    ]
                }
            }
        }
    }
}

/// A flattened subpath in grid units.
#[derive(Debug, PartialEq)]
struct Sub {
    pts: Vec<(f32, f32)>,
    closed: bool,
}

/// Parser state for the path commands the icon sheet uses:
/// `M m L l H h V v C c S s A a Z z`.
#[derive(Default)]
struct Builder {
    subs: Vec<Sub>,
    cur: (f32, f32),
    start: (f32, f32),
    /// Second control point of the last cubic, for `S`/`s`.
    ctrl: Option<(f32, f32)>,
}

impl Builder {
    /// The subpath being drawn; a new one starts at `cur` after `Z`.
    fn sub(&mut self) -> &mut Sub {
        if self.subs.last().is_none_or(|s| s.closed) {
            self.subs.push(Sub {
                pts: vec![self.cur],
                closed: false,
            });
        }
        self.subs.last_mut().expect("just pushed")
    }

    fn move_to(&mut self, p: (f32, f32)) {
        self.cur = p;
        self.start = p;
        self.subs.push(Sub {
            pts: vec![p],
            closed: false,
        });
    }

    fn line_to(&mut self, p: (f32, f32)) {
        self.sub().pts.push(p);
        self.cur = p;
    }

    fn cubic(&mut self, c1: (f32, f32), c2: (f32, f32), p: (f32, f32)) {
        let p0 = self.cur;
        const N: usize = 16;
        for i in 1..=N {
            let t = i as f32 / N as f32;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.sub().pts.push((
                a * p0.0 + b * c1.0 + c * c2.0 + d * p.0,
                a * p0.1 + b * c1.1 + c * c2.1 + d * p.1,
            ));
        }
        self.cur = p;
        self.ctrl = Some(c2);
    }

    /// Elliptical arc by endpoint parameterization, SVG spec F.6.5.
    fn arc(&mut self, rx: f32, ry: f32, phi_deg: f32, large: bool, sweep: bool, p: (f32, f32)) {
        let (x1, y1) = self.cur;
        let (x2, y2) = p;
        let (mut rx, mut ry) = (rx.abs(), ry.abs());
        if (x1 == x2 && y1 == y2) || rx == 0.0 || ry == 0.0 {
            self.line_to(p);
            return;
        }
        let phi = phi_deg.to_radians();
        let (sin, cos) = phi.sin_cos();
        let (dx, dy) = ((x1 - x2) / 2.0, (y1 - y2) / 2.0);
        let (x1p, y1p) = (cos * dx + sin * dy, -sin * dx + cos * dy);
        let lambda = (x1p / rx).powi(2) + (y1p / ry).powi(2);
        if lambda > 1.0 {
            let s = lambda.sqrt();
            rx *= s;
            ry *= s;
        }
        let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
        let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
        let sign = if large == sweep { -1.0 } else { 1.0 };
        let coef = sign * (num / den).max(0.0).sqrt();
        let (cxp, cyp) = (coef * rx * y1p / ry, -coef * ry * x1p / rx);
        let cx = cos * cxp - sin * cyp + (x1 + x2) / 2.0;
        let cy = sin * cxp + cos * cyp + (y1 + y2) / 2.0;
        let th1 = ((y1p - cyp) / ry).atan2((x1p - cxp) / rx);
        let th2 = ((-y1p - cyp) / ry).atan2((-x1p - cxp) / rx);
        let mut dth = th2 - th1;
        if !sweep && dth > 0.0 {
            dth -= std::f32::consts::TAU;
        } else if sweep && dth < 0.0 {
            dth += std::f32::consts::TAU;
        }
        // 48 segments per full turn; the epsilon keeps a half turn at exactly 24.
        let n = ((dth.abs() * 48.0 / std::f32::consts::TAU - 1e-3).ceil() as usize).max(1);
        for i in 1..=n {
            let th = th1 + dth * i as f32 / n as f32;
            let (s, c) = th.sin_cos();
            self.sub().pts.push((
                cx + cos * rx * c - sin * ry * s,
                cy + sin * rx * c + cos * ry * s,
            ));
        }
        // End exactly on the requested point.
        *self.sub().pts.last_mut().expect("arc has points") = p;
        self.cur = p;
    }

    fn close(&mut self) {
        if let Some(s) = self.subs.last_mut() {
            s.closed = true;
        }
        self.cur = self.start;
    }
}

/// Parses an SVG path `d` string.
fn parse(d: &str) -> Vec<Sub> {
    let b = d.as_bytes();
    let mut i = 0;
    // A number: optional sign, digits, optional fraction. Stops at the next sign.
    let num = |i: &mut usize| -> f32 {
        while *i < b.len() && matches!(b[*i], b' ' | b',') {
            *i += 1;
        }
        let from = *i;
        if *i < b.len() && matches!(b[*i], b'+' | b'-') {
            *i += 1;
        }
        while *i < b.len() && (b[*i].is_ascii_digit() || b[*i] == b'.') {
            if b[*i] == b'.' && d[from..*i].contains('.') {
                break;
            }
            *i += 1;
        }
        d[from..*i].parse().expect("icon path number")
    };
    let mut bld = Builder::default();
    let mut cmd = b' ';
    loop {
        while i < b.len() && matches!(b[i], b' ' | b',') {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        if b[i].is_ascii_alphabetic() {
            cmd = b[i];
            i += 1;
            if cmd == b'Z' || cmd == b'z' {
                bld.close();
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { bld.cur } else { (0.0, 0.0) };
        let prev_ctrl = bld.ctrl.take();
        match cmd.to_ascii_uppercase() {
            b'M' => {
                bld.move_to((ox + num(&mut i), oy + num(&mut i)));
                // Further pairs are implicit lineto.
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => bld.line_to((ox + num(&mut i), oy + num(&mut i))),
            b'H' => bld.line_to((ox + num(&mut i), bld.cur.1)),
            b'V' => bld.line_to((bld.cur.0, oy + num(&mut i))),
            b'C' => {
                let c1 = (ox + num(&mut i), oy + num(&mut i));
                let c2 = (ox + num(&mut i), oy + num(&mut i));
                bld.cubic(c1, c2, (ox + num(&mut i), oy + num(&mut i)));
            }
            b'S' => {
                let (x, y) = bld.cur;
                let c1 = prev_ctrl.map_or((x, y), |c| (2.0 * x - c.0, 2.0 * y - c.1));
                let c2 = (ox + num(&mut i), oy + num(&mut i));
                bld.cubic(c1, c2, (ox + num(&mut i), oy + num(&mut i)));
            }
            b'A' => {
                let (rx, ry, phi) = (num(&mut i), num(&mut i), num(&mut i));
                let (large, sweep) = (num(&mut i) != 0.0, num(&mut i) != 0.0);
                bld.arc(
                    rx,
                    ry,
                    phi,
                    large,
                    sweep,
                    (ox + num(&mut i), oy + num(&mut i)),
                );
            }
            other => panic!("icon path command {}", other as char),
        }
    }
    bld.subs
}

/// Closed polygon of a circle, 48 segments.
fn circle_pts(cx: f32, cy: f32, r: f32) -> Vec<(f32, f32)> {
    (0..48)
        .map(|k| {
            let (s, c) = (k as f32 * std::f32::consts::TAU / 48.0).sin_cos();
            (cx + r * c, cy + r * s)
        })
        .collect()
}

fn subpaths(prim: Prim) -> Vec<Sub> {
    match prim {
        Prim::Path(d) => parse(d),
        // ponytail: the sheet's rx is at most 1 unit (under 1.5), so a plain rect is
        // visually the same at toolbar size. No rounded corners.
        Prim::Rect(x, y, w, h, _rx) => vec![Sub {
            pts: vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
            closed: true,
        }],
        Prim::Circle(cx, cy, r) => vec![Sub {
            pts: circle_pts(cx, cy, r),
            closed: true,
        }],
    }
}

/// The shapes of `icon` inside `rect`; `fg` is the theme stroke color.
fn shapes(rect: Rect, icon: Icon, fg: Color32) -> Vec<Shape> {
    let scale = rect.width().min(rect.height()) / 24.0;
    let origin = rect.center() - vec2(12.0, 12.0) * scale;
    let map = |p: (f32, f32)| -> Pos2 { origin + vec2(p.0, p.1) * scale };
    let mut out = Vec::new();
    for e in icon.elements() {
        let fade = |c: Color32| c.gamma_multiply(e.opacity);
        let stroke = |c: Color32| Stroke::new(STROKE * scale, fade(c));
        for sub in subpaths(e.prim) {
            let mut pts: Vec<Pos2> = sub.pts.iter().map(|&p| map(p)).collect();
            match e.look {
                Look::Fill => out.push(Shape::convex_polygon(pts, fade(ACCENT), Stroke::NONE)),
                Look::FillStroke => {
                    out.push(Shape::convex_polygon(pts, fade(ACCENT), stroke(ACCENT)))
                }
                Look::Tint => {
                    let fill = fade(ACCENT.gamma_multiply(0.35));
                    if e.dash.0 > 0.0 {
                        out.push(Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
                        pts.push(pts[0]);
                        out.extend(Shape::dashed_line(
                            &pts,
                            stroke(fg),
                            e.dash.0 * scale,
                            e.dash.1 * scale,
                        ));
                    } else {
                        out.push(Shape::convex_polygon(pts, fill, stroke(fg)));
                    }
                }
                Look::Solid => out.push(Shape::convex_polygon(pts, fade(fg), Stroke::NONE)),
                Look::Theme | Look::Accent | Look::Guide => {
                    let color = match e.look {
                        Look::Accent => ACCENT,
                        Look::Guide => GUIDE,
                        _ => fg,
                    };
                    if sub.closed {
                        pts.push(pts[0]);
                    }
                    if e.dash.0 > 0.0 {
                        out.extend(Shape::dashed_line(
                            &pts,
                            stroke(color),
                            e.dash.0 * scale,
                            e.dash.1 * scale,
                        ));
                    } else {
                        if !sub.closed {
                            // Round caps: a disc at each end of an open line.
                            for end in [pts[0], pts[pts.len() - 1]] {
                                out.push(Shape::circle_filled(
                                    end,
                                    STROKE * scale / 2.0,
                                    fade(color),
                                ));
                            }
                        }
                        out.push(Shape::line(pts, stroke(color)));
                    }
                }
            }
        }
    }
    out
}

/// Paints `icon` into `rect` (the 24x24 grid is fitted to its smaller side).
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, fg: Color32) {
    painter.extend(shapes(rect, icon, fg));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn absolute_and_implicit_lineto() {
        let s = parse("M5 19 19 5");
        assert_eq!(
            s,
            vec![Sub {
                pts: vec![(5.0, 19.0), (19.0, 5.0)],
                closed: false
            }]
        );
    }

    #[test]
    fn relative_h_v_and_close() {
        // The Select arrow: relative v, l, H absolute, Z.
        let s = parse("M6 3v16l4.2-4.2 2.8 6.2H18Z");
        assert_eq!(s.len(), 1);
        assert!(s[0].closed);
        let want = [
            (6.0, 3.0),
            (6.0, 19.0),
            (10.2, 14.8),
            (13.0, 21.0),
            (18.0, 21.0),
        ];
        assert_eq!(s[0].pts.len(), want.len());
        for (p, w) in s[0].pts.iter().zip(want) {
            assert!(close(*p, w), "{p:?} vs {w:?}");
        }
    }

    #[test]
    fn numbers_with_leading_dot_and_joined_signs() {
        // `.2` has no leading zero; `4.2-4.2` is two numbers; the third pair is an implicit l.
        let s = parse("M0 0l.2 4.2-4.2 1");
        assert!(close(s[0].pts[1], (0.2, 4.2)));
        assert!(close(s[0].pts[2], (-4.0, 5.2)));
    }

    #[test]
    fn relative_moveto_after_close_and_second_subpath() {
        let s = parse("M15 15l5 5m0-5-5 5");
        assert_eq!(s.len(), 2);
        assert!(close(s[0].pts[1], (20.0, 20.0)));
        assert!(close(s[1].pts[0], (20.0, 15.0)));
        assert!(close(s[1].pts[1], (15.0, 20.0)));
    }

    #[test]
    fn close_then_draw_starts_new_subpath_at_start() {
        let s = parse("M0 0h4v4ZM10 10h1");
        assert_eq!(s.len(), 2);
        assert!(s[0].closed && !s[1].closed);
    }

    #[test]
    fn half_circle_arc() {
        let s = parse("M4 17a8 8 0 0 1 16 0");
        let pts = &s[0].pts;
        assert!(close(pts[0], (4.0, 17.0)));
        assert!(close(*pts.last().unwrap(), (20.0, 17.0)));
        // Center (12, 17), radius 8, sweeping through the top (12, 9).
        assert!(pts.iter().any(|&p| close(p, (12.0, 9.0))), "{pts:?}");
        for &(x, y) in pts {
            let r = ((x - 12.0).powi(2) + (y - 17.0).powi(2)).sqrt();
            assert!((r - 8.0).abs() < 1e-3, "radius {r}");
            assert!(y <= 17.0 + 1e-3);
        }
    }

    #[test]
    fn large_arc_flag_takes_the_long_way() {
        // Rotate icon arc: starts (19,12), ends near (17,7.05), large arc, sweep.
        let s = parse("M19 12a7 7 0 1 1-2.05-4.95");
        let pts = &s[0].pts;
        assert!(close(*pts.last().unwrap(), (16.95, 7.05)));
        // Long way round the center (12,12): passes the bottom (12,19).
        assert!(pts.iter().any(|&p| close(p, (12.0, 19.0))), "{pts:?}");
    }

    #[test]
    fn cubic_ends_on_target_and_smooth_continues() {
        let s = parse("M0 0C0 4 4 8 8 8s4-4 0-8");
        let pts = &s[0].pts;
        assert!(close(pts[16], (8.0, 8.0)));
        assert!(close(*pts.last().unwrap(), (8.0, 0.0)));
    }

    #[test]
    fn every_icon_parses_and_stays_inside_its_rect() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), vec2(48.0, 48.0));
        let margin = STROKE * 2.0;
        let bounds = rect.expand(margin);
        for icon in Icon::ALL {
            let shapes = shapes(rect, icon, Color32::WHITE);
            assert!(!shapes.is_empty(), "{icon:?} paints nothing");
            let mut n = 0;
            for shape in &shapes {
                let pts: Vec<Pos2> = match shape {
                    Shape::Path(p) => p.points.clone(),
                    Shape::LineSegment { points, .. } => points.to_vec(),
                    Shape::Circle(c) => vec![
                        c.center - vec2(c.radius, c.radius),
                        c.center + vec2(c.radius, c.radius),
                    ],
                    other => panic!("{icon:?}: unexpected shape {other:?}"),
                };
                for p in pts {
                    n += 1;
                    assert!(bounds.contains(p), "{icon:?}: {p:?} outside {bounds:?}");
                }
            }
            assert!(n > 1, "{icon:?} has no points");
        }
    }

    #[test]
    fn view_labels_map_to_icons() {
        assert_eq!(Icon::view("view.top"), Some(Icon::ViewTop));
        assert_eq!(Icon::view("view.iso"), Some(Icon::ViewIso));
        assert_eq!(Icon::view("view.back"), Some(Icon::ViewBack));
        assert_eq!(Icon::view("view.left"), Some(Icon::ViewLeft));
        for (label, ..) in crate::camera::VIEWS {
            assert!(Icon::view(label).is_some(), "{label} has no icon");
        }
    }
}
