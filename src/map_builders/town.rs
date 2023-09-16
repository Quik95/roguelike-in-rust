use std::collections::HashSet;

use crate::astar::a_star_search;
use itertools::Itertools;
use rltk::{DistanceAlg, Point};

use crate::components::Position;
use crate::map::tiletype::TileType;
use crate::map_builders::{BuilderChain, BuilderMap, InitialMapBuilder};
use crate::rng::roll_dice;

pub fn town_builder(new_depth: i32, width: i32, height: i32) -> BuilderChain {
    let mut chain = BuilderChain::new(new_depth, width, height, "The Town of Bracketon");
    chain.start_with(TownBuilder::new());
    chain
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum BuildingTag {
    Pub,
    Temple,
    Blacksmith,
    Clothier,
    Alchemist,
    PlayerHouse,
    Hovel,
    Abandoned,
    Unassigned,
}

pub struct TownBuilder {}

impl InitialMapBuilder for TownBuilder {
    fn build_map(&mut self, build_data: &mut BuilderMap) {
        self.build(build_data);
    }
}

impl TownBuilder {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    pub fn build(&mut self, build_data: &mut BuilderMap) {
        self.grass_layer(build_data);
        self.water_and_piers(build_data);

        let (mut available_building_tiles, wall_gap_y) = self.town_walls(build_data);
        let buildings = self.buildings(build_data, &mut available_building_tiles);
        let doors = self.add_doors(build_data, &buildings, wall_gap_y);
        self.add_paths(build_data, &doors);
        for y in wall_gap_y - 3..wall_gap_y + 4 {
            let exit_idx = build_data.map.xy_idx(build_data.width - 2, y);
            build_data.map.tiles[exit_idx] = TileType::DownStairs;
        }

        self.spawn_dockers(build_data);
        self.spawn_townsfolk(build_data, &available_building_tiles);

        let buildings_sorted = self.sort_buildings(&buildings);
        self.building_factory(build_data, &buildings, &buildings_sorted);

        for t in &mut build_data.map.visible_tiles {
            *t = true;
        }

        build_data.take_snapshot();
    }
    fn grass_layer(&self, build_data: &mut BuilderMap) {
        for t in &mut build_data.map.tiles {
            *t = TileType::Grass;
        }
        build_data.take_snapshot();
    }
    fn water_and_piers(&self, build_data: &mut BuilderMap) {
        let mut n = (roll_dice(1, 65535) as f32) / 65535f32;
        let mut water_width = Vec::new();

        for y in 0..build_data.height {
            let n_water = (f32::sin(n) * 10.0) as i32 + 14 + roll_dice(1, 6);
            water_width.push(n_water);
            n += 0.1;
            for x in 0..n_water {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = TileType::DeepWater;
            }
            for x in n_water..n_water + 3 {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = TileType::ShallowWater;
            }
        }
        build_data.take_snapshot();

        for _i in 0..roll_dice(1, 4) + 6 {
            let y = roll_dice(1, build_data.height) - 1;
            for x in 2 + roll_dice(1, 6)..water_width[y as usize] + 4 {
                let idx = build_data.map.xy_idx(x, y);
                build_data.map.tiles[idx] = TileType::WoodFloor;
            }
        }
        build_data.take_snapshot();
    }
    fn town_walls(&self, build_data: &mut BuilderMap) -> (HashSet<usize>, i32) {
        let mut available_building_tiles = HashSet::new();
        let wall_gap_y = roll_dice(1, build_data.height - 9) + 5;
        for y in 1..build_data.height - 2 {
            if !(y > wall_gap_y - 4 && y < wall_gap_y + 4) {
                let idx = build_data.map.xy_idx(30, y);
                build_data.map.tiles[idx] = TileType::Wall;
                build_data.map.tiles[idx - 1] = TileType::Floor;
                let idx_right = build_data.map.xy_idx(build_data.width - 2, y);
                build_data.map.tiles[idx_right] = TileType::Wall;
                for x in 31..build_data.width - 2 {
                    let gravel_idx = build_data.map.xy_idx(x, y);
                    build_data.map.tiles[gravel_idx] = TileType::Gravel;
                    if y > 2 && y < build_data.height - 1 {
                        available_building_tiles.insert(gravel_idx);
                    }
                }
            } else {
                for x in 30..build_data.width {
                    let road_idx = build_data.map.xy_idx(x, y);
                    build_data.map.tiles[road_idx] = TileType::Road;
                }
            }
        }
        build_data.take_snapshot();

        for x in 30..build_data.width - 1 {
            let idx_top = build_data.map.xy_idx(x, 1);
            build_data.map.tiles[idx_top] = TileType::Wall;
            let idx_bot = build_data.map.xy_idx(x, build_data.height - 2);
            build_data.map.tiles[idx_bot] = TileType::Wall;
        }
        build_data.take_snapshot();

        (available_building_tiles, wall_gap_y)
    }
    fn buildings(
        &self,

        build_data: &mut BuilderMap,
        available_building_tiles: &mut HashSet<usize>,
    ) -> Vec<(i32, i32, i32, i32)> {
        let mut buildings = Vec::new();
        let mut n_buildings = 0;
        while n_buildings < 12 {
            let bx = roll_dice(1, build_data.map.width - 32) + 30;
            let by = roll_dice(1, build_data.map.height) - 2;
            let bw = roll_dice(1, 8) + 4;
            let bh = roll_dice(1, 8) + 4;
            let mut possible = true;
            for y in by..by + bh {
                for x in bx..bx + bw {
                    if x < 0 || x > build_data.width - 1 || y < 0 || y > build_data.height - 1 {
                        possible = false;
                    } else {
                        let idx = build_data.map.xy_idx(x, y);
                        if !available_building_tiles.contains(&idx) {
                            possible = false;
                        }
                    }
                }
            }
            if possible {
                n_buildings += 1;
                buildings.push((bx, by, bw, bh));
                for y in by..by + bh {
                    for x in bx..bx + bw {
                        let idx = build_data.map.xy_idx(x, y);
                        build_data.map.tiles[idx] = TileType::WoodFloor;
                        available_building_tiles.remove(&idx);
                        available_building_tiles.remove(&(idx + 1));
                        available_building_tiles.remove(&(idx + build_data.width as usize));
                        available_building_tiles.remove(&(idx - 1));
                        available_building_tiles.remove(&(idx - build_data.width as usize));
                    }
                }
                build_data.take_snapshot();
            }
        }

        let mut mapclone = build_data.map.clone();
        for y in 2..build_data.height - 2 {
            for x in 32..build_data.width - 2 {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::WoodFloor {
                    let mut neighbors = 0;
                    if build_data.map.tiles[idx - 1] != TileType::WoodFloor {
                        neighbors += 1;
                    }
                    if build_data.map.tiles[idx + 1] != TileType::WoodFloor {
                        neighbors += 1;
                    }
                    if build_data.map.tiles[idx - build_data.width as usize] != TileType::WoodFloor
                    {
                        neighbors += 1;
                    }
                    if build_data.map.tiles[idx + build_data.width as usize] != TileType::WoodFloor
                    {
                        neighbors += 1;
                    }
                    if neighbors > 0 {
                        mapclone.tiles[idx] = TileType::Wall;
                    }
                }
            }
        }

        build_data.map = mapclone;
        build_data.take_snapshot();
        buildings
    }
    fn add_doors(
        &self,

        build_data: &mut BuilderMap,
        buildings: &[(i32, i32, i32, i32)],
        wall_gap_y: i32,
    ) -> Vec<usize> {
        let mut doors = Vec::new();
        for building in buildings {
            let door_x = building.0 + 1 + roll_dice(1, building.2 - 3);
            let cy = building.1 + (building.3 / 2);
            let idx = if cy > wall_gap_y {
                build_data.map.xy_idx(door_x, building.1)
            } else {
                build_data.map.xy_idx(door_x, building.1 + building.3 - 1)
            };
            build_data.map.tiles[idx] = TileType::Floor;
            build_data.spawn_list.push((idx, "Door".to_string()));
            doors.push(idx);
        }

        build_data.take_snapshot();
        doors
    }
    fn add_paths(&self, build_data: &mut BuilderMap, doors: &[usize]) {
        let mut roads = Vec::new();
        for y in 0..build_data.height {
            for x in 0..build_data.width {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::Road {
                    roads.push(idx);
                }
            }
        }

        build_data.map.populate_blocked();
        for door_idx in doors {
            let mut nearest_roads = Vec::new();
            let door_pt = rltk::Point::new(
                *door_idx as i32 % build_data.map.width,
                *door_idx as i32 / build_data.map.width,
            );
            for r in &roads {
                nearest_roads.push((
                    *r,
                    DistanceAlg::PythagorasSquared.distance2d(
                        door_pt,
                        Point::new(
                            *r as i32 % build_data.map.width,
                            *r as i32 / build_data.map.width,
                        ),
                    ),
                ));
            }
            nearest_roads.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let destination = nearest_roads[0].0;
            let path = a_star_search(*door_idx, destination, &build_data.map);
            if path.success {
                for step in &path.steps {
                    let idx = *step;
                    build_data.map.tiles[idx] = TileType::Road;
                    roads.push(idx);
                }
            }
            build_data.take_snapshot();
        }
    }

    fn sort_buildings(
        &mut self,
        buildings: &[(i32, i32, i32, i32)],
    ) -> Vec<(usize, i32, BuildingTag)> {
        let mut sorted = buildings
            .iter()
            .enumerate()
            .map(|(i, building)| (i, building.2 * building.3, BuildingTag::Unassigned))
            .sorted_by(|a, b| b.1.cmp(&a.1))
            .collect_vec();

        sorted[0].2 = BuildingTag::Pub;
        sorted[1].2 = BuildingTag::Temple;
        sorted[2].2 = BuildingTag::Blacksmith;
        sorted[3].2 = BuildingTag::Clothier;
        sorted[4].2 = BuildingTag::Alchemist;
        sorted[5].2 = BuildingTag::PlayerHouse;
        for b in sorted.iter_mut().skip(6) {
            b.2 = BuildingTag::Hovel;
        }
        let last_index = sorted.len() - 1;
        sorted[last_index].2 = BuildingTag::Abandoned;
        sorted
    }
    fn building_factory(
        &mut self,

        build_data: &mut BuilderMap,
        buildings: &[(i32, i32, i32, i32)],
        building_index: &[(usize, i32, BuildingTag)],
    ) {
        for (i, building) in buildings.iter().enumerate() {
            let build_type = &building_index[i].2;
            match build_type {
                BuildingTag::Pub => self.build_pub(building, build_data),
                BuildingTag::Temple => self.build_temple(building, build_data),
                BuildingTag::Blacksmith => self.build_smith(building, build_data),
                BuildingTag::Clothier => self.build_clothier(building, build_data),
                BuildingTag::Alchemist => self.build_alchemist(building, build_data),
                BuildingTag::PlayerHouse => self.build_my_house(building, build_data),
                BuildingTag::Hovel => self.build_hovel(building, build_data),
                BuildingTag::Abandoned => self.build_abandoned_house(building, build_data),
                _ => {}
            }
        }
    }
    fn build_pub(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        build_data.starting_position = Some(Position {
            x: building.0 + (building.2 / 2),
            y: building.1 + (building.3 / 2),
        });
        let player_idx = build_data
            .map
            .xy_idx(building.0 + (building.2 / 2), building.1 + (building.3 / 2));

        let mut to_place = vec![
            "Barkeep",
            "Shady Salesman",
            "Patron",
            "Patron",
            "Keg",
            "Table",
            "Chair",
            "Table",
            "Chair",
        ];

        self.random_building_spawn(building, build_data, &mut to_place, player_idx);
    }

    fn random_building_spawn(
        &mut self,
        building: &(i32, i32, i32, i32),
        build_data: &mut BuilderMap,

        to_place: &mut Vec<&str>,
        player_idx: usize,
    ) {
        for y in building.1..building.1 + building.3 {
            for x in building.0..building.0 + building.2 {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::WoodFloor
                    && idx != player_idx
                    && roll_dice(1, 3) == 1
                    && !to_place.is_empty()
                {
                    let entity_tag = to_place[0];
                    to_place.remove(0);
                    build_data.spawn_list.push((idx, entity_tag.to_string()));
                }
            }
        }
    }
    fn build_temple(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        let mut to_place = vec![
            "Priest",
            "Altar",
            "Parishioner",
            "Parishioner",
            "Chair",
            "Chair",
            "Candle",
            "Candle",
        ];
        self.random_building_spawn(building, build_data, &mut to_place, 0);
    }
    fn build_smith(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        // Place items
        let mut to_place: Vec<&str> = vec![
            "Blacksmith",
            "Anvil",
            "Water Trough",
            "Weapon Rack",
            "Armor Stand",
        ];
        self.random_building_spawn(building, build_data, &mut to_place, 0);
    }

    fn build_clothier(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        // Place items
        let mut to_place: Vec<&str> = vec!["Clothier", "Cabinet", "Table", "Loom", "Hide Rack"];
        self.random_building_spawn(building, build_data, &mut to_place, 0);
    }

    fn build_alchemist(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        // Place items
        let mut to_place: Vec<&str> =
            vec!["Alchemist", "Chemistry Set", "Dead Thing", "Chair", "Table"];
        self.random_building_spawn(building, build_data, &mut to_place, 0);
    }

    fn build_my_house(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        // Place items
        let mut to_place: Vec<&str> = vec!["Mom", "Bed", "Cabinet", "Chair", "Table"];
        self.random_building_spawn(building, build_data, &mut to_place, 0);
    }

    fn build_hovel(&mut self, building: &(i32, i32, i32, i32), build_data: &mut BuilderMap) {
        // Place items
        let mut to_place: Vec<&str> = vec!["Peasant", "Bed", "Chair", "Table"];
        self.random_building_spawn(building, build_data, &mut to_place, 0);
    }
    fn build_abandoned_house(
        &mut self,
        building: &(i32, i32, i32, i32),
        build_data: &mut BuilderMap,
    ) {
        for y in building.1..building.1 + building.3 {
            for x in building.0..building.0 + building.2 {
                let idx = build_data.map.xy_idx(x, y);
                if build_data.map.tiles[idx] == TileType::WoodFloor
                    && idx != 0
                    && roll_dice(1, 2) == 1
                {
                    build_data.spawn_list.push((idx, "Rat".to_string()));
                }
            }
        }
    }
    fn spawn_dockers(&self, build_data: &mut BuilderMap) {
        for (idx, tt) in build_data.map.tiles.iter_mut().enumerate() {
            if *tt == TileType::Bridge && roll_dice(1, 6) == 1 {
                let roll = roll_dice(1, 3);
                match roll {
                    1 => build_data.spawn_list.push((idx, "Dock Worker".to_string())),
                    2 => build_data
                        .spawn_list
                        .push((idx, "Wannabe Pirate".to_string())),
                    3 => build_data.spawn_list.push((idx, "Fisher".to_string())),
                    _ => unreachable!(),
                }
            }
        }
    }
    fn spawn_townsfolk(
        &self,
        build_data: &mut BuilderMap,

        available_building_tiles: &HashSet<usize>,
    ) {
        for idx in available_building_tiles {
            if roll_dice(1, 10) == 1 {
                let roll = roll_dice(1, 4);
                match roll {
                    1 => build_data.spawn_list.push((*idx, "Peasant".to_string())),
                    2 => build_data.spawn_list.push((*idx, "Drunk".to_string())),
                    3 => build_data
                        .spawn_list
                        .push((*idx, "Dock Worker".to_string())),
                    4 => build_data.spawn_list.push((*idx, "Fisher".to_string())),
                    _ => unreachable!(),
                }
            }
        }
    }
}
