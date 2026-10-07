//! Where the notch sits on screen, see docs/fonctionnel/DF-0006-position-du-notch.md.
//! Pure geometry in physical pixels: the OS layer supplies the monitor work
//! area and applies the result.
//!
//! The notch window has a fixed size (the largest expanded shape). It is
//! glued to one edge of the work area; the visible shape is glued to the same
//! side of the window and opens towards the inside of the screen.

use serde::{Deserialize, Serialize};

/// The screen edge the notch is attached to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    /// Left and right edges get a thin vertical notch.
    pub fn is_vertical(self) -> bool {
        matches!(self, Edge::Left | Edge::Right)
    }
}

/// Look of the notch: glued to the edge, or a pill detached from it
/// ("dynamic island", ADR-0003).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Style {
    #[default]
    Notch,
    Pill,
}

/// Distance range between a pill and its screen edge, in logical pixels.
pub const GAP_RANGE: std::ops::RangeInclusive<u32> = 4..=40;

/// Saved in config.json. `offset` is the position of the notch center along
/// the edge, from 0 (left / top end) to 1 (right / bottom end); 0.5 is
/// centered. Dragging the
/// notch along its edge only changes `offset` (step 3).
/// `screen`: id of the chosen monitor, `None` = whichever is primary.
/// `gap`: distance between a pill and the edge, logical pixels (unused by
/// the notch style).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Placement {
    pub edge: Edge,
    pub offset: f64,
    pub screen: Option<String>,
    pub style: Style,
    pub gap: u32,
}

impl Placement {
    /// Distance between the window and its edge, in physical pixels.
    pub fn gap_px(&self, scale: f64) -> i32 {
        match self.style {
            Style::Notch => 0,
            Style::Pill => (f64::from(self.gap) * scale).round() as i32,
        }
    }
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            edge: Edge::Top,
            offset: 0.5,
            screen: None,
            style: Style::Notch,
            gap: 10,
        }
    }
}

/// A monitor as seen by the notch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    /// OS name of the monitor (e.g. `\.DISPLAY1` on Windows).
    pub id: String,
    /// Whole monitor, for ordering and labels.
    pub bounds: Area,
    /// Monitor minus the taskbar: where the notch is glued.
    pub work_area: Area,
    pub primary: bool,
}

/// The screen the notch goes on: the chosen one if it is connected,
/// otherwise the primary one (so unplugging a monitor never loses the notch;
/// it comes back when the monitor is plugged in again).
pub fn choose_screen<'a>(screens: &'a [Screen], wanted: Option<&str>) -> Option<&'a Screen> {
    wanted
        .and_then(|id| screens.iter().find(|s| s.id == id))
        .or_else(|| screens.iter().find(|s| s.primary))
        .or_else(|| screens.first())
}

/// Human labels for the settings, numbered from left to right like the
/// Windows display settings usually are: "Screen 1 · 1920 × 1080 · primary".
pub fn screen_labels(screens: &[Screen]) -> Vec<(String, String)> {
    let mut ordered: Vec<&Screen> = screens.iter().collect();
    ordered.sort_by_key(|s| (s.bounds.x, s.bounds.y));
    ordered
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let label = crate::t!(
                "screen.label",
                number = i + 1,
                width = s.bounds.width,
                height = s.bounds.height
            );
            let label = if s.primary {
                crate::t!("screen.primary", screen = label)
            } else {
                label
            };
            (s.id.clone(), label)
        })
        .collect()
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Where the notch window goes and where the notch sits inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Top-left corner of the window on screen.
    pub window: (i32, i32),
    /// Center of the compact notch along the edge, from the start (left /
    /// top) of the window. Every shape, compact or expanded, is centered on
    /// it, then pushed back inside the window: at an end of the edge, the
    /// notch therefore opens towards the other end.
    pub anchor: i32,
}

/// Start and length of the edge along which the notch moves.
fn edge_span(edge: Edge, work_area: Area) -> (i32, i32) {
    if edge.is_vertical() {
        (work_area.y, work_area.height)
    } else {
        (work_area.x, work_area.width)
    }
}

