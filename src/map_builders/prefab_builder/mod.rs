use std::collections::HashSet;

use itertools::Itertools;
use rltk::{console, XpFile};

use crate::components::Position;
use crate::map::tiletype::TileType;
use crate::map_builders::prefab_builder::prefab_level::PrefabLevel;
use crate::map_builders::prefab_builder::prefab_rooms::{
    CHECKERBOARD, SILLY_SIMPLE, TOTALLY_NOT_A_TRAP,
};
use crate::map_builders::prefab_builder::prefab_section::{
    HorizontalPlacement, PrefabSection, VerticalPlacement,
};
use crate::map_builders::{BuilderMap, InitialMapBuilder, MetaMapBuilder};
use crate::rng::roll_dice;

pub mod prefab_level;
pub mod prefab_rooms;
pub mod prefab_section;

#[derive(Eq, PartialEq, Clone)]
#[allow(dead_code)]
pub enum PrefabMode {
    RexLevel { template: &'static str },
    Constant { level: PrefabLevel },
    Sectional { section: PrefabSection },
    RoomVaults,
}

pub struct PrefabBuilder {
    mode: PrefabMode,
}

impl MetaMapBuilder for PrefabBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl InitialMapBuilder for PrefabBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl PrefabBuilder {
    #[allow(dead_code)]
    pub fn rex_level(template: &'static str) -> Box<Self> {
        Box::new(Self {
            mode: PrefabMode::RexLevel { template },
        })
    }

    pub fn constant(level: PrefabLevel) -> Box<Self> {
        Box::new(Self {
            mode: PrefabMode::Constant { level },
        })
    }

    pub fn sectional(section: PrefabSection) -> Box<Self> {
        Box::new(Self {
            mode: PrefabMode::Sectional { section },
        })
    }

    pub fn vaults() -> Box<Self> {
        Box::new(Self {
            mode: PrefabMode::RoomVaults,
        })
    }

