//! Cursor resistance at the notch edge, see docs/fonctionnel/DF-0004-resistance-du-curseur.md.
//! Pure geometry: the OS layer feeds mouse moves in and applies the verdict.

/// Screen rectangle in physical pixels; `right` and `bottom` are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn contains(&self, (x, y): (i32, i32)) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// How far the user must push into the notch (physical px, summed over the
/// blocked moves) to get through.
pub const BREAKTHROUGH: f64 = 80.0;
/// Pushes further apart than this (ms) start over from zero.
pub const PUSH_WINDOW_MS: u32 = 400;

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Let the move happen.
    Allow,
    /// Block the move and put the cursor here instead, against the edge.
    Hold(i32, i32),
}

#[derive(Default)]
pub struct Resistance {
    pressure: f64,
    last_push_ms: u32,
}

impl Resistance {
    /// Decides what to do with a cursor move from `from` to `to` at time `now_ms`.
    pub fn on_move(
        &mut self,
        rect: Rect,
        from: (i32, i32),
        to: (i32, i32),
        now_ms: u32,
    ) -> Verdict {
        if rect.contains(from) || !rect.contains(to) {
            return Verdict::Allow;
        }
        if now_ms.wrapping_sub(self.last_push_ms) > PUSH_WINDOW_MS {
            self.pressure = 0.0;
        }
        self.last_push_ms = now_ms;
        self.pressure += f64::from(to.0 - from.0).hypot(f64::from(to.1 - from.1));
        if self.pressure >= BREAKTHROUGH {
            self.pressure = 0.0;
            return Verdict::Allow;
        }
        let (x, y) = against_edge(rect, from, to);
        Verdict::Hold(x, y)
    }
}

/// The point just outside `rect`, on the side the cursor comes from, keeping
/// the movement along that edge so the cursor can slide on it.
fn against_edge(rect: Rect, from: (i32, i32), to: (i32, i32)) -> (i32, i32) {
    if from.1 >= rect.bottom {
        (to.0, rect.bottom)
    } else if from.1 < rect.top {
        (to.0, rect.top - 1)
    } else if from.0 < rect.left {
        (rect.left - 1, to.1)
    } else {
        (rect.right, to.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 300 x 36 notch at the top-center of a 2560-wide screen.
    const NOTCH: Rect = Rect {
        left: 1130,
        top: 0,
        right: 1430,
        bottom: 36,
    };

    #[test]
    fn moves_away_from_the_notch_are_free() {
        let mut r = Resistance::default();
        assert_eq!(r.on_move(NOTCH, (1200, 100), (1200, 90), 0), Verdict::Allow);
        assert_eq!(r.on_move(NOTCH, (1200, 20), (1200, 60), 0), Verdict::Allow);
    }

    #[test]
    fn entering_from_below_is_held_against_the_bottom_edge() {
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1200, 40), (1210, 30), 0),
            Verdict::Hold(1210, 36)
        );
    }

    #[test]
    fn entering_from_the_sides_is_held_against_that_side() {
        let mut r = Resistance::default();
        assert_eq!(
            r.on_move(NOTCH, (1125, 10), (1135, 12), 0),
            Verdict::Hold(1129, 12)
        );
        assert_eq!(
            r.on_move(NOTCH, (1435, 10), (1425, 12), 0),
            Verdict::Hold(1430, 12)
        );
    }

    #[test]
    fn pushing_long_enough_breaks_through() {
        let mut r = Resistance::default();
        let mut t = 0;
        // 10 px pushes, every 10 ms: blocked until 80 px of pressure.
        for _ in 0..7 {
            assert!(matches!(
                r.on_move(NOTCH, (1200, 36), (1200, 26), t),
                Verdict::Hold(..)
            ));
            t += 10;
        }
        assert_eq!(r.on_move(NOTCH, (1200, 36), (1200, 26), t), Verdict::Allow);
    }

    #[test]
    fn hesitant_pushes_start_over() {
        let mut r = Resistance::default();
        for i in 0..7 {
            assert!(matches!(
                r.on_move(NOTCH, (1200, 36), (1200, 26), i * 10),
                Verdict::Hold(..)
            ));
        }
        // Pause longer than the push window: pressure is reset.
        assert!(matches!(
            r.on_move(NOTCH, (1200, 36), (1200, 26), 1000),
            Verdict::Hold(..)
        ));
    }

    #[test]
    fn a_fast_flick_goes_through_at_once() {
        let mut r = Resistance::default();
        assert_eq!(r.on_move(NOTCH, (1200, 120), (1200, 20), 0), Verdict::Allow);
    }
}