/// Size of the notch window: the whole length of its edge, so the notch, its
/// cards and any number of pins fit anywhere along it (DF-0012), and `depth`
/// towards the inside of the screen. Physical pixels.
pub fn window_size(edge: Edge, work_area: Area, depth: i32) -> (i32, i32) {
    if edge.is_vertical() {
        (depth, work_area.height)
    } else {
        (work_area.width, depth)
    }
}

/// Center of the compact notch along the edge (`compact` long), kept so the
/// whole compact notch is on screen: at the ends it touches the corner.
fn notch_center(offset: f64, begin: i32, length: i32, compact: i32) -> i32 {
    let wanted = begin + (offset.clamp(0.0, 1.0) * f64::from(length)).round() as i32;
    let lowest = begin + compact / 2;
    let highest = (begin + length - (compact - compact / 2)).max(lowest);
    wanted.clamp(lowest, highest)
}

/// Places the notch window, glued to `placement.edge` of the monitor work
/// area (so a bottom notch sits just above the taskbar) and kept fully inside
/// it; a pill sits `gap` physical pixels away from the edge. `compact` is the
/// length of the compact notch along the edge.
pub fn layout(
    placement: &Placement,
    work_area: Area,
    window: (i32, i32),
    compact: i32,
    gap: i32,
) -> Layout {
    let (width, height) = window;
    let (begin, length) = edge_span(placement.edge, work_area);
    let size = if placement.edge.is_vertical() {
        height
    } else {
        width
    };
    let center = notch_center(placement.offset, begin, length, compact);
    let start = (center - size / 2).clamp(begin, (begin + length - size).max(begin));
    let wa = work_area;
    let window = match placement.edge {
        Edge::Top => (start, wa.y + gap),
        Edge::Bottom => (start, wa.y + wa.height - height - gap),
        Edge::Left => (wa.x + gap, start),
        Edge::Right => (wa.x + wa.width - width - gap, start),
    };
    Layout {
        window,
        anchor: center - start,
    }
}

/// Offset after dragging the notch by `delta` physical pixels along its edge
/// (DF-0006, step 3), starting from `start`. Clamped to the reachable range,
/// so dragging past an end and back moves the notch at once. Within `snap`
/// pixels of a quarter, the middle or three quarters of the edge, it snaps
/// there.
pub fn dragged_offset(
    start: &Placement,
    work_area: Area,
    compact: i32,
    delta: i32,
    snap: i32,
) -> f64 {
    let (begin, length) = edge_span(start.edge, work_area);
    if length <= 0 {
        return 0.5;
    }
    let from = notch_center(start.offset, begin, length, compact);
    let center = notch_center(
        f64::from(from + delta - begin) / f64::from(length),
        begin,
        length,
        compact,
    );
    for fraction in [0.25, 0.5, 0.75] {
        let mark = begin + (fraction * f64::from(length)).round() as i32;
        if (center - mark).abs() <= snap {
            return fraction;
        }
    }
    f64::from(center - begin) / f64::from(length)
}

