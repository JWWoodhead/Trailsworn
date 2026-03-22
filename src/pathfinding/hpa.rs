use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

use super::astar::{astar_tile_grid, astar_tile_grid_bounded, tile_path_cost};

pub type HpaNodeId = u32;

/// Default cluster size in tiles. Adjustable via [`HpaGraphBuilder::cluster_size`].
const DEFAULT_CLUSTER_SIZE: u32 = 10;

#[derive(Clone, Debug)]
pub struct Cluster {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Debug)]
pub struct HpaNode {
    pub id: HpaNodeId,
    pub tx: u32,
    pub ty: u32,
    pub cluster_idx: usize,
    pub edges: Vec<(HpaNodeId, f32)>,
}

#[derive(Clone, Debug, Default)]
pub struct HpaGraph {
    pub clusters: Vec<Cluster>,
    pub nodes: Vec<HpaNode>,
    pub cluster_width: u32,
    pub cluster_height: u32,
    /// Per-tile: (nearest entrance node id, cost). `u32::MAX` = unreachable.
    nearest_entrance: Vec<(HpaNodeId, f32)>,
    grid_width: u32,
    grid_height: u32,
}

/// Builder for constructing an [`HpaGraph`] from a walkability grid.
pub struct HpaGraphBuilder<'a> {
    walk_cost: &'a [f32],
    width: u32,
    height: u32,
    cluster_size: u32,
    bounded_max_expansions: u32,
}

impl<'a> HpaGraphBuilder<'a> {
    /// Create a new builder.
    ///
    /// `walk_cost` is a row-major grid of per-tile costs (0.0 = impassable, >0 = cost multiplier).
    /// Must have length `width * height`.
    pub fn new(walk_cost: &'a [f32], width: u32, height: u32) -> Self {
        debug_assert_eq!(walk_cost.len(), (width * height) as usize);
        Self {
            walk_cost,
            width,
            height,
            cluster_size: DEFAULT_CLUSTER_SIZE,
            bounded_max_expansions: 500,
        }
    }

    /// Set the cluster size in tiles (default: 10).
    pub fn cluster_size(mut self, size: u32) -> Self {
        self.cluster_size = size;
        self
    }

    /// Set max expansions for bounded intra-cluster A* (default: 500).
    pub fn bounded_max_expansions(mut self, max: u32) -> Self {
        self.bounded_max_expansions = max;
        self
    }

