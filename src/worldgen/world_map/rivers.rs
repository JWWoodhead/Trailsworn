use rand::{Rng, RngExt};

use crate::worldgen::noise_util::NoiseLayer;

/// Generate rivers by walking downhill from high-elevation sources to ocean.
pub(super) fn generate_rivers(
    elevation: &[f32],
    river: &mut [bool],
    river_progress: &mut [f32],
    river_edges: &mut [[bool; 4]],
    moisture: &mut [f32],
    width: u32,
    height: u32,
    ocean_threshold: f32,
    rng: &mut impl Rng,
) {
    let n = (width * height) as usize;
    let num_rivers = (width as usize * height as usize / 4000).clamp(5, 40);

    // Collect candidate source cells (elevated land — mountains and hills)
    let mut candidates: Vec<usize> = (0..n)
        .filter(|&i| elevation[i] > 0.52 && elevation[i] < 0.90)
        .collect();

    // Shuffle and pick sources, ensuring minimum spacing between river starts
    for i in (1..candidates.len()).rev() {
        let j = rng.random_range(0..=i);
        candidates.swap(i, j);
    }

    let min_source_dist_sq = (width.min(height) / 10).pow(2) as u32;
    let mut sources: Vec<(u32, u32)> = Vec::new();

    for &c in &candidates {
        if sources.len() >= num_rivers {
            break;
        }
        let cx = (c as u32) % width;
        let cy = (c as u32) / width;
        let too_close = sources
            .iter()
            .any(|(sx, sy)| cx.abs_diff(*sx).pow(2) + cy.abs_diff(*sy).pow(2) < min_source_dist_sq);
        if too_close {
            continue;
        }
        sources.push((cx, cy));
        walk_river(c, elevation, river, river_progress, river_edges, moisture, width, height, ocean_threshold);
    }
}

