use std::collections::HashSet;

use itertools::Itertools;
use rltk::{console, RandomNumberGenerator, XpFile};

use crate::components::Position;
use crate::map::{Map, TileType};
use crate::map_builders::common::remove_unreachable_areas_returning_most_distant;
use crate::map_builders::MapBuilder;
use crate::map_builders::prefab_builder::prefab_level::PrefabLevel;
use crate::map_builders::prefab_builder::prefab_rooms::{CHECKERBOARD, SILLY_SIMPLE, TOTALLY_NOT_A_TRAP};
use crate::map_builders::prefab_builder::prefab_section::{
    HorizontalPlacement, PrefabSection, VerticalPlacement
};
use crate::SHOW_MAPGEN_VISUALIZER;

pub mod prefab_level;
pub mod prefab_section;
pub mod prefab_rooms;

#[derive(Eq, PartialEq, Clone)]
pub enum PrefabMode {
    RexLevel { template: &'static str },
    Constant { level: PrefabLevel },
    Sectional { section: PrefabSection },
    RoomVaults,
}

pub struct PrefabBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    mode: PrefabMode,
    spawn_list: Vec<(usize, String)>,
    previous_builder: Option<Box<dyn MapBuilder>>,
}

impl MapBuilder for PrefabBuilder {
    fn build_map(&mut self) {
        self.build()
    }

    fn get_map(&self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&mut self) -> crate::components::Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn take_snapshot(&mut self) {
        if SHOW_MAPGEN_VISUALIZER {
            let mut snapshot = self.map.clone();
            for v in snapshot.revealed_tiles.iter_mut() {
                *v = true;
            }
            self.history.push(snapshot);
        }
    }

    fn get_spawn_list(&self) -> &Vec<(usize, String)> {
        &self.spawn_list
    }
}

impl PrefabBuilder {
    fn new(new_depth: i32, previous_builder: Option<Box<dyn MapBuilder>>) -> Self {
        Self {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            mode: PrefabMode::RoomVaults,
            spawn_list: Vec::new(),
            previous_builder,
        }
    }

    pub fn rex_level(new_depth: i32, template: &'static str) -> Self {
        Self {
            mode: PrefabMode::RexLevel { template },
            ..Self::new(new_depth, None)
        }
    }

    pub fn constant(new_depth: i32, level: PrefabLevel) -> Self {
        Self {
            mode: PrefabMode::Constant { level },
            ..Self::new(new_depth, None)
        }
    }

    pub fn sectional(new_depth: i32, section: PrefabSection, previous_builder: Box<dyn MapBuilder>) -> Self {
        Self {
            mode: PrefabMode::Sectional { section },
            ..Self::new(new_depth, Some(previous_builder))
        }
    }

    pub fn vaults(new_depth: i32, previous_builder: Box<dyn MapBuilder>) -> Self {
        Self {
            mode: PrefabMode::RoomVaults,
            ..Self::new(new_depth, Some(previous_builder))
        }
    }

