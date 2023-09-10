use std::cmp;

use rltk::RandomNumberGenerator;

use crate::map::{tiletype::TileType, Map};
use crate::map_builders::{BuilderMap, InitialMapBuilder};

const TOP: usize = 0;
const RIGHT: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;

pub struct MazeBuilder {}

impl InitialMapBuilder for MazeBuilder {
    fn build_map(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        self.build(rng, build_data);
    }
}

impl MazeBuilder {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    pub(crate) fn build(&mut self, rng: &mut RandomNumberGenerator, build_data: &mut BuilderMap) {
        let mut maze = Grid::new(
            (build_data.map.width / 2) - 2,
            (build_data.map.height / 2) - 2,
            rng,
        );
        maze.generate_maze(build_data);
    }
}

#[derive(Copy, Clone)]
struct Cell {
    row: i32,
    column: i32,
    walls: [bool; 4],
    visited: bool,
}

impl Cell {
    const fn new(row: i32, column: i32) -> Self {
        Self {
            row,
            column,
            walls: [true, true, true, true],
            visited: false,
        }
    }

    fn remove_walls(&mut self, next: &mut Self) {
        let x = self.column - next.column;
        let y = self.row - next.row;

        if x == 1 {
            self.walls[LEFT] = false;
            next.walls[RIGHT] = false;
        } else if x == -1 {
            self.walls[RIGHT] = false;
            next.walls[LEFT] = false;
        } else if y == 1 {
            self.walls[TOP] = false;
            self.walls[BOTTOM] = false;
        } else if y == -1 {
            self.walls[BOTTOM] = false;
            next.walls[TOP] = false;
        }
    }
}

struct Grid<'a> {
    width: i32,
    height: i32,
    cells: Vec<Cell>,
    backtrace: Vec<usize>,
    current: usize,
    rng: &'a mut RandomNumberGenerator,
}

impl<'a> Grid<'a> {
    fn new(width: i32, height: i32, rng: &mut RandomNumberGenerator) -> Grid {
        let mut grid = Grid {
            width,
            height,
            cells: Vec::new(),
            backtrace: Vec::new(),
            current: 0,
            rng,
        };

        for row in 0..height {
            for column in 0..width {
                grid.cells.push(Cell::new(row, column));
            }
        }

        grid
    }

    const fn calculate_index(&self, row: i32, column: i32) -> Option<i32> {
        if row < 0 || column < 0 || column > self.width - 1 || row > self.height - 1 {
            None
        } else {
            Some(column + (row * self.width))
        }
    }

    fn get_available_neighbors(&self) -> Vec<usize> {
        let mut neighbors = Vec::new();

        let current_row = self.cells[self.current].row;
        let current_column = self.cells[self.current].column;

        let neighbor_indices: [Option<i32>; 4] = [
            self.calculate_index(current_row - 1, current_column),
            self.calculate_index(current_row, current_column + 1),
            self.calculate_index(current_row + 1, current_column),
            self.calculate_index(current_row, current_column - 1),
        ];

        for i in neighbor_indices.iter().flatten() {
            if !self.cells[*i as usize].visited {
                neighbors.push(*i as usize);
            }
        }

        neighbors
    }

    fn find_next_cell(&mut self) -> Option<usize> {
        let neighbors = self.get_available_neighbors();
        if !neighbors.is_empty() {
            return if neighbors.len() == 1 {
                Some(neighbors[0])
            } else {
                Some(neighbors[(self.rng.roll_dice(1, neighbors.len() as i32) - 1) as usize])
            };
        }

        None
    }

    fn generate_maze(&mut self, build_data: &mut BuilderMap) {
        let mut i = 0;
        loop {
            self.cells[self.current].visited = true;
            let next = self.find_next_cell();

            match next {
                Some(next) => {
                    self.cells[next].visited = true;
                    self.backtrace.push(self.current);
                    let (lower_part, higher_part) =
                        self.cells.split_at_mut(cmp::max(self.current, next));
                    let cell1 = &mut lower_part[cmp::min(self.current, next)];
                    let cell2 = &mut higher_part[0];
                    cell1.remove_walls(cell2);
                    self.current = next;
                }
                None => {
                    if !self.backtrace.is_empty() {
                        self.current = self.backtrace[0];
                        self.backtrace.remove(0);
                    } else {
                        break;
                    }
                }
            }
            if i % 50 == 0 {
                self.copy_to_map(&mut build_data.map);
                build_data.take_snapshot();
            }
            i += 1;
        }
    }
    fn copy_to_map(&self, map: &mut Map) {
        for i in &mut map.tiles {
            *i = TileType::Wall;
        }

        for cell in &self.cells {
            let x = cell.column + 1;
            let y = cell.row + 1;
            let idx = map.xy_idx(x * 2, y * 2);

            map.tiles[idx] = TileType::Floor;
            if !cell.walls[TOP] {
                map.tiles[idx - map.width as usize] = TileType::Floor;
            }
            if !cell.walls[RIGHT] {
                map.tiles[idx + 1] = TileType::Floor;
            }
            if !cell.walls[BOTTOM] {
                map.tiles[idx + map.width as usize] = TileType::Floor;
            }
            if !cell.walls[LEFT] {
                map.tiles[idx - 1] = TileType::Floor;
            }
        }
    }
}
