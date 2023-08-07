use std::collections::HashSet;

use rltk::console;

use crate::map::{Map, TileType};
use crate::map_builders::waveform_collapse::common::{MapChunk, tile_idx_in_chunk};

pub fn build_patterns(map: &Map, chunk_size: i32, include_flipping: bool, dedupe: bool) -> Vec<Vec<TileType>> {
    let chunks_x = map.width / chunk_size;
    let chunks_y = map.height / chunk_size;
    let mut patterns = Vec::new();

    for cy in 0..chunks_y {
        for cx in 0..chunks_x {
            let mut pattern = Vec::new();
            let start_x = cx * chunk_size;
            let end_x = (cx + 1) * chunk_size;
            let start_y = cy * chunk_size;
            let end_y = (cy + 1) * chunk_size;

            for y in start_y..end_y {
                for x in start_x..end_x {
                    let idx = Map::xy_idx(x, y);
                    pattern.push(map.tiles[idx]);
                }
            }
            patterns.push(pattern);

            if include_flipping {
                pattern = Vec::new();
                for y in start_y..end_y {
                    for x in start_x..end_x {
                        let idx = Map::xy_idx(end_x - (x + 1), y);
                        pattern.push(map.tiles[idx]);
                    }
                }
                patterns.push(pattern);

                pattern = Vec::new();
                for y in start_y..end_y {
                    for x in start_x..end_x {
                        let idx = Map::xy_idx(x, end_y - (y + 1));
                        pattern.push(map.tiles[idx]);
                    }
                }
                patterns.push(pattern);

                pattern = Vec::new();
                for y in start_y..end_y {
                    for x in start_x..end_x {
                        let idx = Map::xy_idx(end_x - (x + 1), end_y - (y + 1));
                        pattern.push(map.tiles[idx]);
                    }
                }
                patterns.push(pattern);
            }
        }
    }

    if dedupe {
        console::log(format!("Pre de-duplication, there are {} patterns", patterns.len()));
        let set: HashSet<Vec<TileType>> = patterns.drain(..).collect();
        patterns.extend(set.into_iter());
        console::log(format!("Post de-duplication, there are {} patterns", patterns.len()));
    }

    patterns
}

pub fn render_pattern_to_map(map: &mut Map, chunk: &MapChunk, chunk_size: i32, start_x: i32, start_y: i32) {
    let mut i = 0usize;
    for tile_y in 0..chunk_size {
        for tile_x in 0..chunk_size {
            let map_idx = Map::xy_idx(start_x + tile_x, start_y + tile_y);
            map.tiles[map_idx] = chunk.pattern[i];
            map.visible_tiles[map_idx] = true;
            i += 1;
        }
    }
}

pub fn patterns_to_constraints(patterns: Vec<Vec<TileType>>, chunk_size: i32) -> Vec<MapChunk> {
    let mut constraints: Vec<MapChunk> = Vec::new();
    for p in patterns {
        let mut new_chunk = MapChunk {
            pattern: p,
            exists: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            has_exits: true,
            compatible_with: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        };
        for exit in new_chunk.exists.iter_mut() {
            for _i in 0..chunk_size {
                exit.push(false)
            }
        }

        let mut n_exits = 0;
        for x in 0..chunk_size {
            let north_idx = tile_idx_in_chunk(chunk_size, x, 0);
            if new_chunk.pattern[north_idx] == TileType::Floor {
                new_chunk.exists[0][x as usize] = true;
                n_exits += 1;
            }

            let south_idx = tile_idx_in_chunk(chunk_size, x, chunk_size - 1);
            if new_chunk.pattern[south_idx] == TileType::Floor {
                new_chunk.exists[1][x as usize] = true;
                n_exits += 1;
            }

            let west_idx = tile_idx_in_chunk(chunk_size, 0, x);
            if new_chunk.pattern[west_idx] == TileType::Floor {
                new_chunk.exists[2][x as usize] = true;
                n_exits += 1;
            }

            let east_idx = tile_idx_in_chunk(chunk_size, chunk_size - 1, x);
            if new_chunk.pattern[east_idx] == TileType::Floor {
                new_chunk.exists[3][x as usize] = true;
                n_exits += 1;
            }
        }

        if n_exits == 0 {
            new_chunk.has_exits = false;
        }

        constraints.push(new_chunk);
    }

    let ch = constraints.clone();
    for c in constraints.iter_mut() {
        for (j, potential) in ch.iter().enumerate() {
            if !c.has_exits || !potential.has_exits {
                for compat in c.compatible_with.iter_mut() {
                    compat.push(j);
                }
            } else {
                for (direction, exit_list) in c.exists.iter_mut().enumerate() {
                    let opposite = match direction {
                        0 => 1,
                        1 => 0,
                        2 => 3,
                        3 => 2,
                        _ => unreachable!()
                    };

                    let mut it_fits = false;
                    let mut has_any = false;

                    for (slot, can_enter) in exit_list.iter().enumerate() {
                        if *can_enter {
                            has_any = true;
                            if potential.exists[opposite][slot] {
                                it_fits = true;
                            }
                        }
                    }
                    if it_fits {
                        c.compatible_with[direction].push(j);
                    }
                    if !has_any {
                        let matching_exit_count = potential.exists[opposite].iter().filter(|a| !**a).count();
                        if matching_exit_count == 0 {
                            c.compatible_with[direction].push(j);
                        }
                    }
                }
            }
        }
    }

    constraints
}