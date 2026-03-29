/// Bresenham line-of-sight check.
///
/// Returns `true` if no **intermediate** tile between `from` and `to` has
/// `blocks_los == true`. Start and end tiles are excluded from the check
/// (you can always see your own tile and your target's tile).
pub fn has_line_of_sight(
    from: (u32, u32),
    to: (u32, u32),
    width: u32,
    blocks_los: &[bool],
) -> bool {
    let mut x0 = from.0 as i32;
    let mut y0 = from.1 as i32;
    let x1 = to.0 as i32;
    let y1 = to.1 as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        // Check intermediate tiles (skip start and end)
        if (x0 != from.0 as i32 || y0 != from.1 as i32)
            && (x0 != x1 || y0 != y1)
        {
            let idx = (y0 as u32 * width + x0 as u32) as usize;
            if blocks_los[idx] {
                return false;
            }
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid(width: u32, height: u32, blocked: &[(u32, u32)]) -> Vec<bool> {
        let mut grid = vec![false; (width * height) as usize];
        for &(x, y) in blocked {
            grid[(y * width + x) as usize] = true;
        }
        grid
    }

    #[test]
    fn open_field_clear() {
        let grid = make_grid(10, 10, &[]);
        assert!(has_line_of_sight((0, 0), (9, 9), 10, &grid));
    }

    #[test]
    fn wall_blocks() {
        let grid = make_grid(10, 10, &[(5, 5)]);
        assert!(!has_line_of_sight((0, 0), (9, 9), 10, &grid));
    }

    #[test]
    fn same_tile_always_clear() {
        let grid = make_grid(10, 10, &[(3, 3)]);
        assert!(has_line_of_sight((3, 3), (3, 3), 10, &grid));
    }

    #[test]
    fn adjacent_always_clear() {
        // Even if both tiles block, adjacents can always see each other
        let grid = make_grid(10, 10, &[(3, 3), (4, 3)]);
        assert!(has_line_of_sight((3, 3), (4, 3), 10, &grid));
    }

    #[test]
    fn diagonal_through_blocker() {
        let grid = make_grid(5, 5, &[(2, 2)]);
        assert!(!has_line_of_sight((0, 0), (4, 4), 5, &grid));
    }

    #[test]
    fn clear_path_around_blocker() {
        // Horizontal line doesn't hit a blocker that's off to the side
        let grid = make_grid(10, 10, &[(5, 3)]);
        assert!(has_line_of_sight((0, 5), (9, 5), 10, &grid));
    }

    #[test]
    fn vertical_line() {
        let grid = make_grid(5, 10, &[(2, 5)]);
        assert!(!has_line_of_sight((2, 0), (2, 9), 5, &grid));
    }

    #[test]
    fn horizontal_line() {
        let grid = make_grid(10, 5, &[(5, 2)]);
        assert!(!has_line_of_sight((0, 2), (9, 2), 10, &grid));
    }
}
