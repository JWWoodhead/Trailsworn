use gold_and_glory::resources::movement::MovePath;

#[test]
fn new_path_starts_at_zero() {
    let path = MovePath::new(vec![(0, 0), (1, 0), (2, 0)]);
    assert_eq!(path.current_index, 0);
    assert_eq!(path.progress, 0.0);
}

#[test]
fn current_and_next_tile() {
    let path = MovePath::new(vec![(0, 0), (1, 0), (2, 0)]);
    assert_eq!(path.current_tile(), Some((0, 0)));
    assert_eq!(path.next_tile(), Some((1, 0)));
}

#[test]
fn advance_moves_to_next() {
    let mut path = MovePath::new(vec![(0, 0), (1, 0), (2, 0)]);
    path.advance();
    assert_eq!(path.current_tile(), Some((1, 0)));
    assert_eq!(path.next_tile(), Some((2, 0)));
    assert!(!path.is_finished());
}

#[test]
fn finished_at_last_tile() {
    let mut path = MovePath::new(vec![(0, 0), (1, 0)]);
    path.advance();
    assert!(path.is_finished());
}

#[test]
fn single_tile_path_is_finished() {
    let path = MovePath::new(vec![(5, 5)]);
    assert!(path.is_finished());
}