/// Top-left corner of a visible shape inside the window: against the
/// attached side, centered on `anchor` along it, kept inside the window.
pub fn shape_in_window(
    edge: Edge,
    window: (i32, i32),
    shape: (i32, i32),
    anchor: i32,
) -> (i32, i32) {
    let (ww, wh) = window;
    let (sw, sh) = shape;
    let along = |size: i32, length: i32| (anchor - size / 2).clamp(0, (length - size).max(0));
    match edge {
        Edge::Top => (along(sw, ww), 0),
        Edge::Bottom => (along(sw, ww), wh - sh),
        Edge::Left => (0, along(sh, wh)),
        Edge::Right => (ww - sw, along(sh, wh)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2560 x 1440 screen with a 48 px taskbar at the bottom.
    const WORK_AREA: Area = Area {
        x: 0,
        y: 0,
        width: 2560,
        height: 1392,
    };
    const WINDOW: (i32, i32) = (380, 174);

    /// Length of the compact notch along each edge.
    fn compact(edge: Edge) -> i32 {
        if edge.is_vertical() { 120 } else { 300 }
    }

    fn placed(edge: Edge, offset: f64) -> Layout {
        let placement = Placement {
            edge,
            offset,
            ..Placement::default()
        };
        layout(&placement, WORK_AREA, WINDOW, compact(edge), 0)
    }

    fn at(edge: Edge, offset: f64) -> (i32, i32) {
        placed(edge, offset).window
    }

    #[test]
    fn centered_on_each_edge() {
        assert_eq!(at(Edge::Top, 0.5), (1090, 0));
        assert_eq!(at(Edge::Bottom, 0.5), (1090, 1392 - 174));
        assert_eq!(at(Edge::Left, 0.5), (0, 696 - 87));
        assert_eq!(at(Edge::Right, 0.5), (2560 - 380, 696 - 87));
        assert_eq!(placed(Edge::Top, 0.5).anchor, 190);
        assert_eq!(placed(Edge::Left, 0.5).anchor, 87);
    }

    #[test]
    fn reaches_the_corners_and_opens_towards_the_other_end() {
        let start = placed(Edge::Top, 0.0);
        assert_eq!(start.window, (0, 0));
        // Compact notch in the corner, expanded one too: it grows rightwards.
        assert_eq!(
            shape_in_window(Edge::Top, WINDOW, (300, 36), start.anchor),
            (0, 0)
        );
        assert_eq!(
            shape_in_window(Edge::Top, WINDOW, (380, 96), start.anchor),
            (0, 0)
        );

        let end = placed(Edge::Top, 1.0);
        assert_eq!(end.window, (2560 - 380, 0));
        assert_eq!(
            shape_in_window(Edge::Top, WINDOW, (300, 36), end.anchor),
            (80, 0)
        );

        let bottom = placed(Edge::Right, 7.0);
        assert_eq!(bottom.window, (2560 - 380, 1392 - 174));
        assert_eq!(
            shape_in_window(Edge::Right, WINDOW, (36, 120), bottom.anchor),
            (344, 54)
        );
        assert_eq!(
            shape_in_window(Edge::Right, WINDOW, (380, 174), bottom.anchor),
            (0, 0)
        );
    }

    #[test]
    fn follows_a_monitor_that_is_not_at_the_origin() {
        let second = Area {
            x: -1920,
            y: 200,
            width: 1920,
            height: 1040,
        };
        let placement = Placement {
            edge: Edge::Bottom,
            ..Placement::default()
        };
        assert_eq!(
            layout(&placement, second, WINDOW, 300, 0).window,
            (-1920 + 960 - 190, 200 + 1040 - 174)
        );
    }

    #[test]
    fn shape_is_glued_to_the_attached_side() {
        assert_eq!(shape_in_window(Edge::Top, WINDOW, (300, 36), 190), (40, 0));
        assert_eq!(
            shape_in_window(Edge::Bottom, WINDOW, (300, 36), 190),
            (40, 138)
        );
        assert_eq!(shape_in_window(Edge::Left, WINDOW, (36, 120), 87), (0, 27));
        assert_eq!(
            shape_in_window(Edge::Right, WINDOW, (36, 120), 87),
            (344, 27)
        );
    }

    fn dragged(edge: Edge, offset: f64, delta: i32) -> f64 {
        let start = Placement {
            edge,
            offset,
            ..Placement::default()
        };
        dragged_offset(&start, WORK_AREA, compact(edge), delta, 10)
    }

    #[test]
    fn a_pill_stands_off_its_edge() {
        let pill = |edge| Placement {
            edge,
            style: Style::Pill,
            ..Placement::default()
        };
        let window = |edge| layout(&pill(edge), WORK_AREA, WINDOW, compact(edge), 15).window;
        assert_eq!(window(Edge::Top), (1090, 15));
        assert_eq!(window(Edge::Bottom), (1090, 1392 - 174 - 15));
        assert_eq!(window(Edge::Left), (15, 696 - 87));
        assert_eq!(window(Edge::Right), (2560 - 380 - 15, 696 - 87));
        assert_eq!(pill(Edge::Top).gap_px(1.5), 15);
        assert_eq!(Placement::default().gap_px(1.5), 0);
    }

    #[test]
    fn drag_moves_the_notch_by_the_cursor_delta() {
        let offset = dragged(Edge::Top, 0.5, -500);
        assert_eq!(at(Edge::Top, offset), (1090 - 500, 0));
        let offset = dragged(Edge::Left, 0.5, 300);
        assert_eq!(at(Edge::Left, offset), (0, 696 - 87 + 300));
    }

    #[test]
    fn drag_snaps_to_the_quarters_and_the_center() {
        assert_eq!(dragged(Edge::Top, 0.5, 8), 0.5);
        assert_eq!(dragged(Edge::Bottom, 0.3, 512 - 6), 0.5);
        assert_eq!(dragged(Edge::Top, 0.5, -640 + 9), 0.25);
        assert_eq!(dragged(Edge::Left, 0.5, 348 - 4), 0.75);
        assert_ne!(dragged(Edge::Top, 0.5, 30), 0.5);
    }

    #[test]
    fn drag_stops_at_the_ends_of_the_edge() {
        let offset = dragged(Edge::Top, 0.5, -5000);
        let start = placed(Edge::Top, offset);
        assert_eq!(start.window, (0, 0));
        assert_eq!(start.anchor, 150);
        // Coming back from beyond the end moves the notch immediately.
        let back = Placement {
            edge: Edge::Top,
            offset,
            ..Placement::default()
        };
        let offset = dragged_offset(&back, WORK_AREA, 300, 100, 10);
        assert_eq!(at(Edge::Top, offset), (250 - 190, 0));
        let offset = dragged(Edge::Right, 0.5, 5000);
        assert_eq!(at(Edge::Right, offset), (2560 - 380, 1392 - 174));
    }

    fn screen(id: &str, x: i32, width: i32, primary: bool) -> Screen {
        let area = Area {
            x,
            y: 0,
            width,
            height: 1080,
        };
        Screen {
            id: id.into(),
            bounds: area,
            work_area: area,
            primary,
        }
    }

    #[test]
    fn chosen_screen_or_primary_when_unplugged() {
        let screens = [
            screen("DISPLAY1", 0, 2560, true),
            screen("DISPLAY2", -1920, 1920, false),
        ];
        assert_eq!(
            choose_screen(&screens, Some("DISPLAY2")).unwrap().id,
            "DISPLAY2"
        );
        assert_eq!(choose_screen(&screens, None).unwrap().id, "DISPLAY1");
        assert_eq!(
            choose_screen(&screens, Some("DISPLAY3")).unwrap().id,
            "DISPLAY1"
        );
        assert!(choose_screen(&[], None).is_none());
    }

    #[test]
    fn screens_are_numbered_from_left_to_right() {
        let screens = [
            screen("DISPLAY1", 0, 2560, true),
            screen("DISPLAY2", -1920, 1920, false),
        ];
        let labels = screen_labels(&screens);
        assert_eq!(
            labels[0],
            ("DISPLAY2".into(), "Screen 1 · 1920 × 1080".into())
        );
        assert_eq!(
            labels[1],
            ("DISPLAY1".into(), "Screen 2 · 2560 × 1080 · primary".into())
        );
    }

    #[test]
    fn the_window_spans_its_whole_edge() {
        assert_eq!(window_size(Edge::Top, WORK_AREA, 300), (2560, 300));
        assert_eq!(window_size(Edge::Bottom, WORK_AREA, 300), (2560, 300));
        assert_eq!(window_size(Edge::Left, WORK_AREA, 400), (400, 1392));
        // Spanning the edge, the window starts at its beginning and the
        // anchor is the notch's position along it.
        let placement = Placement {
            edge: Edge::Top,
            offset: 0.25,
            ..Placement::default()
        };
        let layout = layout(&placement, WORK_AREA, (2560, 300), 300, 0);
        assert_eq!(layout.window, (0, 0));
        assert_eq!(layout.anchor, 640);
    }
}
