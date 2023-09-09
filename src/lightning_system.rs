use rltk::{DistanceAlg, Point, BLACK, RGB};
use specs::{Join, ReadStorage, System, WriteExpect};

use crate::components::{LightSource, Position, Viewshed};
use crate::map::Map;

pub struct LightingSystem {}

impl<'a> System<'a> for LightingSystem {
    type SystemData = (
        WriteExpect<'a, Map>,
        ReadStorage<'a, Viewshed>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, LightSource>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut map, viewshed, positions, lighting) = data;

        if map.natural_light {
            return;
        }

        let black = RGB::named(BLACK);
        for l in &mut map.light {
            *l = black;
        }

        for (viewshed, pos, light) in (&viewshed, &positions, &lighting).join() {
            let light_point = Point::new(pos.x, pos.y);
            let range_f = light.range as f32;
            for t in &viewshed.visible_tiles {
                if t.x > 0 && t.x < map.width && t.y > 0 && t.y < map.height {
                    let idx = map.xy_idx(t.x, t.y);
                    let distance = DistanceAlg::Pythagoras.distance2d(light_point, *t);
                    let intensity = (range_f - distance) / range_f;

                    map.light[idx] = map.light[idx] + (light.color * intensity);
                }
            }
        }
    }
}
