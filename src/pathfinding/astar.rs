use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// 8-directional neighbors: (dx, dy, base_cost)
const DIRS: [(i32, i32, f32); 8] = [
    (1, 0, 1.0),
    (-1, 0, 1.0),
    (0, 1, 1.0),
    (0, -1, 1.0),
    (1, 1, std::f32::consts::SQRT_2),
    (1, -1, std::f32::consts::SQRT_2),
    (-1, 1, std::f32::consts::SQRT_2),
    (-1, -1, std::f32::consts::SQRT_2),
];

/// Octile distance heuristic — optimal and admissible for 8-directional movement.
pub fn octile_heuristic(x0: u32, y0: u32, x1: u32, y1: u32) -> f32 {
    let dx = (x1 as f32 - x0 as f32).abs();
    let dy = (y1 as f32 - y0 as f32).abs();
    let diag = dx.min(dy);
    let straight = dx.max(dy) - diag;
    diag * std::f32::consts::SQRT_2 + straight
}

/// A* pathfinding on a tile grid. 8-directional with octile distance heuristic.
/// No corner-cutting: diagonals blocked if either adjacent cardinal is impassable.
///
/// `walk_cost` is per-tile: 0.0 = impassable, >0 = movement cost multiplier.
/// Returns tile coords from start to goal (inclusive), or `None` if no path found.
pub fn astar_tile_grid(
    start: (u32, u32),
    goal: (u32, u32),
    width: u32,
    height: u32,
    walk_cost: &[f32],
    max_expansions: u32,
) -> Option<Vec<(u32, u32)>> {
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };
    let n = (width * height) as usize;

    if walk_cost[idx(start.0, start.1)] <= 0.0 || walk_cost[idx(goal.0, goal.1)] <= 0.0 {
        return None;
    }

    let mut g = vec![f32::INFINITY; n];
    let mut prev = vec![u32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();

    let start_idx = idx(start.0, start.1) as u32;
    let goal_idx = idx(goal.0, goal.1) as u32;

    g[start_idx as usize] = 0.0;
    let h0 = octile_heuristic(start.0, start.1, goal.0, goal.1);
    heap.push(Reverse((h0.to_bits(), start_idx)));

    let mut expansions = 0u32;

    while let Some(Reverse((_f_bits, ci))) = heap.pop() {
        if ci == goal_idx {
            break;
        }

        expansions += 1;
        if expansions > max_expansions {
            return None;
        }

        let cx = ci % width;
        let cy = ci / width;
        let cg = g[ci as usize];

        for &(dx, dy, base_cost) in &DIRS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;

            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }

            let nxu = nx as u32;
            let nyu = ny as u32;
            let ni = idx(nxu, nyu);

            let tile_cost = walk_cost[ni];
            if tile_cost <= 0.0 {
                continue;
            }

            // No corner-cutting: both adjacent cardinals must be walkable
            if dx != 0 && dy != 0 {
                let adj1 = idx(cx.wrapping_add_signed(dx), cy);
                let adj2 = idx(cx, cy.wrapping_add_signed(dy));
                if walk_cost[adj1] <= 0.0 || walk_cost[adj2] <= 0.0 {
                    continue;
                }
            }

            let ng = cg + base_cost * tile_cost;
            if ng < g[ni] {
                g[ni] = ng;
                prev[ni] = ci;
                let h = octile_heuristic(nxu, nyu, goal.0, goal.1);
                let f = ng + h;
                heap.push(Reverse((f.to_bits(), ni as u32)));
            }
        }
    }

    if g[goal_idx as usize].is_infinite() {
        return None;
    }

    let mut path = Vec::new();
    let mut cur = goal_idx;
    while cur != start_idx {
        path.push((cur % width, cur / width));
        cur = prev[cur as usize];
        if cur == u32::MAX {
            return None;
        }
    }
    path.push(start);
    path.reverse();
    Some(path)
}

/// A* restricted to a rectangular subregion of the tile grid.
/// Used by HPA* for intra-cluster pathfinding.
pub fn astar_tile_grid_bounded(
    start: (u32, u32),
    goal: (u32, u32),
    walk_cost: &[f32],
    grid_width: u32,
    grid_height: u32,
    bound_x: u32,
    bound_y: u32,
    bound_w: u32,
    bound_h: u32,
    max_expansions: u32,
) -> Option<Vec<(u32, u32)>> {
    let idx = |x: u32, y: u32| -> usize { (y * grid_width + x) as usize };
    let n = (grid_width * grid_height) as usize;

    let in_bounds =
        |x: u32, y: u32| -> bool { x >= bound_x && x < bound_x + bound_w && y >= bound_y && y < bound_y + bound_h };

    if !in_bounds(start.0, start.1) || !in_bounds(goal.0, goal.1) {
        return None;
    }
    if walk_cost[idx(start.0, start.1)] <= 0.0 || walk_cost[idx(goal.0, goal.1)] <= 0.0 {
        return None;
    }

    let mut g = vec![f32::INFINITY; n];
    let mut prev = vec![u32::MAX; n];
    let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();

    let start_idx = idx(start.0, start.1) as u32;
    let goal_idx = idx(goal.0, goal.1) as u32;

    g[start_idx as usize] = 0.0;
    let h0 = octile_heuristic(start.0, start.1, goal.0, goal.1);
    heap.push(Reverse((h0.to_bits(), start_idx)));

    let mut expansions = 0u32;

    while let Some(Reverse((_f_bits, ci))) = heap.pop() {
        if ci == goal_idx {
            break;
        }
        expansions += 1;
        if expansions > max_expansions {
            return None;
        }

        let cx = ci % grid_width;
        let cy = ci / grid_width;
        let cg = g[ci as usize];

        for &(dx, dy, base_cost) in &DIRS {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= grid_width as i32 || ny >= grid_height as i32 {
                continue;
            }
            let nxu = nx as u32;
            let nyu = ny as u32;
            if !in_bounds(nxu, nyu) {
                continue;
            }

            let ni = idx(nxu, nyu);
            let tile_cost = walk_cost[ni];
            if tile_cost <= 0.0 {
                continue;
            }

            if dx != 0 && dy != 0 {
                let adj1 = idx(cx.wrapping_add_signed(dx), cy);
                let adj2 = idx(cx, cy.wrapping_add_signed(dy));
                if walk_cost[adj1] <= 0.0 || walk_cost[adj2] <= 0.0 {
                    continue;
                }
            }

            let ng = cg + base_cost * tile_cost;
            if ng < g[ni] {
                g[ni] = ng;
                prev[ni] = ci;
                let h = octile_heuristic(nxu, nyu, goal.0, goal.1);
                heap.push(Reverse(((ng + h).to_bits(), ni as u32)));
            }
        }
    }

    if g[goal_idx as usize].is_infinite() {
        return None;
    }

    let mut path = Vec::new();
    let mut cur = goal_idx;
    while cur != start_idx {
        path.push((cur % grid_width, cur / grid_width));
        cur = prev[cur as usize];
        if cur == u32::MAX {
            return None;
        }
    }
    path.push(start);
    path.reverse();
    Some(path)
}

/// Compute the tile-step cost along a path using octile distances.
pub fn tile_path_cost(path: &[(u32, u32)]) -> f32 {
    let mut cost = 0.0f32;
    for pair in path.windows(2) {
        let dx = (pair[1].0 as f32 - pair[0].0 as f32).abs();
        let dy = (pair[1].1 as f32 - pair[0].1 as f32).abs();
        cost += if dx > 0.0 && dy > 0.0 { std::f32::consts::SQRT_2 } else { 1.0 };
    }
    cost
}

