//! Cursor resistance at the notch edge, see docs/fonctionnel/DF-0004-resistance-du-curseur.md.
//! Pure geometry: the OS layer feeds mouse moves in and applies the verdict.

use crate::placement::Edge as Side;

/// The notch shape in physical screen pixels: a rectangle (`right` and
/// `bottom` exclusive) glued to the screen edge `attached`, whose two
/// corners on the inside of the screen are rounded with `radius`, exactly
/// like the drawn notch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub radius: i32,
    pub attached: Side,
}

impl Rect {
    /// Inside the bounding rectangle, rounded corners included.
    fn in_bounds(&self, (x, y): (i32, i32)) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Inside the visible shape: the transparent bits of the rounded corners
    /// do not count, so the cursor does not snag on what it cannot see.
    pub fn contains(&self, point: (i32, i32)) -> bool {
        if !self.in_bounds(point) {
            return false;
        }
        let r = f64::from(self.radius);
        // Pixel centers, against the centers of the rounded corner circles.
        let (x, y) = (f64::from(point.0) + 0.5, f64::from(point.1) + 0.5);
        let (left, top) = (f64::from(self.left), f64::from(self.top));
        let (right, bottom) = (f64::from(self.right), f64::from(self.bottom));
        // The two rounded corners are on the inner side, opposite the
        // attached edge: (is_left, is_top) for each.
        let corners = match self.attached {
            Side::Top => [(true, false), (false, false)],
            Side::Bottom => [(true, true), (false, true)],
            Side::Left => [(false, true), (false, false)],
            Side::Right => [(true, true), (true, false)],
        };
        corners.iter().all(|&(is_left, is_top)| {
            let cx = if is_left { left + r } else { right - r };
            let cy = if is_top { top + r } else { bottom - r };
            let in_corner_x = if is_left { x < cx } else { x > cx };
            let in_corner_y = if is_top { y < cy } else { y > cy };
            !(in_corner_x && in_corner_y) || (x - cx).hypot(y - cy) <= r
        })
    }
}

/// Presets offered in the settings: how hard the user must push to get through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Strength {
    Soft,
    #[default]
    Medium,
    Strong,
}

impl Strength {
    /// Push distance (physical px, summed over the blocked moves) to get through.
    pub fn breakthrough(self) -> f64 {
        match self {
            Strength::Soft => 40.0,
            Strength::Medium => 80.0,
            Strength::Strong => 150.0,
        }
    }
}

/// Pushes further apart than this (ms) start over from zero.
pub const PUSH_WINDOW_MS: u32 = 400;
/// Most a single mouse event can add to the pressure (px). With pointer
/// acceleration a quick flick delivers huge deltas: without a cap it would go
/// through in one or two events and the wall would not be felt.
const MAX_STEP: f64 = 15.0;
/// Minimum push duration, in ms per px of breakthrough: a flick, however
/// strong, must last a little to get through (80 px -> 160 ms).
const MS_PER_PX: f64 = 2.0;
// All edges resist the same. The sides once felt much softer, but that was the
// screen-top bypass fixed by `clip_to_screen_top`, not a matter of strength:
// measured afterwards, equal thresholds give comparable push times.

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Let the move happen.
    Allow,
    /// The cursor pushed hard enough and gets into the notch.
    Enter(Breakthrough),
    /// Block the move and put the cursor here instead, against the edge.
    Hold(i32, i32),
}

/// How a breakthrough happened, logged to tune the resistance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breakthrough {
    /// Through the edge facing the inside of the screen (vs a lateral edge).
    pub inner: bool,
    pub duration_ms: u32,
    pub events: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

impl Edge {
    /// The edge the cursor crosses when going from `from` (outside) into `rect`.
    fn crossed(rect: Rect, from: (i32, i32)) -> Edge {
        if from.1 >= rect.bottom {
            Edge::Bottom
        } else if from.1 < rect.top {
            Edge::Top
        } else if from.0 < rect.left {
            Edge::Left
        } else {
            Edge::Right
        }
    }

    /// The edge of the notch facing the inside of the screen.
    fn inner(attached: Side) -> Edge {
        match attached {
            Side::Top => Edge::Bottom,
            Side::Bottom => Edge::Top,
            Side::Left => Edge::Right,
            Side::Right => Edge::Left,
        }
    }

    /// The point just outside `rect` on this edge, keeping the movement along
    /// the edge so the cursor can slide on it.
    fn hold(self, rect: Rect, to: (i32, i32)) -> (i32, i32) {
        match self {
            Edge::Bottom => (to.0, rect.bottom),
            Edge::Top => (to.0, rect.top - 1),
            Edge::Left => (rect.left - 1, to.1),
            Edge::Right => (rect.right, to.1),
        }
    }

    /// How deep `to` goes past this edge: only the push into the notch counts,
    /// not the movement along the edge.
    fn depth(self, rect: Rect, to: (i32, i32)) -> f64 {
        let depth = match self {
            Edge::Bottom => rect.bottom - to.1,
            Edge::Top => to.1 - rect.top + 1,
            Edge::Left => to.0 - rect.left + 1,
            Edge::Right => rect.right - to.0,
        };
        f64::from(depth)
    }
}

