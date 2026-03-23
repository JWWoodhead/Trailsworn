use trailsworn::resources::game_time::{GameTime, TICK_DURATION};

#[test]
fn paused_produces_zero_ticks() {
    let mut gt = GameTime::default();
    gt.paused = true;
    assert_eq!(gt.accumulate(1.0), 0);
}

#[test]
fn normal_frame_produces_expected_ticks() {
    let mut gt = GameTime::default();
    // A normal 60fps frame should produce 1 tick
    let ticks = gt.accumulate(TICK_DURATION);
    assert_eq!(ticks, 1);
}

#[test]
fn large_dt_capped_at_10() {
    let mut gt = GameTime::default();
    // 1 second frame = 60 ticks, but capped at 10
    let ticks = gt.accumulate(1.0);
    assert_eq!(ticks, 10);
}

#[test]
fn small_dt_accumulates() {
    let mut gt = GameTime::default();
    // Half a tick's worth of time — should produce 0 ticks
    let ticks = gt.accumulate(TICK_DURATION * 0.5);
    assert_eq!(ticks, 0);
    // Another half — should now produce 1 tick
    let ticks = gt.accumulate(TICK_DURATION * 0.5);
    assert_eq!(ticks, 1);
}

#[test]
fn speed_3x_triples_ticks() {
    let mut gt = GameTime::default();
    gt.speed = 3.0;
    let ticks = gt.accumulate(TICK_DURATION);
    assert_eq!(ticks, 3);
}

#[test]
fn interpolation_alpha_between_0_and_1() {
    let mut gt = GameTime::default();
    gt.accumulate(TICK_DURATION * 0.5);
    let alpha = gt.interpolation_alpha();
    assert!(alpha > 0.4 && alpha < 0.6, "alpha was {alpha}");
}
