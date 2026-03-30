use std::collections::{BinaryHeap, HashSet};
use std::cmp::Reverse;

use super::{SettlementSize, WorldCell};
use crate::worldgen::zone::ZoneType;

/// Maximum path cost for a hamlet/village to be connected to the road network.
/// Settlements with MST edge cost above this are considered too isolated.
const ISOLATION_THRESHOLD: f32 = 80.0;

/// Maximum path cost for bonus edges between towns/cities.
const BONUS_EDGE_MAX_COST: f32 = 120.0;

/// Generate a road network connecting settlements on the world map.
///
/// Algorithm:
/// 1. Collect unique settlement positions (deduplicate multi-cell footprints)
/// 2. Build a minimum spanning tree (Prim's) using A* path costs
/// 3. Prune leaf edges to isolated hamlets/villages
/// 4. Add bonus edges between towns/cities for loops
/// 5. Paint all paths onto cells (road, road_entry, road_class)
pub(super) fn generate_roads(cells: &mut [WorldCell], width: u32, height: u32) {
    // 1. Collect unique settlement positions
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut settlements: Vec<(usize, SettlementSize)> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        if let (Some(name), Some(size)) = (&cell.settlement_name, cell.settlement_size) {
            if seen_names.insert(name.clone()) {
                settlements.push((i, size));
            }
        }
    }

    if settlements.len() < 2 {
        return;
    }

    let n = settlements.len();

    // 2. Build MST via Prim's algorithm
    // We lazily compute A* paths as needed rather than precomputing all pairs.
    let mut in_tree = vec![false; n];
    // (cost, from_idx, to_idx, path)
    let mut mst_edges: Vec<(f32, usize, usize, Vec<usize>)> = Vec::new();

    // Start from the first settlement (arbitrary)
    in_tree[0] = true;

    // For each node not in tree, track cheapest edge into tree: (cost, tree_node, path)
    let mut cheapest: Vec<Option<(f32, usize, Vec<usize>)>> = vec![None; n];

    // Initialize: compute paths from node 0 to all others
    for j in 1..n {
        if let Some((cost, path)) = astar_world_grid(
            cells, width, height, settlements[0].0, settlements[j].0,
        ) {
            cheapest[j] = Some((cost, 0, path));
        }
    }

    for _ in 1..n {
        // Find the cheapest edge from outside tree into tree
        let mut best_idx = None;
        let mut best_cost = f32::MAX;
        for j in 0..n {
            if in_tree[j] { continue; }
            if let Some((cost, _, _)) = &cheapest[j] {
                if *cost < best_cost {
                    best_cost = *cost;
                    best_idx = Some(j);
                }
            }
        }

        let Some(new_node) = best_idx else { break };
        in_tree[new_node] = true;

        let (cost, from, path) = cheapest[new_node].take().unwrap();
        mst_edges.push((cost, from, new_node, path));

        // Update cheapest edges: the new node might offer shorter paths to remaining nodes
        for j in 0..n {
            if in_tree[j] { continue; }
            if let Some((new_cost, new_path)) = astar_world_grid(
                cells, width, height, settlements[new_node].0, settlements[j].0,
            ) {
                let better = match &cheapest[j] {
                    None => true,
                    Some((old_cost, _, _)) => new_cost < *old_cost,
                };
                if better {
                    cheapest[j] = Some((new_cost, new_node, new_path));
                }
            }
        }
    }

    // 3. Prune isolated leaf edges (hamlets/villages with cost > threshold)
    // Build adjacency to find leaves
    let mut degree = vec![0u32; n];
    for (_, from, to, _) in &mst_edges {
        degree[*from] += 1;
        degree[*to] += 1;
    }

    let mut pruned = vec![false; mst_edges.len()];
    for (ei, (cost, from, to, _)) in mst_edges.iter().enumerate() {
        if *cost <= ISOLATION_THRESHOLD { continue; }

        // Check if either endpoint is a leaf hamlet/village
        let from_is_leaf = degree[*from] == 1
            && matches!(settlements[*from].1, SettlementSize::Hamlet | SettlementSize::Village);
        let to_is_leaf = degree[*to] == 1
            && matches!(settlements[*to].1, SettlementSize::Hamlet | SettlementSize::Village);

        if from_is_leaf || to_is_leaf {
            pruned[ei] = true;
            // Decrease degree (for cascading, though we don't re-check — one pass is enough)
            degree[*from] -= 1;
            degree[*to] -= 1;
        }
    }

    // 4. Add bonus edges for towns/cities
    let mut bonus_edges: Vec<(f32, usize, usize, Vec<usize>)> = Vec::new();
    let town_city_indices: Vec<usize> = (0..n)
        .filter(|&i| matches!(settlements[i].1, SettlementSize::Town | SettlementSize::City))
        .collect();

    for &i in &town_city_indices {
        // Find cheapest non-tree edge to another town/city
        let mut candidates: Vec<(f32, usize, Vec<usize>)> = Vec::new();
        for &j in &town_city_indices {
            if i == j { continue; }

            // Skip if already connected by an unpruned MST edge
            let already_connected = mst_edges.iter().enumerate().any(|(ei, (_, from, to, _))| {
                !pruned[ei] && ((*from == i && *to == j) || (*from == j && *to == i))
            });
            if already_connected { continue; }

            if let Some((cost, path)) = astar_world_grid(
                cells, width, height, settlements[i].0, settlements[j].0,
            ) {
                if cost <= BONUS_EDGE_MAX_COST {
                    candidates.push((cost, j, path));
                }
            }
        }

        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Cities get 2 bonus edges, towns get 1
        let max_bonus = if settlements[i].1 == SettlementSize::City { 2 } else { 1 };
        for (cost, j, path) in candidates.into_iter().take(max_bonus) {
            // Avoid duplicate edges (if j already added a bonus to i)
            let already_bonus = bonus_edges.iter().any(|(_, from, to, _)| {
                (*from == i && *to == j) || (*from == j && *to == i)
            });
            if !already_bonus {
                bonus_edges.push((cost, i, j, path));
            }
        }
    }

    // 5. Paint all roads onto cells
    for (ei, (_, from, to, path)) in mst_edges.iter().enumerate() {
        if pruned[ei] { continue; }
        let is_major = matches!(settlements[*from].1, SettlementSize::Town | SettlementSize::City)
            || matches!(settlements[*to].1, SettlementSize::Town | SettlementSize::City);
        let road_class = if is_major { 2u8 } else { 1u8 };
        paint_road(cells, width, height, path, road_class);
    }

    for (_, from, to, path) in &bonus_edges {
        let is_major = matches!(settlements[*from].1, SettlementSize::Town | SettlementSize::City)
            || matches!(settlements[*to].1, SettlementSize::Town | SettlementSize::City);
        let road_class = if is_major { 2u8 } else { 1u8 };
        paint_road(cells, width, height, path, road_class);
    }
}

