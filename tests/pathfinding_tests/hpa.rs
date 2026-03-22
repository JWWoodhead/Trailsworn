use gold_and_glory::pathfinding::HpaGraphBuilder;

fn open_grid(w: u32, h: u32) -> Vec<f32> {
    vec![1.0; (w * h) as usize]
}

#[test]
fn build_and_query_open_grid() {
    let w = 50;
    let h = 50;
    let cost = open_grid(w, h);
    let graph = HpaGraphBuilder::new(&cost, w, h).build();

    assert!(!graph.nodes.is_empty());
    assert!(!graph.clusters.is_empty());

    let path = graph.find_path((0, 0), (49, 49), &cost).unwrap();
    assert_eq!(path.first(), Some(&(0, 0)));
    assert_eq!(path.last(), Some(&(49, 49)));
}

#[test]
fn query_same_cluster() {
    let w = 50;
    let h = 50;
    let cost = open_grid(w, h);
    let graph = HpaGraphBuilder::new(&cost, w, h).build();

    let path = graph.find_path((1, 1), (5, 5), &cost).unwrap();
    assert_eq!(path.first(), Some(&(1, 1)));
    assert_eq!(path.last(), Some(&(5, 5)));
}

#[test]
fn wall_blocks_path() {
    let w = 30;
    let h = 30;
    let mut cost = open_grid(w, h);
    for x in 0..w {
        cost[(15 * w + x) as usize] = 0.0;
    }
    let graph = HpaGraphBuilder::new(&cost, w, h).build();
    assert!(graph.find_path((5, 0), (5, 29), &cost).is_none());
}

#[test]
fn wall_with_gap() {
    let w = 30;
    let h = 30;
    let mut cost = open_grid(w, h);
    for x in 0..w {
        if x != 15 {
            cost[(15 * w + x) as usize] = 0.0;
        }
    }
    let graph = HpaGraphBuilder::new(&cost, w, h).build();
    let path = graph.find_path((5, 0), (5, 29), &cost).unwrap();
    assert_eq!(path.first(), Some(&(5, 0)));
    assert_eq!(path.last(), Some(&(5, 29)));
}

#[test]
fn custom_cluster_size() {
    let w = 40;
    let h = 40;
    let cost = open_grid(w, h);
    let graph = HpaGraphBuilder::new(&cost, w, h).cluster_size(20).build();

    assert_eq!(graph.cluster_width, 2);
    assert_eq!(graph.cluster_height, 2);

    let path = graph.find_path((0, 0), (39, 39), &cost).unwrap();
    assert_eq!(path.first(), Some(&(0, 0)));
    assert_eq!(path.last(), Some(&(39, 39)));
}

#[test]
fn orphan_detection() {
    let w = 30;
    let h = 30;
    let cost = open_grid(w, h);
    let graph = HpaGraphBuilder::new(&cost, w, h).build();
    let orphans = graph.find_orphans();
    assert!(orphans.iter().all(|&o| !o));
}