/// True when `to` lies past the screen edge the notch is attached to, where it
/// may be off-screen (cheap check before asking the OS about monitors).
pub fn beyond_attached_edge(rect: Rect, to: (i32, i32)) -> bool {
    match rect.attached {
        Side::Top => to.1 < rect.top,
        Side::Bottom => to.1 >= rect.bottom,
        Side::Left => to.0 < rect.left,
        Side::Right => to.0 >= rect.right,
    }
}

/// Windows hands the hook the *requested* position, which can be off-screen:
/// a cursor resting on the screen edge the notch is attached to, pushed
/// outwards and sideways, asks for e.g. y < 0, which Windows then clamps back
/// onto that edge, right inside the notch, bypassing the resistance. Applies
/// the same clamp first. `offscreen`: no monitor contains `to` (a monitor
/// beyond that edge keeps those positions valid).
pub fn clip_to_screen_edge(rect: Rect, to: (i32, i32), offscreen: bool) -> (i32, i32) {
    if !offscreen || !beyond_attached_edge(rect, to) {
        return to;
    }
    match rect.attached {
        Side::Top => (to.0, rect.top),
        Side::Bottom => (to.0, rect.bottom - 1),
        Side::Left => (rect.left, to.1),
        Side::Right => (rect.right - 1, to.1),
    }
}

#[derive(Default)]
pub struct Resistance {
    pressure: f64,
    push_start_ms: u32,
    last_push_ms: u32,
    events: u32,
}

