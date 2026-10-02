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

/// Saved in config.json. `offset` is the position along the edge, from 0
/// (left / top end) to 1 (right / bottom end); 0.5 is centered. Free
/// placement along the edge (later step) will only change `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Placement {
    pub edge: Edge,
    pub offset: f64,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            edge: Edge::Top,
            offset: 0.5,
        }
    }
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Top-left corner of the notch window on screen, glued to `placement.edge`
/// of the monitor work area (so a bottom notch sits just above the taskbar)
/// and kept fully inside it.
pub fn window_position(placement: Placement, work_area: Area, window: (i32, i32)) -> (i32, i32) {
    let (width, height) = window;
    let offset = placement.offset.clamp(0.0, 1.0);
    let along = |start: i32, length: i32, size: i32| {
        let wanted = start + (offset * f64::from(length)).round() as i32 - size / 2;
        wanted.clamp(start, (start + length - size).max(start))
    };
    let wa = work_area;
    match placement.edge {
        Edge::Top => (along(wa.x, wa.width, width), wa.y),
        Edge::Bottom => (along(wa.x, wa.width, width), wa.y + wa.height - height),
        Edge::Left => (wa.x, along(wa.y, wa.height, height)),
        Edge::Right => (wa.x + wa.width - width, along(wa.y, wa.height, height)),
    }
}

/// Top-left corner of the visible shape inside the window: against the
/// attached side, centered along it.
pub fn shape_in_window(edge: Edge, window: (i32, i32), shape: (i32, i32)) -> (i32, i32) {
    let (ww, wh) = window;
    let (sw, sh) = shape;
    match edge {
        Edge::Top => ((ww - sw) / 2, 0),
        Edge::Bottom => ((ww - sw) / 2, wh - sh),
        Edge::Left => (0, (wh - sh) / 2),
        Edge::Right => (ww - sw, (wh - sh) / 2),
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

    fn at(edge: Edge, offset: f64) -> (i32, i32) {
        window_position(Placement { edge, offset }, WORK_AREA, WINDOW)
    }

    #[test]
    fn centered_on_each_edge() {
        assert_eq!(at(Edge::Top, 0.5), (1090, 0));
        assert_eq!(at(Edge::Bottom, 0.5), (1090, 1392 - 174));
        assert_eq!(at(Edge::Left, 0.5), (0, 696 - 87));
        assert_eq!(at(Edge::Right, 0.5), (2560 - 380, 696 - 87));
    }

    #[test]
    fn stays_inside_the_work_area_at_the_ends() {
        assert_eq!(at(Edge::Top, 0.0), (0, 0));
        assert_eq!(at(Edge::Top, 1.0), (2560 - 380, 0));
        assert_eq!(at(Edge::Left, 1.0), (0, 1392 - 174));
        assert_eq!(at(Edge::Right, 7.0), (2560 - 380, 1392 - 174));
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
            offset: 0.5,
        };
        assert_eq!(
            window_position(placement, second, WINDOW),
            (-1920 + 960 - 190, 200 + 1040 - 174)
        );
    }

    #[test]
    fn shape_is_glued_to_the_attached_side() {
        assert_eq!(shape_in_window(Edge::Top, WINDOW, (300, 36)), (40, 0));
        assert_eq!(shape_in_window(Edge::Bottom, WINDOW, (300, 36)), (40, 138));
        assert_eq!(shape_in_window(Edge::Left, WINDOW, (36, 120)), (0, 27));
        assert_eq!(shape_in_window(Edge::Right, WINDOW, (36, 120)), (344, 27));
    }
}
