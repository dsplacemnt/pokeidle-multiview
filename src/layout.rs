//! Pure layout math: where each slot goes and where the draggable splitters sit.
//!
//! - Grid: 1 slot fills the area; 2 sit side by side; 3 use two on top and one wide below;
//!   4 make a 2×2 grid. The vertical and horizontal splitters move the column/row boundary.
//! - Focus: the focused slot is large on the left, the others share a column on the right;
//!   one splitter moves the boundary.

use serde::Serialize;

use crate::config::GridSplit;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Space between tiles, in logical pixels; the splitters fill it.
pub const GAP: f64 = 6.0;

/// Which setting a splitter changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitKind {
    /// Grid column boundary (vertical bar, drag left/right).
    GridV,
    /// Grid row boundary (horizontal bar, drag up/down).
    GridH,
    /// Focus mode boundary (vertical bar).
    Focus,
}

impl SplitKind {
    pub fn is_vertical(self) -> bool {
        matches!(self, SplitKind::GridV | SplitKind::Focus)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub tiles: Vec<(u8, Rect)>,
    pub splitters: Vec<(SplitKind, Rect)>,
}

/// `ids` in display order; `focus` = the large slot in focus mode (None = grid).
pub fn compute(area: Rect, ids: &[u8], focus: Option<u8>, grid: GridSplit, focus_split: f64) -> Layout {
    let mut tiles = Vec::new();
    let mut splitters = Vec::new();
    let n = ids.len();
    if n == 0 {
        return Layout { tiles, splitters };
    }
    if n == 1 {
        tiles.push((ids[0], area));
        return Layout { tiles, splitters };
    }

    // Splits the area's width at `fraction`, leaving GAP between the two parts.
    let left_w = |fraction: f64| ((area.w - GAP) * fraction).max(0.0);
    let top_h = |fraction: f64| ((area.h - GAP) * fraction).max(0.0);

    if let Some(focused) = focus.filter(|f| ids.contains(f)) {
        let lw = left_w(focus_split);
        let rx = area.x + lw + GAP;
        let rw = (area.w - lw - GAP).max(0.0);
        tiles.push((focused, Rect { x: area.x, y: area.y, w: lw, h: area.h }));
        let rest: Vec<u8> = ids.iter().copied().filter(|id| *id != focused).collect();
        let count = rest.len() as f64;
        let each = ((area.h - GAP * (count - 1.0)) / count).max(0.0);
        for (i, id) in rest.into_iter().enumerate() {
            let y = area.y + i as f64 * (each + GAP);
            tiles.push((id, Rect { x: rx, y, w: rw, h: each }));
        }
        splitters.push((SplitKind::Focus, Rect { x: area.x + lw, y: area.y, w: GAP, h: area.h }));
        return Layout { tiles, splitters };
    }

    let lw = left_w(grid.v);
    let rx = area.x + lw + GAP;
    let rw = (area.w - lw - GAP).max(0.0);
    match n {
        2 => {
            tiles.push((ids[0], Rect { x: area.x, y: area.y, w: lw, h: area.h }));
            tiles.push((ids[1], Rect { x: rx, y: area.y, w: rw, h: area.h }));
            splitters.push((SplitKind::GridV, Rect { x: area.x + lw, y: area.y, w: GAP, h: area.h }));
        }
        _ => {
            let th = top_h(grid.h);
            let by = area.y + th + GAP;
            let bh = (area.h - th - GAP).max(0.0);
            tiles.push((ids[0], Rect { x: area.x, y: area.y, w: lw, h: th }));
            tiles.push((ids[1], Rect { x: rx, y: area.y, w: rw, h: th }));
            if n == 3 {
                // The third slot spans the whole bottom row.
                tiles.push((ids[2], Rect { x: area.x, y: by, w: area.w, h: bh }));
                splitters.push((SplitKind::GridV, Rect { x: area.x + lw, y: area.y, w: GAP, h: th }));
            } else {
                tiles.push((ids[2], Rect { x: area.x, y: by, w: lw, h: bh }));
                tiles.push((ids[3], Rect { x: rx, y: by, w: rw, h: bh }));
                splitters.push((SplitKind::GridV, Rect { x: area.x + lw, y: area.y, w: GAP, h: area.h }));
            }
            splitters.push((SplitKind::GridH, Rect { x: area.x, y: area.y + th, w: area.w, h: GAP }));
        }
    }
    Layout { tiles, splitters }
}

/// Turns a pointer position (logical px, same space as `area`) into a split fraction.
pub fn fraction_at(area: Rect, kind: SplitKind, x: f64, y: f64) -> f64 {
    let (pos, start, len) = if kind.is_vertical() { (x, area.x, area.w) } else { (y, area.y, area.h) };
    if len <= GAP {
        return 0.5;
    }
    // The pointer is in the middle of the splitter, which starts at fraction*(len-GAP).
    (pos - start - GAP / 2.0) / (len - GAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect { x: 0.0, y: 40.0, w: 1006.0, h: 606.0 };
    const HALF: GridSplit = GridSplit { v: 0.5, h: 0.5 };

    fn tile(l: &Layout, id: u8) -> Rect {
        l.tiles.iter().find(|(i, _)| *i == id).unwrap().1
    }

    #[test]
    fn single_slot_fills_area() {
        let l = compute(AREA, &[1], None, HALF, 0.7);
        assert_eq!(l.tiles, vec![(1, AREA)]);
        assert!(l.splitters.is_empty());
    }

    #[test]
    fn four_slots_make_a_2x2_grid_with_two_splitters() {
        let l = compute(AREA, &[1, 2, 3, 4], None, HALF, 0.7);
        assert_eq!(tile(&l, 4), Rect { x: 506.0, y: 346.0, w: 500.0, h: 300.0 });
        assert_eq!(l.splitters.len(), 2);
        assert_eq!(l.splitters[0], (SplitKind::GridV, Rect { x: 500.0, y: 40.0, w: GAP, h: 606.0 }));
        assert_eq!(l.splitters[1], (SplitKind::GridH, Rect { x: 0.0, y: 340.0, w: 1006.0, h: GAP }));
    }

    #[test]
    fn three_slots_put_the_third_across_the_bottom() {
        let l = compute(AREA, &[1, 2, 3], None, HALF, 0.7);
        assert_eq!(tile(&l, 3), Rect { x: 0.0, y: 346.0, w: 1006.0, h: 300.0 });
        // The column splitter only covers the top row.
        assert_eq!(l.splitters[0].1.h, 300.0);
    }

    #[test]
    fn grid_split_moves_the_boundary() {
        let l = compute(AREA, &[1, 2], None, GridSplit { v: 0.25, h: 0.5 }, 0.7);
        assert_eq!(tile(&l, 1).w, 250.0);
        assert_eq!(tile(&l, 2), Rect { x: 256.0, y: 40.0, w: 750.0, h: 606.0 });
    }

    #[test]
    fn focus_puts_others_in_a_side_column() {
        let l = compute(AREA, &[1, 2, 3], Some(2), HALF, 0.7);
        assert_eq!(tile(&l, 2), Rect { x: 0.0, y: 40.0, w: 700.0, h: 606.0 });
        assert_eq!(tile(&l, 1), Rect { x: 706.0, y: 40.0, w: 300.0, h: 300.0 });
        assert_eq!(tile(&l, 3), Rect { x: 706.0, y: 346.0, w: 300.0, h: 300.0 });
        assert_eq!(l.splitters, vec![(SplitKind::Focus, Rect { x: 700.0, y: 40.0, w: GAP, h: 606.0 })]);
    }

    #[test]
    fn focus_on_a_missing_slot_falls_back_to_grid() {
        let l = compute(AREA, &[1, 2], Some(4), HALF, 0.7);
        assert_eq!(l.splitters[0].0, SplitKind::GridV);
    }

    #[test]
    fn pointer_position_maps_back_to_the_split() {
        // Splitter of a 0.25 split starts at x=250; its middle is 253.
        let f = fraction_at(AREA, SplitKind::GridV, 253.0, 0.0);
        assert!((f - 0.25).abs() < 1e-9);
        let f = fraction_at(AREA, SplitKind::GridH, 0.0, 40.0 + 300.0 + 3.0);
        assert!((f - 0.5).abs() < 1e-9);
    }

    #[test]
    fn no_slots_no_tiles() {
        let l = compute(AREA, &[], None, HALF, 0.7);
        assert!(l.tiles.is_empty() && l.splitters.is_empty());
    }
}