/// Walk a single river downhill from source until reaching ocean or map edge.
///
/// Traces a smooth float-coordinate path guided by elevation gradient + noise meander,
/// then rasterizes it onto the grid with increasing width. This avoids the staircase
/// artifacts of grid-cell-by-cell walking.
fn walk_river(
    start: usize,
    elevation: &[f32],
    river: &mut [bool],
    river_progress: &mut [f32],
    river_edges: &mut [[bool; 4]],
    moisture: &mut [f32],
    width: u32,
    height: u32,
    ocean_threshold: f32,
) {
    let meander = NoiseLayer::new(start as u32, 0.05, 3);

    // Start at the center of the source cell
    let mut fx = ((start as u32) % width) as f64 + 0.5;
    let mut fy = ((start as u32) / width) as f64 + 0.5;
    let max_steps = (width + height) as usize * 4;
    let step_size = 0.8;

    // Track previous direction for momentum when gradient is flat
    let mut prev_dx = 0.0f64;
    let mut prev_dy = -1.0; // default: flow "south" (toward y=0)

    let mut path: Vec<(f64, f64)> = Vec::new();

    for step in 0..max_steps {
        let ix = (fx as i32).clamp(0, width as i32 - 1) as u32;
        let iy = (fy as i32).clamp(0, height as i32 - 1) as u32;
        let idx = (iy * width + ix) as usize;

        path.push((fx, fy));

        // Reached ocean — done
        if elevation[idx] < ocean_threshold {
            break;
        }

        // Compute downhill gradient from neighboring cells (wide radius to see past flat areas)
        let mut grad_x = 0.0f64;
        let mut grad_y = 0.0f64;
        let sample_r = 4.0;
        for &(dx, dy) in &[(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0),
                           (0.7, 0.7), (0.7, -0.7), (-0.7, 0.7), (-0.7, -0.7)] {
            let sx = (fx + dx * sample_r).clamp(0.0, width as f64 - 1.0);
            let sy = (fy + dy * sample_r).clamp(0.0, height as f64 - 1.0);
            let si = (sy as u32 * width + sx as u32) as usize;
            let diff = elevation[idx] as f64 - elevation[si] as f64;
            grad_x += dx * diff;
            grad_y += dy * diff;
        }

        // Normalize gradient; if flat, rely more on momentum
        let grad_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
        if grad_len > 1e-10 {
            grad_x /= grad_len;
            grad_y /= grad_len;
        } else {
            grad_x = prev_dx;
            grad_y = prev_dy;
        }

        // Blend with previous direction: heavy momentum keeps rivers flowing through flats
        let momentum = 0.65;
        grad_x = grad_x * (1.0 - momentum) + prev_dx * momentum;
        grad_y = grad_y * (1.0 - momentum) + prev_dy * momentum;
        let blend_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
        if blend_len > 1e-8 {
            grad_x /= blend_len;
            grad_y /= blend_len;
        }

        // Add meander: perpendicular oscillation via noise
        let perp_x = -grad_y;
        let perp_y = grad_x;
        let meander_strength = meander.sample(step as f64 * 0.05, fx * 0.01) * 0.6;
        let dir_x = grad_x + perp_x * meander_strength;
        let dir_y = grad_y + perp_y * meander_strength;

        // Normalize and step
        let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
        let ndx = dir_x / dir_len;
        let ndy = dir_y / dir_len;
        fx += ndx * step_size;
        fy += ndy * step_size;
        prev_dx = ndx;
        prev_dy = ndy;

        // Out of bounds — done
        if fx < 0.0 || fy < 0.0 || fx >= width as f64 || fy >= height as f64 {
            break;
        }
    }

    // Rasterize the smooth path with increasing width + track edge crossings
    let path_len = path.len() as f32;
    let mut prev_cell: Option<(u32, u32)> = None;
    // Subsample path to avoid over-painting (take every ~2 points)
    let paint_step = 2.max(1);
    for (step, &(px, py)) in path.iter().enumerate().step_by(paint_step) {
        let progress = step as f32 / path_len;
        let half_width = (0.8 + progress * 2.2) as i32; // 1 at source, ~3 at mouth

        let cx = px as i32;
        let cy = py as i32;
        for dx in -half_width..=half_width {
            for dy in -half_width..=half_width {
                if dx * dx + dy * dy > half_width * half_width + 1 {
                    continue;
                }
                let rx = cx + dx;
                let ry = cy + dy;
                if rx >= 0 && ry >= 0 && rx < width as i32 && ry < height as i32 {
                    let ri = (ry as u32 * width + rx as u32) as usize;
                    river[ri] = true;
                    // Track maximum progress (width) for each cell
                    if progress > river_progress[ri] {
                        river_progress[ri] = progress;
                    }
                }
            }
        }

        // Track edge crossings between world cells
        let cur_cell = (cx.max(0) as u32, cy.max(0) as u32);
        if let Some(prev) = prev_cell {
            if cur_cell != prev {
                let pi = (prev.1 * width + prev.0) as usize;
                let ci = (cur_cell.1.min(height - 1) * width + cur_cell.0.min(width - 1)) as usize;
                // Determine which edge was crossed: N=+y, S=-y, E=+x, W=-x
                if cur_cell.1 > prev.1 {
                    // Moved north: prev exits N, cur enters S
                    if pi < river_edges.len() { river_edges[pi][0] = true; }
                    if ci < river_edges.len() { river_edges[ci][2] = true; }
                } else if cur_cell.1 < prev.1 {
                    if pi < river_edges.len() { river_edges[pi][2] = true; }
                    if ci < river_edges.len() { river_edges[ci][0] = true; }
                }
                if cur_cell.0 > prev.0 {
                    if pi < river_edges.len() { river_edges[pi][1] = true; }
                    if ci < river_edges.len() { river_edges[ci][3] = true; }
                } else if cur_cell.0 < prev.0 {
                    if pi < river_edges.len() { river_edges[pi][3] = true; }
                    if ci < river_edges.len() { river_edges[ci][1] = true; }
                }
            }
        }
        prev_cell = Some(cur_cell);

        // Moisture boost
        let moist_r = half_width + 2;
        for dx in -moist_r..=moist_r {
            for dy in -moist_r..=moist_r {
                let mx = cx + dx;
                let my = cy + dy;
                if mx >= 0 && my >= 0 && mx < width as i32 && my < height as i32 {
                    let mi = (my as u32 * width + mx as u32) as usize;
                    moisture[mi] = (moisture[mi] + 0.15).min(1.0);
                }
            }
        }
    }
}
