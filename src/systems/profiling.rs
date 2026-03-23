use std::time::Instant;
use bevy::prelude::*;

use crate::systems::debug::DebugFlags;

/// Tracks frame timing for profiling.
#[derive(Resource)]
pub struct FrameProfiler {
    last_report: Instant,
    frame_count: u32,
    total_time_ms: f64,
    max_time_ms: f64,
}

impl Default for FrameProfiler {
    fn default() -> Self {
        Self {
            last_report: Instant::now(),
            frame_count: 0,
            total_time_ms: 0.0,
            max_time_ms: 0.0,
        }
    }
}

pub fn frame_profiler(
    flags: Res<DebugFlags>,
    mut profiler: ResMut<FrameProfiler>,
    time: Res<Time>,
) {
    if !flags.profiling { return; }

    let dt_ms = time.delta_secs_f64() * 1000.0;
    profiler.frame_count += 1;
    profiler.total_time_ms += dt_ms;
    if dt_ms > profiler.max_time_ms {
        profiler.max_time_ms = dt_ms;
    }

    if profiler.last_report.elapsed().as_secs_f64() >= 2.0 {
        let avg = profiler.total_time_ms / profiler.frame_count as f64;
        let fps = 1000.0 / avg;
        info!(
            "PROFILE: {:.0} FPS | avg {:.2}ms | max {:.2}ms | {} frames",
            fps, avg, profiler.max_time_ms, profiler.frame_count
        );
        profiler.frame_count = 0;
        profiler.total_time_ms = 0.0;
        profiler.max_time_ms = 0.0;
        profiler.last_report = Instant::now();
    }
}

pub fn entity_counter(
    flags: Res<DebugFlags>,
    all: Query<Entity>,
    with_body: Query<Entity, With<crate::resources::body::Body>>,
    with_path: Query<Entity, With<crate::resources::movement::MovePath>>,
    with_text: Query<Entity, With<Text2d>>,
    ui_nodes: Query<Entity, With<Node>>,
) {
    if !flags.profiling { return; }

    static mut COUNTER: u32 = 0;
    unsafe {
        COUNTER += 1;
        if COUNTER % 120 == 0 {
            info!(
                "ENTITIES: {} total | {} with Body | {} moving | {} text2d | {} UI nodes",
                all.iter().count(),
                with_body.iter().count(),
                with_path.iter().count(),
                with_text.iter().count(),
                ui_nodes.iter().count(),
            );
        }
    }
}