/// A* pathfinding on the world grid between two cell indices.
/// Returns (total_cost, path_as_cell_indices) or None if unreachable.
fn astar_world_grid(
    cells: &[WorldCell],
    width: u32,
    height: u32,
    start: usize,
    goal: usize,
) -> Option<(f32, Vec<usize>)> {
    let n = cells.len();
    let goal_x = (goal as u32) % width;
    let goal_y = (goal as u32) / width;

    // Heuristic: Manhattan distance
    let heuristic = |idx: usize| -> f32 {
        let x = (idx as u32) % width;
        let y = (idx as u32) / width;
        (x.abs_diff(goal_x) + y.abs_diff(goal_y)) as f32
    };

    let mut g_cost = vec![f32::MAX; n];
    let mut came_from = vec![usize::MAX; n];
    // (f_cost as Reverse for min-heap, index)
    let mut open: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();

    g_cost[start] = 0.0;
    // Pack f32 into u32 for the heap (multiply by 100 for precision)
    let f_start = (heuristic(start) * 100.0) as u32;
    open.push(Reverse((f_start, start)));

    let offsets: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    while let Some(Reverse((_, current))) = open.pop() {
        if current == goal {
            // Reconstruct path
            let mut path = Vec::new();
            let mut node = goal;
            while node != usize::MAX {
                path.push(node);
                node = came_from[node];
            }
            path.reverse();
            return Some((g_cost[goal], path));
        }

        let cx = (current as u32) % width;
        let cy = (current as u32) / width;

        for (dx, dy) in &offsets {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let ni = (ny as u32 * width + nx as u32) as usize;

            let move_cost = road_path_cost(&cells[ni]);
            if move_cost >= f32::MAX { continue; } // impassable

            let tentative_g = g_cost[current] + move_cost;
            if tentative_g < g_cost[ni] {
                g_cost[ni] = tentative_g;
                came_from[ni] = current;
                let f = ((tentative_g + heuristic(ni)) * 100.0) as u32;
                open.push(Reverse((f, ni)));
            }
        }
    }

    None // unreachable
}

/// Movement cost for road pathfinding across the world grid.
fn road_path_cost(cell: &WorldCell) -> f32 {
    // Ocean is impassable
    if cell.zone_type == ZoneType::Ocean {
        return f32::MAX;
    }

    // Already-roaded cells are very cheap (encourages merging)
    if cell.road {
        return 0.1;
    }

    // River cells are expensive — roads should go around, not through
    let river_penalty = if cell.river { 6.0 } else { 0.0 };

    let base = match cell.zone_type {
        ZoneType::Ocean => unreachable!(),
        ZoneType::Mountain => 8.0,
        ZoneType::Swamp => 3.0,
        ZoneType::Forest => 2.0,
        ZoneType::Tundra => 2.0,
        ZoneType::Desert => 2.5,
        ZoneType::Coast => 1.5,
        ZoneType::Grassland => 1.0,
        ZoneType::Settlement => 0.5,
    };

    base + river_penalty
}

/// Paint a road path onto cells: set road, road_entry, road_class.
fn paint_road(
    cells: &mut [WorldCell],
    width: u32,
    _height: u32,
    path: &[usize],
    road_class: u8,
) {
    for (step, &idx) in path.iter().enumerate() {
        // Never paint road on ocean cells
        if cells[idx].zone_type == ZoneType::Ocean {
            continue;
        }
        cells[idx].road = true;
        if road_class > cells[idx].road_class {
            cells[idx].road_class = road_class;
        }

        // Set entry/exit edges based on transitions
        if step > 0 {
            let prev = path[step - 1];
            let prev_x = (prev as u32) % width;
            let prev_y = (prev as u32) / width;
            let cur_x = (idx as u32) % width;
            let cur_y = (idx as u32) / width;

            // Determine which edge was crossed
            if cur_y > prev_y {
                // Moved north: prev exits N (0), cur enters S (2)
                cells[prev].road_entry[0] = true;
                cells[idx].road_entry[2] = true;
            } else if cur_y < prev_y {
                // Moved south: prev exits S (2), cur enters N (0)
                cells[prev].road_entry[2] = true;
                cells[idx].road_entry[0] = true;
            }
            if cur_x > prev_x {
                // Moved east: prev exits E (1), cur enters W (3)
                cells[prev].road_entry[1] = true;
                cells[idx].road_entry[3] = true;
            } else if cur_x < prev_x {
                // Moved west: prev exits W (3), cur enters E (1)
                cells[prev].road_entry[3] = true;
                cells[idx].road_entry[1] = true;
            }
        }
    }
}