    fn load_rex_map(&mut self, path: &str, build_data: &mut BuilderMap) {
        let xp_file = XpFile::from_resource(path).unwrap();

        for layer in &xp_file.layers {
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let cell = layer.get(x, y).unwrap();
                    if x < build_data.map.width as usize && y < build_data.map.height as usize {
                        let idx = build_data.map.xy_idx(x as i32, y as i32);
                        self.char_to_map(cell.ch as u8 as char, idx, build_data);
                    }
                }
            }
        }
    }

    fn load_ascii_map(&mut self, level: &PrefabLevel, build_data: &mut BuilderMap) {
        let string_vec = Self::read_ascii_to_vec(level.template);

        let mut i = 0;
        for ty in 0..level.height {
            for tx in 0..level.width {
                if tx < build_data.map.width as usize && ty < build_data.map.height as usize {
                    let idx = build_data.map.xy_idx(tx as i32, ty as i32);
                    if let Some(&c) = string_vec.get(i) {
                        self.char_to_map(c, idx, build_data);
                    } else {
                        self.char_to_map(' ', idx, build_data);
                    }
                }
                i += 1;
            }
        }
    }

    fn char_to_map(&mut self, ch: char, idx: usize, build_data: &mut BuilderMap) {
        match ch {
            ' ' => build_data.map.tiles[idx] = TileType::Floor,
            '#' => build_data.map.tiles[idx] = TileType::Wall,
            '@' => {
                let x = idx as i32 % build_data.map.width;
                let y = idx as i32 / build_data.map.width;
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.starting_position = Some(Position { x, y });
            }
            '>' => build_data.map.tiles[idx] = TileType::DownStairs,
            'g' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Goblin".to_string()));
            }
            'o' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Orc".to_string()));
            }
            '^' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Bear Trap".to_string()));
            }
            '%' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Rations".to_string()));
            }
            '!' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data
                    .spawn_list
                    .push((idx, "Health Potion".to_string()));
            }
            '≈' => build_data.map.tiles[idx] = TileType::DeepWater,
            'O' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Orc Leader".to_string()));
            }
            '☼' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Watch Fire".to_string()));
            }
            'e' => {
                build_data.map.tiles[idx] = TileType::Floor;
                build_data.spawn_list.push((idx, "Dark Elf".into()));
            }
            unknown => {
                console::log(format!("Unknown glyph loading map: {unknown}"));
            }
        }
    }

    fn read_ascii_to_vec(template: &str) -> Vec<char> {
        let mut string_vec: Vec<char> = template
            .chars()
            .filter(|a| *a != '\r' && *a != '\n')
            .collect();
        for c in &mut string_vec {
            if *c as u8 == 160u8 {
                *c = ' ';
            }
        }
        string_vec
    }

    pub fn apply_sectional(&mut self, section: &PrefabSection, build_data: &mut BuilderMap) {
        let string_vec = Self::read_ascii_to_vec(section.template);

        // Place the new section
        let chunk_x = match section.placement.0 {
            HorizontalPlacement::Left => 0,
            HorizontalPlacement::Center => (build_data.map.width / 2) - (section.width as i32 / 2),
            HorizontalPlacement::Right => (build_data.map.width - 1) - section.width as i32,
        };

        let chunk_y = match section.placement.1 {
            VerticalPlacement::Top => 0,
            VerticalPlacement::Center => (build_data.map.height / 2) - (section.height as i32 / 2),
            VerticalPlacement::Bottom => (build_data.map.height - 1) - section.height as i32,
        };

        // Build the map
        self.apply_previous_iteration(
            |x, y| {
                x < chunk_x
                    || x > (chunk_x + section.width as i32)
                    || y < chunk_y
                    || y > (chunk_y + section.height as i32)
            },
            build_data,
        );

        let mut i = 0;
        for ty in 0..section.height {
            for tx in 0..section.width {
                if tx > 0
                    && tx < build_data.map.width as usize - 1
                    && ty < build_data.map.height as usize - 1
                    && ty > 0
                {
                    let idx = build_data
                        .map
                        .xy_idx(tx as i32 + chunk_x, ty as i32 + chunk_y);
                    if let Some(&c) = string_vec.get(i) {
                        self.char_to_map(c, idx, build_data);
                    } else {
                        eprintln!(
                            "Broken template detected for y: {ty}, x: {tx}, using empty tile."
                        );
                        self.char_to_map(' ', idx, build_data);
                    }
                }
                i += 1;
            }
        }
        build_data.take_snapshot();
    }

    fn apply_previous_iteration<F>(&mut self, mut filter: F, build_data: &mut BuilderMap)
    where
        F: FnMut(i32, i32) -> bool,
    {
        let width = build_data.map.width;
        build_data.spawn_list.retain(|(idx, _name)| {
            let x = *idx as i32 % width;
            let y = *idx as i32 / width;
            filter(x, y)
        });
        build_data.take_snapshot();
    }

    fn apply_room_vaults(&mut self, build_data: &mut BuilderMap) {
        self.apply_previous_iteration(|_, _| true, build_data);

        let vault_roll = roll_dice(1, 6) + build_data.map.depth;
        if vault_roll < 4 {
            return;
        }

        let master_vault_list = vec![TOTALLY_NOT_A_TRAP, SILLY_SIMPLE, CHECKERBOARD];

        for vault in &master_vault_list {
            assert_eq!(
                vault.template.len(),
                vault.width * vault.height,
                "This template is broken: {}",
                vault.template
            );
        }

        let mut possible_vaults = master_vault_list
            .iter()
            .filter(|v| {
                build_data.map.depth >= v.first_depth && build_data.map.depth <= v.last_depth
            })
            .collect_vec();

        if possible_vaults.is_empty() {
            return;
        }

        let n_vaults = i32::min(roll_dice(1, 3), possible_vaults.len() as i32);
        let mut used_tiles = HashSet::new();

        for _ in 0..n_vaults {
            let vault_index = if possible_vaults.len() == 1 {
                0
            } else {
                (roll_dice(1, possible_vaults.len() as i32) - 1) as usize
            };
            let vault = possible_vaults[vault_index];

            let mut vault_positions = Vec::new();
            let mut idx = 0usize;

            loop {
                let x = (idx % build_data.map.width as usize) as i32;
                let y = (idx / build_data.map.width as usize) as i32;

                if x > 1
                    && (x + vault.width as i32) < build_data.map.width - 2
                    && y > 1
                    && (y + vault.height as i32) < build_data.map.height - 2
                {
                    let mut possible = true;
                    for ty in 0..vault.height as i32 {
                        for tx in 0..vault.width as i32 {
                            let idx = build_data.map.xy_idx(tx + x, ty + y);
                            if build_data.map.tiles[idx] != TileType::Floor {
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
                if idx >= build_data.map.tiles.len() - 1 {
                    break;
                }
            }

            if !vault_positions.is_empty() {
                let pos_idx = if vault_positions.len() == 1 {
                    0
                } else {
                    (roll_dice(1, vault_positions.len() as i32) - 1) as usize
                };
                let pos = &vault_positions[pos_idx];

                let chunk_x = pos.x;
                let chunk_y = pos.y;

                let width = build_data.map.width;
                let height = build_data.map.height;
                build_data.spawn_list.retain(|e| {
                    let idx = e.0 as i32;
                    let x = idx % width;
                    let y = idx / height;
                    x < chunk_x
                        || x > chunk_x + vault.width as i32
                        || y < chunk_y
                        || y > chunk_y + vault.height as i32
                });

                let string_vec = Self::read_ascii_to_vec(vault.template);
                let mut i = 0;
                for ty in 0..vault.height {
                    for tx in 0..vault.width {
                        let idx = build_data
                            .map
                            .xy_idx(tx as i32 + chunk_x, ty as i32 + chunk_y);
                        if let Some(&c) = string_vec.get(i) {
                            self.char_to_map(c, idx, build_data);
                            used_tiles.insert(idx);
                        } else {
                            eprintln!("Recovered out-of-bounds: i: {i}, tx: {tx}, ty: {ty}!!");
                        }
                        i += 1;
                    }
                }

                build_data.take_snapshot();
                possible_vaults.remove(vault_index);
            }
        }
    }

    fn build(&mut self, build_data: &mut BuilderMap) {
        match self.mode {
            PrefabMode::RexLevel { template } => self.load_rex_map(template, build_data),
            PrefabMode::Constant { level } => self.load_ascii_map(&level, build_data),
            PrefabMode::Sectional { section } => self.apply_sectional(&section, build_data),
            PrefabMode::RoomVaults => self.apply_room_vaults(build_data),
        }
        build_data.take_snapshot();
    }
}
