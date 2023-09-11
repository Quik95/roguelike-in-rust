use itertools::Itertools;
use rltk::{BaseMap, NavigationPath};
use std::time::Instant;

pub fn a_star_search(start: usize, end: usize, map: &dyn BaseMap) -> NavigationPath {
    let res = pathfinding::directed::astar::astar(
        &start,
        |&p| {
            map.get_available_exits(p)
                .iter()
                .map(|(exit, cost)| (*exit, *cost as u32))
                .collect_vec()
        },
        |&p| map.get_pathing_distance(p, end) as u32,
        |&p| p == end,
    );

    if let Some(p) = res {
        return NavigationPath {
            destination: end,
            success: true,
            steps: p.0,
        };
    }

    NavigationPath {
        destination: end,
        success: false,
        steps: vec![],
    }
}
