use rltk::{FontCharType, Rltk, RGB};
use specs::{Entities, Join, System, World, WorldExt, WriteExpect, WriteStorage};

use crate::components::{ParticleLifetime, Position, Renderable};

pub fn cull_dead_particles(ecs: &mut World, ctx: &Rltk) {
    let mut dead_particles = Vec::new();
    {
        let mut particles = ecs.write_storage::<ParticleLifetime>();
        let entities = ecs.entities();

        for (entity, particles) in (&entities, &mut particles).join() {
            particles.lifetime_ms -= ctx.frame_time_ms;
            if particles.lifetime_ms < 0.0 {
                dead_particles.push(entity);
            }
        }
    }

    for dead in &dead_particles {
        ecs.delete_entity(*dead).expect("Unable to delete");
    }
}

struct ParticleRequest {
    x: i32,
    y: i32,
    fg: RGB,
    bg: RGB,
    glyph: FontCharType,
    lifetime: f32,
}

pub struct ParticleBuilder {
    requests: Vec<ParticleRequest>,
}

impl ParticleBuilder {
    pub const fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    pub fn request(
        &mut self,
        x: i32,
        y: i32,
        fg: RGB,
        bg: RGB,
        glyph: FontCharType,
        lifetime: f32,
    ) {
        self.requests.push(ParticleRequest {
            x,
            y,
            fg,
            bg,
            glyph,
            lifetime,
        });
    }
}

pub struct ParticleSpawnSystem {}

impl<'a> System<'a> for ParticleSpawnSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Renderable>,
        WriteStorage<'a, ParticleLifetime>,
        WriteExpect<'a, ParticleBuilder>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut positions, mut renderables, mut particles, mut particle_builder) = data;
        for new_particle in &particle_builder.requests {
            let p = entities.create();
            positions
                .insert(
                    p,
                    Position {
                        x: new_particle.x,
                        y: new_particle.y,
                    },
                )
                .expect("Unable to insert");
            renderables
                .insert(
                    p,
                    Renderable {
                        glyph: new_particle.glyph,
                        fg: new_particle.fg,
                        bg: new_particle.bg,
                        render_order: 0,
                    },
                )
                .expect("Unable to insert");
            particles
                .insert(
                    p,
                    ParticleLifetime {
                        lifetime_ms: new_particle.lifetime,
                    },
                )
                .expect("Unable to insert");
        }

        particle_builder.requests.clear();
    }
}
