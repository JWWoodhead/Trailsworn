use gold_and_glory::pathfinding::{astar_tile_grid, astar_tile_grid_bounded};

fn open_grid(w: u32, h: u32) -> Vec<f32> {
    vec![1.0; (w * h) as usize]
}

#[test]
fn straight_line_path() {
    let w = 10;
    let h = 10;
    let cost = open_grid(w, h);
    let path = astar_tile_grid((0, 0), (9, 0), w, h, &cost, 1000).unwrap();
    assert_eq!(path.first(), Some(&(0, 0)));
    assert_eq!(path.last(), Some(&(9, 0)));
    assert_eq!(path.len(), 10);
}

#[test]
fn diagonal_path() {
    let w = 10;
    let h = 10;
    let cost = open_grid(w, h);
    let path = astar_tile_grid((0, 0), (5, 5), w, h, &cost, 1000).unwrap();
    assert_eq!(path.first(), Some(&(0, 0)));
    assert_eq!(path.last(), Some(&(5, 5)));
    assert_eq!(path.len(), 6);
}

#[test]
fn blocked_goal_returns_none() {
    let w = 10;
    let h = 10;
    let mut cost = open_grid(w, h);
    cost[(0 * w + 9) as usize] = 0.0;
    assert!(astar_tile_grid((0, 0), (9, 0), w, h, &cost, 1000).is_none());
}

#[test]
fn wall_forces_detour() {
    let w = 10;
    let h = 10;
    let mut cost = open_grid(w, h);
    for x in 0..9 {
        cost[(4 * w + x) as usize] = 0.0;
    }
    let path = astar_tile_grid((4, 0), (4, 8), w, h, &cost, 5000).unwrap();
    assert_eq!(path.first(), Some(&(4, 0)));
    assert_eq!(path.last(), Some(&(4, 8)));
    assert!(path.len() > 9);
}

#[test]
fn no_corner_cutting() {
    let w = 3;
    let h = 3;
    let mut cost = open_grid(w, h);
    cost[(0 * w + 1) as usize] = 0.0; // (1, 0)
    cost[(1 * w + 0) as usize] = 0.0; // (0, 1)
    let path = astar_tile_grid((0, 0), (1, 1), w, h, &cost, 100);
    assert!(path.is_none());
}

#[test]
fn expansion_limit_returns_none() {
    let w = 100;
    let h = 100;
    let cost = open_grid(w, h);
    assert!(astar_tile_grid((0, 0), (99, 99), w, h, &cost, 10).is_none());
}

#[test]
fn bounded_stays_in_bounds() {
    let w = 20;
    let h = 20;
    let cost = open_grid(w, h);
    let path = astar_tile_grid_bounded((5, 5), (9, 9), &cost, w, h, 5, 5, 5, 5, 500).unwrap();
    for &(x, y) in &path {
        assert!(x >= 5 && x < 10 && y >= 5 && y < 10);
    }
}
