use rltk::{field_of_view, Point, RandomNumberGenerator, RED};
use specs::prelude::*;

use crate::components::{BlocksVisibility, Hidden, Name};
use crate::{
    components::{Player, Position, Viewshed},
    gamelog,
    map::Map,
    spatial,
};

pub struct VisibilitySystem {}

impl<'a> System<'a> for VisibilitySystem {
    type SystemData = (
        WriteExpect<'a, Map>,
        Entities<'a>,
        WriteStorage<'a, Viewshed>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Hidden>,
        WriteExpect<'a, RandomNumberGenerator>,
        ReadStorage<'a, Name>,
        ReadStorage<'a, BlocksVisibility>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut map,
            entities,
            mut viewshed,
            pos,
            player,
            mut hidden,
            mut rng,
            name,
            blocks_visibility,
        ) = data;

        map.view_blocked.clear();
        for (block_pos, _block) in (&pos, &blocks_visibility).join() {
            let idx = map.xy_idx(block_pos.x, block_pos.y);
            map.view_blocked.insert(idx);
        }

        for (ent, viewshed, pos) in (&entities, &mut viewshed, &pos).join() {
            if !viewshed.dirty {
                continue;
            }

            viewshed.visible_tiles.clear();
            viewshed.visible_tiles = field_of_view(Point::new(pos.x, pos.y), viewshed.range, &*map);
            viewshed
                .visible_tiles
                .retain(|p| p.x >= 0 && p.x < map.width && p.y >= 0 && p.y < map.height);

            let p: Option<&Player> = player.get(ent);
            if let Some(_p) = p {
                for t in &mut map.visible_tiles {
                    *t = false;
                }
                for vis in &viewshed.visible_tiles {
                    let idx = map.xy_idx(vis.x, vis.y);
                    map.revealed_tiles[idx] = true;
                    map.visible_tiles[idx] = true;

                    spatial::for_each_tile_content(idx, |e| {
                        let maybe_hidden = hidden.get(e);
                        if let Some(_hidden) = maybe_hidden {
                            if rng.roll_dice(1, 24) == 1 {
                                let name = name.get(e);
                                if let Some(name) = name {
                                    gamelog::Logger::new()
                                        .append("You spotted:")
                                        .color(RED)
                                        .append(&name.name)
                                        .log();
                                }
                                hidden.remove(e);
                            }
                        }
                    });
                }
            }
        }
    }
}