    /// Build the HPA* graph.
    pub fn build(self) -> HpaGraph {
        let cs = self.cluster_size;
        let w = self.width;
        let h = self.height;

        // 1. Cluster decomposition
        let cw = (w + cs - 1) / cs;
        let ch = (h + cs - 1) / cs;
        let mut clusters = Vec::new();
        for cy in 0..ch {
            for cx in 0..cw {
                let x = cx * cs;
                let y = cy * cs;
                let cwidth = cs.min(w - x);
                let cheight = cs.min(h - y);
                clusters.push(Cluster { x, y, w: cwidth, h: cheight });
            }
        }

        let mut nodes: Vec<HpaNode> = Vec::new();
        let mut node_id_counter: HpaNodeId = 0;
        let mut tile_to_node: HashMap<(u32, u32), HpaNodeId> = HashMap::new();

        // 2. Entrance finding: scan shared borders between adjacent clusters
        // Horizontal borders
        for cy in 0..ch - 1 {
            for cx in 0..cw {
                let top_idx = (cy * cw + cx) as usize;
                let bot_idx = ((cy + 1) * cw + cx) as usize;
                let border_y_top = clusters[top_idx].y + clusters[top_idx].h - 1;
                let border_y_bot = clusters[bot_idx].y;
                let x_start = clusters[top_idx].x;
                let x_end = x_start + clusters[top_idx].w;

                let mut run_start: Option<u32> = None;
                for x in x_start..=x_end {
                    let walkable = x < x_end
                        && self.walk_cost[(border_y_top * w + x) as usize] > 0.0
                        && self.walk_cost[(border_y_bot * w + x) as usize] > 0.0;
                    if walkable {
                        if run_start.is_none() {
                            run_start = Some(x);
                        }
                    } else if let Some(rs) = run_start {
                        emit_entrance_nodes(
                            &mut nodes,
                            &mut node_id_counter,
                            &mut tile_to_node,
                            rs,
                            x - 1,
                            border_y_top,
                            border_y_bot,
                            true,
                            top_idx,
                            bot_idx,
                        );
                        run_start = None;
                    }
                }
            }
        }

        // Vertical borders
        for cy in 0..ch {
            for cx in 0..cw - 1 {
                let left_idx = (cy * cw + cx) as usize;
                let right_idx = (cy * cw + cx + 1) as usize;
                let border_x_left = clusters[left_idx].x + clusters[left_idx].w - 1;
                let border_x_right = clusters[right_idx].x;
                let y_start = clusters[left_idx].y;
                let y_end = y_start + clusters[left_idx].h;

                let mut run_start: Option<u32> = None;
                for y in y_start..=y_end {
                    let walkable = y < y_end
                        && self.walk_cost[(y * w + border_x_left) as usize] > 0.0
                        && self.walk_cost[(y * w + border_x_right) as usize] > 0.0;
                    if walkable {
                        if run_start.is_none() {
                            run_start = Some(y);
                        }
                    } else if let Some(rs) = run_start {
                        emit_entrance_nodes(
                            &mut nodes,
                            &mut node_id_counter,
                            &mut tile_to_node,
                            rs,
                            y - 1,
                            border_x_left,
                            border_x_right,
                            false,
                            left_idx,
                            right_idx,
                        );
                        run_start = None;
                    }
                }
            }
        }

        // 3. Intra-cluster edges: A* between every pair of entrance nodes within each cluster
        let mut cluster_nodes: Vec<Vec<HpaNodeId>> = vec![Vec::new(); clusters.len()];
        for node in &nodes {
            cluster_nodes[node.cluster_idx].push(node.id);
        }

        for (ci, node_ids) in cluster_nodes.iter().enumerate() {
            let c = &clusters[ci];
            for i in 0..node_ids.len() {
                for j in (i + 1)..node_ids.len() {
                    let ni = node_ids[i];
                    let nj = node_ids[j];
                    let from = (nodes[ni as usize].tx, nodes[ni as usize].ty);
                    let to = (nodes[nj as usize].tx, nodes[nj as usize].ty);
                    if let Some(path) = astar_tile_grid_bounded(
                        from,
                        to,
                        self.walk_cost,
                        w,
                        h,
                        c.x,
                        c.y,
                        c.w,
                        c.h,
                        self.bounded_max_expansions,
                    ) {
                        let cost = tile_path_cost(&path);
                        nodes[ni as usize].edges.push((nj, cost));
                        nodes[nj as usize].edges.push((ni, cost));
                    }
                }
            }
        }

        // 4. Multi-source BFS: for each walkable tile, find nearest entrance node
        let grid_size = (w * h) as usize;
        let mut nearest = vec![(u32::MAX, f32::INFINITY); grid_size];
        let mut queue = VecDeque::new();

        for node in &nodes {
            let idx = (node.ty * w + node.tx) as usize;
            if self.walk_cost[idx] > 0.0 {
                nearest[idx] = (node.id, 0.0);
                queue.push_back((node.tx, node.ty));
            }
        }

        while let Some((x, y)) = queue.pop_front() {
            let ci = (y * w + x) as usize;
            let (nid, cost) = nearest[ci];
            for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let ni = (ny as u32 * w + nx as u32) as usize;
                if self.walk_cost[ni] <= 0.0 {
                    continue;
                }
                let new_cost = cost + self.walk_cost[ni];
                if new_cost < nearest[ni].1 {
                    nearest[ni] = (nid, new_cost);
                    queue.push_back((nx as u32, ny as u32));
                }
            }
        }

        HpaGraph {
            clusters,
            nodes,
            cluster_width: cw,
            cluster_height: ch,
            nearest_entrance: nearest,
            grid_width: w,
            grid_height: h,
        }
    }
}

impl HpaGraph {
    /// Find the cluster index that contains tile (tx, ty).
    pub fn cluster_for_tile(&self, tx: u32, ty: u32) -> usize {
        let cs = if !self.clusters.is_empty() {
            self.clusters[0].w.max(1)
        } else {
            DEFAULT_CLUSTER_SIZE
        };
        let cx = (tx / cs).min(self.cluster_width.saturating_sub(1));
        let cy = (ty / cs).min(self.cluster_height.saturating_sub(1));
        (cy * self.cluster_width + cx) as usize
    }