    fn load_rex_map(&mut self, path: &str) {
        let xp_file = XpFile::from_resource(path).unwrap();

        for layer in &xp_file.layers {
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let cell = layer.get(x, y).unwrap();
                    if x < self.map.width as usize && y < self.map.height as usize {
                        let idx = Map::xy_idx(x as i32, y as i32);
                        self.char_to_map(cell.ch as u8 as char, idx);
                    }
                }
            }
        }
    }

    fn load_ascii_map(&mut self, level: &PrefabLevel) {
        let string_vec = Self::read_ascii_to_vec(level.template);

        let mut i = 0;
        for ty in 0..level.height {
            for tx in 0..level.width {
                if tx < self.map.width as usize && ty < self.map.height as usize {
                    let idx = Map::xy_idx(tx as i32, ty as i32);
                    self.char_to_map(string_vec[i], idx);
                }
                i += 1;
            }
        }
    }

    fn char_to_map(&mut self, ch: char, idx: usize) {
        match ch {
            ' ' => self.map.tiles[idx] = TileType::Floor,
            '#' => self.map.tiles[idx] = TileType::Wall,
            '@' => {
                let x = idx as i32 % self.map.width;
                let y = idx as i32 / self.map.width;
                self.map.tiles[idx] = TileType::Floor;
                self.starting_position = Position { x, y };
            }
            '>' => self.map.tiles[idx] = TileType::DownStairs,
            'g' => {
                self.map.tiles[idx] = TileType::Floor;
                self.spawn_list.push((idx, "Goblin".to_string()));
            }
            'o' => {
                self.map.tiles[idx] = TileType::Floor;
                self.spawn_list.push((idx, "Orc".to_string()));
            }
            '^' => {
                self.map.tiles[idx] = TileType::Floor;
                self.spawn_list.push((idx, "Bear Trap".to_string()));
            }
            '%' => {
                self.map.tiles[idx] = TileType::Floor;
                self.spawn_list.push((idx, "Rations".to_string()));
            }
            '!' => {
                self.map.tiles[idx] = TileType::Floor;
                self.spawn_list.push((idx, "Health Potion".to_string()));
            }
            unknown => {
                console::log(format!("Unknown glyph loading map: {unknown}"));
            }
        }
    }

    fn read_ascii_to_vec(template: &str) -> Vec<char> {
        let mut string_vec: Vec<char> = template.chars().filter(|a| *a != '\r' && *a != '\n').collect();
        for c in string_vec.iter_mut() { if *c as u8 == 160u8 { *c = ' '; } }
        string_vec
    }

    pub fn apply_section(&mut self, section: &PrefabSection) {
        let string_vec = Self::read_ascii_to_vec(section.template);

        // Place the new section
        let chunk_x = match section.placement.0 {
            HorizontalPlacement::Left => 0,
            HorizontalPlacement::Center => (self.map.width / 2) - (section.width as i32 / 2),
            HorizontalPlacement::Right => (self.map.width - 1) - section.width as i32,
        };

        let chunk_y = match section.placement.1 {
            VerticalPlacement::Top => 0,
            VerticalPlacement::Center => (self.map.height / 2) - (section.height as i32 / 2),
            VerticalPlacement::Bottom => (self.map.height - 1) - section.height as i32,
        };

        // Build the map
        self.apply_previous_iteration(|x, y, e| {
            x < chunk_x || x > (chunk_x + section.width as i32) || y < chunk_y || y > (chunk_y + section.height as i32)
        });

        let mut i = 0;
        for ty in 0..section.height {
            for tx in 0..section.width {
                if tx > 0
                    && tx < self.map.width as usize - 1
                    && ty < self.map.height as usize - 1
                    && ty > 0
                {
                    let idx = Map::xy_idx(tx as i32 + chunk_x, ty as i32 + chunk_y);
                    if let Some(&c) = string_vec.get(i) {
                        self.char_to_map(c, idx);
                    } else {
                        eprintln!("Broken template detected for y: {ty}, x: {tx}, using empty tile.");
                        self.char_to_map(' ', idx);
                    }
                }
                i += 1;
            }
        }
        self.take_snapshot();
    }

    fn apply_previous_iteration<F>(&mut self, mut filter: F)
        where F: FnMut(i32, i32, &(usize, String)) -> bool {
        let prev_builder = self.previous_builder.as_mut().unwrap();
        prev_builder.build_map();
        self.starting_position = prev_builder.get_starting_position();
        self.map = prev_builder.get_map().clone();
        for e in prev_builder.get_spawn_list().iter() {
            let idx = e.0;
            let x = idx as i32 % self.map.width;
            let y = idx as i32 / self.map.width;
            if filter(x, y, e) {
                self.spawn_list.push((idx, e.1.to_string()));
            }
        }
        self.take_snapshot();
    }

    fn apply_room_vaults(&mut self) {
        let mut rng = RandomNumberGenerator::new();
        self.apply_previous_iteration(|_, _, _| true);

        let vault_roll = rng.roll_dice(1, 6) + self.depth;
        if vault_roll < 4 { return; }

        let master_vault_list = vec![TOTALLY_NOT_A_TRAP, SILLY_SIMPLE, CHECKERBOARD];

        for vault in master_vault_list.iter() {
            assert_eq!(vault.template.len(), vault.width * vault.height, "This template is broken: {}", vault.template);
        }

        let mut possible_vaults = master_vault_list
            .iter()
            .filter(|v| self.depth >= v.first_depth && self.depth <= v.last_depth)
            .collect_vec();

        if possible_vaults.is_empty() { return; }

        let n_vaults = i32::min(rng.roll_dice(1, 3), possible_vaults.len() as i32);
        let mut used_tiles = HashSet::new();

        for _ in 0..n_vaults {
            let vault_index = if possible_vaults.len() == 1 { 0 } else { (rng.roll_dice(1, possible_vaults.len() as i32) - 1) as usize };
            let vault = possible_vaults[vault_index];

            let mut vault_positions = Vec::new();
            let mut idx = 0usize;

            loop {
                let x = (idx % self.map.width as usize) as i32;
                let y = (idx / self.map.width as usize) as i32;

                if x > 1
                    && (x + vault.width as i32) < self.map.width - 2
                    && y > 1
                    && (y + vault.height as i32) < self.map.height - 2 {
                    let mut possible = true;
                    for ty in 0..vault.height as i32 {
                        for tx in 0..vault.width as i32 {
                            let idx = Map::xy_idx(tx + x, ty + y);
                            if self.map.tiles[idx] != TileType::Floor {
                                possible = false;
                            }
                            if used_tiles.contains(&idx) {
                                possible = false;
                            }
                        }
                    }

                    if possible {
                        vault_positions.push(Position { x, y });
                        break;
                    }
                }

                idx += 1;
                if idx >= self.map.tiles.len() - 1 { break; }
            }

            if !vault_positions.is_empty() {
                let pos_idx = if vault_positions.len() == 1 { 0 } else { (rng.roll_dice(1, vault_positions.len() as i32) - 1) as usize };
                let pos = &vault_positions[pos_idx];

                let chunk_x = pos.x;
                let chunk_y = pos.y;

                let width = self.map.width;
                let height = self.map.height;
                self.spawn_list.retain(|e| {
                    let idx = e.0 as i32;
                    let x = idx % width;
                    let y = idx / height;
                    x < chunk_x || x > chunk_x + vault.width as i32 || y < chunk_y || y > chunk_y + vault.height as i32
                });

                let string_vec = Self::read_ascii_to_vec(vault.template);
                let mut i = 0;
                for ty in 0..vault.height {
                    for tx in 0..vault.width {
                        let idx = Map::xy_idx(tx as i32 + chunk_x, ty as i32 + chunk_y);
                        if let Some(&c) = string_vec.get(i) {
                            self.char_to_map(c, idx);
                            used_tiles.insert(idx);
                        } else {
                            eprintln!("Recovered out-of-bounds: i: {i}, tx: {tx}, ty: {ty}!!");
                        }
                        i += 1;
                    }
                }

                self.take_snapshot();
                possible_vaults.remove(vault_index);
            }
        }
    }

    fn build(&mut self) {
        match self.mode {
            PrefabMode::RexLevel { template } => self.load_rex_map(&template),
            PrefabMode::Constant { level } => self.load_ascii_map(&level),
            PrefabMode::Sectional { section } => self.apply_section(&section),
            PrefabMode::RoomVaults => self.apply_room_vaults()
        }
        self.take_snapshot();

        // Find a starting point; start at the middle and walk left until we find an open tile
        let mut start_idx;
        if self.starting_position.x == 0 {
            self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
            start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);
            while self.map.tiles[start_idx] != TileType::Floor {
                self.starting_position.x -= 1;
                start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);
            }
            self.take_snapshot();
        }
        let mut has_exit = false;
        for t in self.map.tiles.iter() {
            if *t == TileType::DownStairs { has_exit = true; }
        }

        if !has_exit {
            start_idx = Map::xy_idx(self.starting_position.x, self.starting_position.y);

            // Find all tiles we can reach from the starting point
            let exit_tile = remove_unreachable_areas_returning_most_distant(&mut self.map, start_idx);
            self.take_snapshot();

            // Place the stairs
            self.map.tiles[exit_tile] = TileType::DownStairs;
            self.take_snapshot();
        }
    }
}
