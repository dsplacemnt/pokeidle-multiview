//! Pure layout math: splits the game area into tiles for 1–4 visible slots.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Gap between tiles, in logical pixels.
pub const GAP: f64 = 2.0;

/// 1 slot fills the area, 2 sit side by side, 3–4 use a 2×2 grid (filled row by row).
pub fn grid(area: Rect, count: usize) -> Vec<Rect> {
    let (cols, rows) = match count {
        0 => return Vec::new(),
        1 => (1, 1),
        2 => (2, 1),
        _ => (2, 2),
    };
    let cell_w = ((area.w - GAP * (cols - 1) as f64) / cols as f64).max(0.0);
    let cell_h = ((area.h - GAP * (rows - 1) as f64) / rows as f64).max(0.0);
    (0..count)
        .map(|i| {
            let (col, row) = (i % cols, i / cols);
            Rect {
                x: area.x + col as f64 * (cell_w + GAP),
                y: area.y + row as f64 * (cell_h + GAP),
                w: cell_w,
                h: cell_h,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect { x: 0.0, y: 40.0, w: 1002.0, h: 602.0 };

    #[test]
    fn single_slot_fills_area() {
        assert_eq!(grid(AREA, 1), vec![AREA]);
    }

    #[test]
    fn four_slots_make_a_2x2_grid() {
        let r = grid(AREA, 4);
        assert_eq!(r.len(), 4);
        assert_eq!(r[3], Rect { x: 502.0, y: 342.0, w: 500.0, h: 300.0 });
    }

    #[test]
    fn no_slots_no_tiles() {
        assert!(grid(AREA, 0).is_empty());
    }
}