    /// HPA* path query.
    ///
    /// Uses precomputed nearest-entrance lookup, then Dijkstra on the abstract graph,
    /// then tile-level A* refinement between waypoints.
    pub fn find_path(
        &self,
        start_tile: (u32, u32),
        goal_tile: (u32, u32),
        walk_cost: &[f32],
    ) -> Option<Vec<(u32, u32)>> {
        let width = self.grid_width;
        let height = self.grid_height;

        if self.nodes.is_empty() {
            return None;
        }

        let si = (start_tile.1 * width + start_tile.0) as usize;
        let gi = (goal_tile.1 * width + goal_tile.0) as usize;
        let (start_entrance, _start_cost) = self.nearest_entrance[si];
        let (goal_entrance, _goal_cost) = self.nearest_entrance[gi];

        // Unreachable from any entrance — try direct A* as fallback
        if start_entrance == u32::MAX || goal_entrance == u32::MAX {
            return astar_tile_grid(start_tile, goal_tile, width, height, walk_cost, 2000);
        }

        // Same entrance or adjacent: just do direct tile A*
        if start_entrance == goal_entrance {
            return astar_tile_grid(start_tile, goal_tile, width, height, walk_cost, 2000);
        }

        // Dijkstra on abstract graph
        let n = self.nodes.len();
        let mut dist = vec![f32::INFINITY; n];
        let mut prev = vec![u32::MAX; n];
        let mut heap: BinaryHeap<Reverse<(u32, HpaNodeId)>> = BinaryHeap::new();

        dist[start_entrance as usize] = 0.0;
        heap.push(Reverse((0.0f32.to_bits(), start_entrance)));

        while let Some(Reverse((d_bits, u))) = heap.pop() {
            if u == goal_entrance {
                break;
            }
            let du = f32::from_bits(d_bits);
            if du > dist[u as usize] {
                continue;
            }
            for &(v, w) in &self.nodes[u as usize].edges {
                let nd = du + w;
                if nd < dist[v as usize] {
                    dist[v as usize] = nd;
                    prev[v as usize] = u;
                    heap.push(Reverse((nd.to_bits(), v)));
                }
            }
        }

        if dist[goal_entrance as usize].is_infinite() {
            return astar_tile_grid(start_tile, goal_tile, width, height, walk_cost, 4000);
        }

        // Reconstruct abstract path
        let mut abstract_nodes = vec![goal_entrance];
        let mut cur = goal_entrance;
        while cur != start_entrance {
            cur = prev[cur as usize];
            if cur == u32::MAX {
                return None;
            }
            abstract_nodes.push(cur);
        }
        abstract_nodes.reverse();

        // Build waypoint sequence: start -> entrance nodes -> goal
        let mut waypoints: Vec<(u32, u32)> = Vec::new();
        waypoints.push(start_tile);
        for &nid in &abstract_nodes {
            let nd = &self.nodes[nid as usize];
            waypoints.push((nd.tx, nd.ty));
        }
        waypoints.push(goal_tile);

        // Refine: tile A* between consecutive waypoints
        let mut full_path: Vec<(u32, u32)> = Vec::new();
        for pair in waypoints.windows(2) {
            if pair[0] == pair[1] {
                if full_path.last() != Some(&pair[0]) {
                    full_path.push(pair[0]);
                }
                continue;
            }
            let segment = astar_tile_grid(pair[0], pair[1], width, height, walk_cost, 2000)?;
            for &tile in &segment {
                if full_path.last() != Some(&tile) {
                    full_path.push(tile);
                }
            }
        }

        if full_path.len() < 2 {
            return None;
        }
        Some(full_path)
    }

    /// BFS from the most-connected node. Returns a vec where `true` = orphan (unreachable).
    pub fn find_orphans(&self) -> Vec<bool> {
        let n = self.nodes.len();
        let mut visited = vec![false; n];
        if n == 0 {
            return visited;
        }

        let start = self
            .nodes
            .iter()
            .max_by_key(|nd| nd.edges.len())
            .map(|nd| nd.id as usize)
            .unwrap_or(0);
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        while let Some(u) = queue.pop_front() {
            for &(v, _) in &self.nodes[u].edges {
                let vi = v as usize;
                if !visited[vi] {
                    visited[vi] = true;
                    queue.push_back(vi);
                }
            }
        }

        let mut orphans = vec![false; n];
        for i in 0..n {
            if !visited[i] {
                orphans[i] = true;
            }
        }
        orphans
    }
}

/// Emit entrance nodes for a contiguous walkable run along a cluster border.
fn emit_entrance_nodes(
    nodes: &mut Vec<HpaNode>,
    counter: &mut HpaNodeId,
    tile_to_node: &mut HashMap<(u32, u32), HpaNodeId>,
    run_start: u32,
    run_end: u32,
    border_a: u32,
    border_b: u32,
    horizontal: bool,
    cluster_a: usize,
    cluster_b: usize,
) {
    let add = |nodes: &mut Vec<HpaNode>,
               counter: &mut HpaNodeId,
               tile_to_node: &mut HashMap<(u32, u32), HpaNodeId>,
               tx: u32,
               ty: u32,
               ci: usize|
     -> HpaNodeId {
        if let Some(&existing) = tile_to_node.get(&(tx, ty)) {
            return existing;
        }
        let id = *counter;
        *counter += 1;
        nodes.push(HpaNode { id, tx, ty, cluster_idx: ci, edges: Vec::new() });
        tile_to_node.insert((tx, ty), id);
        id
    };

    for p in run_start..=run_end {
        let (ta, tb) = if horizontal { ((p, border_a), (p, border_b)) } else { ((border_a, p), (border_b, p)) };
        let na = add(nodes, counter, tile_to_node, ta.0, ta.1, cluster_a);
        let nb = add(nodes, counter, tile_to_node, tb.0, tb.1, cluster_b);
        nodes[na as usize].edges.push((nb, 1.0));
        nodes[nb as usize].edges.push((na, 1.0));
    }
}

