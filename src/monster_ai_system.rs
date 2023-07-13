use rltk::{Point};
use specs::{Join, ReadExpect, ReadStorage, System, WriteExpect, WriteStorage, Entities, Entity};

use crate::{
    components::{Monster, Position, Viewshed, WantsToMelee},
    map::Map, player::RunState,
};

pub struct MonsterAI {}
impl<'a> System<'a> for MonsterAI {
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadExpect<'a, Point>,
        ReadExpect<'a, Entity>,
        ReadExpect<'a, RunState>,
        Entities<'a>,
        WriteStorage<'a, Viewshed>,
        ReadStorage<'a, Monster>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, WantsToMelee>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut map,
            player_pos,
            player_entity,
            runstate,
            entities,
            mut viewsheds,
            monsters,
            mut position,
            mut wants_to_melee
        ) = data;

        if *runstate != RunState::MonsterTurn {return;}

        for (entity, mut viewshed, _monster, mut pos)  in (&entities, &mut viewsheds, &monsters, &mut position).join() {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(Point::new(pos.x, pos.y), *player_pos);
            if distance < 1.5 {
                wants_to_melee.insert(entity, WantsToMelee { target: *player_entity }).expect("Unable to insert attack");
            }
            else if viewshed.visible_tiles.contains(&*player_pos) {
                let path = rltk::a_star_search(Map::xy_idx(pos.x, pos.y), Map::xy_idx(player_pos.x, player_pos.y), &mut *map);
                if path.success && path.steps.len() > 1 {
                    let mut idx = Map::xy_idx(pos.x, pos.y);
                    map.blocked[idx] = false;
                    pos.x = path.steps[1] as i32 % map.width;
                    pos.y = path.steps[1] as i32 / map.width;
                    idx = Map::xy_idx(pos.x, pos.y);
                    map.blocked[idx] = true;
                    viewshed.dirty = true;
                }
            }
        }
    }
}