impl Resistance {
    /// Decides what to do with a cursor move from `from` to `to` at time `now_ms`.
    /// Getting through takes `breakthrough` px of pushing into the notch,
    /// sustained for a minimum time, the same on every edge.
    pub fn on_move(
        &mut self,
        rect: Rect,
        from: (i32, i32),
        to: (i32, i32),
        now_ms: u32,
        breakthrough: f64,
    ) -> Verdict {
        if rect.contains(from) || !rect.contains(to) {
            return Verdict::Allow;
        }
        if now_ms.wrapping_sub(self.last_push_ms) > PUSH_WINDOW_MS {
            self.pressure = 0.0;
            self.push_start_ms = now_ms;
            self.events = 0;
        }
        self.last_push_ms = now_ms;
        self.events += 1;

        // Coming from a transparent corner (inside the bounding rectangle but
        // outside the shape): the corners are on the inner side, so it is an
        // inner entry, and the cursor stays where it was, just outside the curve.
        let from_corner = rect.in_bounds(from);
        let inner = Edge::inner(rect.attached);
        let edge = if from_corner {
            inner
        } else {
            Edge::crossed(rect, from)
        };
        let pushed_ms = now_ms.wrapping_sub(self.push_start_ms);
        self.pressure += edge.depth(rect, to).min(MAX_STEP);
        if self.pressure >= breakthrough && f64::from(pushed_ms) >= breakthrough * MS_PER_PX {
            self.pressure = 0.0;
            return Verdict::Enter(Breakthrough {
                inner: edge == inner,
                duration_ms: pushed_ms,
                events: self.events,
            });
        }
        let (x, y) = if from_corner {
            from
        } else {
            edge.hold(rect, to)
        };
        Verdict::Hold(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEDIUM: f64 = 80.0;

    // A 300 x 36 notch at the top-center of a 2560-wide screen.
    const NOTCH: Rect = Rect {
        left: 1130,
        top: 0,
        right: 1430,
        bottom: 36,
        radius: 14,
        attached: Side::Top,
    };

    /// Pushes `step` px into the notch every `every_ms` from `from`, held
    /// against the edge each time; returns how many pushes it took to get through.
    fn pushes_to_enter(from: (i32, i32), step: (i32, i32), every_ms: u32) -> Option<u32> {
        let mut r = Resistance::default();
        for i in 1..=200 {
            let to = (from.0 + step.0, from.1 + step.1);
            if matches!(
                r.on_move(NOTCH, from, to, i * every_ms, MEDIUM),
                Verdict::Enter(_)
            ) {
                return Some(i);
            }
        }
        None
    }

    #[test]
    fn moves_away_from_the_notch_are_free() {
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1200, 100), (1200, 90), 0, MEDIUM),
            Verdict::Allow
        );
        assert_eq!(
            r.on_move(NOTCH, (1200, 20), (1200, 60), 0, MEDIUM),
            Verdict::Allow
        );
    }

    #[test]
    fn entering_from_below_is_held_against_the_bottom_edge() {
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1200, 40), (1210, 30), 0, MEDIUM),
            Verdict::Hold(1210, 36)
        );
    }

    #[test]
    fn entering_from_the_sides_is_held_against_that_side() {
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1125, 10), (1135, 12), 0, MEDIUM),
            Verdict::Hold(1129, 12)
        );
        assert_eq!(
            r.on_move(NOTCH, (1435, 10), (1425, 12), 0, MEDIUM),
            Verdict::Hold(1430, 12)
        );
    }

    #[test]
    fn steady_push_from_below_gets_through() {
        // 10 px every 25 ms: 80 px reached at the 8th push, after 175 ms (>= 160 ms).
        assert_eq!(pushes_to_enter((1200, 36), (0, -10), 25), Some(8));
    }

    #[test]
    fn every_edge_resists_the_same() {
        let bottom = pushes_to_enter((1200, 36), (0, -10), 25);
        let left = pushes_to_enter((1129, 10), (10, 0), 25);
        let right = pushes_to_enter((1430, 10), (-10, 0), 25);
        assert_eq!(left, bottom);
        assert_eq!(right, bottom);
    }

    #[test]
    fn a_fast_flick_does_not_go_through_at_once() {
        // Huge accelerated deltas, 1 ms apart: capped, and too short in time.
        let mut r = Resistance::default();
        for t in 0..10 {
            assert!(matches!(
                r.on_move(NOTCH, (1200, 36), (1200, 0), t, MEDIUM),
                Verdict::Hold(..)
            ));
        }
    }

    #[test]
    fn sliding_along_the_edge_adds_no_pressure() {
        let mut r = Resistance::default();
        // Mostly horizontal moves that dip 1 px into the notch.
        for i in 0..40 {
            assert!(matches!(
                r.on_move(
                    NOTCH,
                    (1150 + i * 5, 36),
                    (1155 + i * 5, 35),
                    i as u32 * 25,
                    MEDIUM
                ),
                Verdict::Hold(..)
            ));
        }
    }

    #[test]
    fn sliding_along_the_screen_top_into_the_side_is_held() {
        // Cursor at y = 0 pushed up-right: Windows requests y = -4 (off-screen).
        let mut r = Resistance::default();
        let to = clip_to_screen_edge(NOTCH, (1140, -4), true);
        assert_eq!(
            r.on_move(NOTCH, (1129, 0), to, 0, MEDIUM),
            Verdict::Hold(1129, 0)
        );
        // With a monitor above the notch, y < 0 is a real position: untouched.
        assert_eq!(clip_to_screen_edge(NOTCH, (1140, -4), false), (1140, -4));
    }

    #[test]
    fn transparent_rounded_corners_are_free() {
        // Bottom-left corner pixel: inside the bounding box, outside the curve.
        assert!(!NOTCH.contains((1130, 35)));
        assert!(!NOTCH.contains((1429, 35)));
        // Just inside the curve, and the straight parts, are the notch.
        assert!(NOTCH.contains((1138, 30)));
        assert!(NOTCH.contains((1130, 10)));
        // Passing diagonally through the transparent corner: no snag.
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1126, 40), (1131, 34), 0, MEDIUM),
            Verdict::Allow
        );
    }

    #[test]
    fn pushing_from_a_transparent_corner_into_the_notch_is_held() {
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1131, 34), (1140, 28), 0, MEDIUM),
            Verdict::Hold(1131, 34)
        );
    }

    #[test]
    fn rounded_corners_follow_the_attached_edge() {
        // Same 300 x 36 shape glued to the bottom of the screen: corners on top.
        let bottom = Rect {
            top: 1356,
            bottom: 1392,
            attached: Side::Bottom,
            ..NOTCH
        };
        assert!(!bottom.contains((1130, 1356)));
        assert!(bottom.contains((1130, 1391)));
        // A vertical 36 x 120 notch on the left edge: corners on the right.
        let left = Rect {
            left: 0,
            top: 600,
            right: 36,
            bottom: 720,
            radius: 14,
            attached: Side::Left,
        };
        assert!(!left.contains((35, 600)));
        assert!(!left.contains((35, 719)));
        assert!(left.contains((0, 600)));
        assert!(left.contains((20, 660)));
    }

    #[test]
    fn pushing_against_the_attached_screen_edge_is_clipped() {
        let right = Rect {
            left: 2524,
            top: 600,
            right: 2560,
            bottom: 720,
            radius: 14,
            attached: Side::Right,
        };
        // Cursor on the right screen border, pushed right and down: Windows
        // requests x = 2563 (off-screen), then clamps it inside the notch.
        let to = clip_to_screen_edge(right, (2563, 610), true);
        assert_eq!(to, (2559, 610));
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(right, (2559, 590), to, 0, MEDIUM),
            Verdict::Hold(2559, 599)
        );
    }

    #[test]
    fn hesitant_pushes_start_over() {
        let mut r = Resistance::default();
        for i in 0..7 {
            assert!(matches!(
                r.on_move(NOTCH, (1200, 36), (1200, 26), i * 25, MEDIUM),
                Verdict::Hold(..)
            ));
        }
        // Pause longer than the push window: pressure is reset.
        assert!(matches!(
            r.on_move(NOTCH, (1200, 36), (1200, 26), 1000, MEDIUM),
            Verdict::Hold(..)
        ));
    }
}
