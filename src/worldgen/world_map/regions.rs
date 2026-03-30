use super::WorldCell;
use crate::worldgen::zone::ZoneType;

/// Flood-fill contiguous same-type land zones to assign region_id.
pub(super) fn assign_regions(cells: &mut [WorldCell], width: u32, height: u32) {
    let n = cells.len();
    let mut region_counter = 0u32;
    let offsets: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    for start in 0..n {
        if cells[start].zone_type == ZoneType::Ocean || cells[start].region_id.is_some() {
            continue;
        }

        let zone_type = cells[start].zone_type;
        let rid = region_counter;
        region_counter += 1;

        let mut stack = vec![start];
        while let Some(ci) = stack.pop() {
            if cells[ci].region_id.is_some() {
                continue;
            }
            if cells[ci].zone_type != zone_type {
                continue;
            }
            cells[ci].region_id = Some(rid);

            let cx = (ci as u32) % width;
            let cy = (ci as u32) / width;
            for (dx, dy) in &offsets {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                    let ni = (ny as u32 * width + nx as u32) as usize;
                    if cells[ni].region_id.is_none() && cells[ni].zone_type == zone_type {
                        stack.push(ni);
                    }
                }
            }
        }
    }
}
