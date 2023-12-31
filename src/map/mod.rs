use std::collections::HashSet;

use rltk::{Algorithm2D, BaseMap, Point, BLACK, RGB};
use serde::{Deserialize, Serialize};
use specs::prelude::*;

pub use themes::*;

use crate::map::tiletype::TileType;

pub mod camera;
pub mod dungeon;
mod themes;
pub mod tiletype;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Map {
    pub tiles: Vec<TileType>,
    pub width: i32,
    pub height: i32,
    pub revealed_tiles: Vec<bool>,
    pub visible_tiles: Vec<bool>,
    pub depth: i32,
    pub bloodstains: HashSet<usize>,
    pub view_blocked: HashSet<usize>,
    pub name: String,
    pub natural_light: bool,
    pub light: Vec<RGB>,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub tile_content: Vec<Vec<Entity>>,
}

impl Map {
    pub const fn xy_idx(&self, x: i32, y: i32) -> usize {
        (y as usize * self.width as usize) + x as usize
    }

    fn is_exit_valid(&self, x: i32, y: i32) -> bool {
        if x < 1 || x > self.width - 1 || y < 1 || y > self.height - 1 {
            return false;
        }
        let idx = self.xy_idx(x, y);
        !crate::spatial::is_blocked(idx)
    }

    pub fn clear_content_index(&mut self) {
        crate::spatial::clear();
    }

    pub fn populate_blocked(&mut self) {
        crate::spatial::populate_blocked_from_map(self);
    }

    pub fn populate_blocked_multi(&mut self, width: i32, height: i32) {
        self.populate_blocked();
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                let idx = self.xy_idx(x, y);
                if crate::spatial::is_blocked(idx) {
                    return;
                }

                for cy in 0..height {
                    for cx in 0..width {
                        let tx = x + cx;
                        let ty = y + cy;
                        if tx < self.width - 1 && ty < self.height - 1 {
                            let tidx = self.xy_idx(tx, ty);
                            if crate::spatial::is_blocked(tidx) {
                                crate::spatial::set_blocked(idx, true);
                            }
                        } else {
                            crate::spatial::set_blocked(idx, true);
                        }
                    }
                }
            }
        }
    }

    /// Generates an empty map, consisting entirely of solid walls
    pub fn new(new_depth: i32, width: i32, height: i32, name: impl ToString) -> Self {
        let map_tile_count = (width * height) as usize;
        crate::spatial::set_size(map_tile_count);
        Self {
            tiles: vec![TileType::Wall; map_tile_count],
            width,
            height,
            revealed_tiles: vec![false; map_tile_count],
            visible_tiles: vec![false; map_tile_count],
            tile_content: vec![Vec::new(); map_tile_count],
            depth: new_depth,
            bloodstains: HashSet::new(),
            view_blocked: HashSet::new(),
            name: name.to_string(),
            natural_light: true,
            light: vec![RGB::named(BLACK); map_tile_count],
        }
    }
}

impl BaseMap for Map {
    fn is_opaque(&self, idx: usize) -> bool {
        if idx > 0 && idx < self.tiles.len() {
            self.tiles[idx].is_opaque() || self.view_blocked.contains(&idx)
        } else {
            true
        }
    }

    fn get_available_exits(&self, idx: usize) -> rltk::SmallVec<[(usize, f32); 10]> {
        let mut exits = rltk::SmallVec::new();
        let x = idx as i32 % self.width;
        let y = idx as i32 / self.width;
        let w = self.width as usize;
        let tt = self.tiles[idx];

        // Cardinal directions
        if self.is_exit_valid(x - 1, y) {
            exits.push((idx - 1, tt.get_cost()));
        };
        if self.is_exit_valid(x + 1, y) {
            exits.push((idx + 1, tt.get_cost()));
        };
        if self.is_exit_valid(x, y - 1) {
            exits.push((idx - w, tt.get_cost()));
        };
        if self.is_exit_valid(x, y + 1) {
            exits.push((idx + w, tt.get_cost()));
        };

        // Diagonals
        if self.is_exit_valid(x - 1, y - 1) {
            exits.push(((idx - w) - 1, tt.get_cost() * 1.45));
        }
        if self.is_exit_valid(x + 1, y - 1) {
            exits.push(((idx - w) + 1, tt.get_cost() * 1.45));
        }
        if self.is_exit_valid(x - 1, y + 1) {
            exits.push(((idx + w) - 1, tt.get_cost() * 1.45));
        }
        if self.is_exit_valid(x + 1, y + 1) {
            exits.push(((idx + w) + 1, tt.get_cost() * 1.45));
        }

        exits
    }

    fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
        let w = self.width as usize;
        let p1 = Point::new(idx1 % w, idx1 / w);
        let p2 = Point::new(idx2 % w, idx2 / w);
        rltk::DistanceAlg::Pythagoras.distance2d(p1, p2)
    }
}

impl Algorithm2D for Map {
    fn dimensions(&self) -> Point {
        Point::new(self.width, self.height)
    }
}
