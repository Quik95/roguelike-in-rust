use rltk::GameState;
use specs::prelude::*;
use specs_derive::Component;

#[derive(Component, Debug)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component, Debug)]
struct Player {}
impl Player {
    fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
        let mut positions = ecs.write_storage::<Position>();
        let mut players = ecs.write_storage::<Player>();
        for (_player, pos) in (&mut players, &mut positions).join() {
            pos.x = i32::min(79, i32::max(0, pos.x + delta_x));
            pos.y = i32::min(49, i32::max(0, pos.y + delta_y));
        }
    }

    fn player_input(gs: &mut State, ctx: &mut rltk::Rltk) {
        match ctx.key {
            None => {}
            Some(key) => match key {
                rltk::VirtualKeyCode::Left => Player::try_move_player(-1, 0, &mut gs.ecs),
                rltk::VirtualKeyCode::Right => Player::try_move_player(1, 0, &mut gs.ecs),
                rltk::VirtualKeyCode::Up => Player::try_move_player(0, -1, &mut gs.ecs),
                rltk::VirtualKeyCode::Down => Player::try_move_player(0, 1, &mut gs.ecs),
                _ => {}
            },
        }
    }
}

#[derive(Component, Debug)]
struct Renderable {
    glyph: rltk::FontCharType,
    fg: rltk::RGB,
    bg: rltk::RGB,
}

#[derive(Component)]
struct LeftMover {}

struct LeftWalker {}
impl<'a> System<'a> for LeftWalker {
    type SystemData = (ReadStorage<'a, LeftMover>, WriteStorage<'a, Position>);

    fn run(&mut self, (lefty, mut pos): Self::SystemData) {
        for (_lefty, pos) in (&lefty, &mut pos).join() {
            pos.x -= 1;
            if pos.x < 0 {
                pos.x = 79;
            }
        }
    }
}

struct State {
    ecs: World,
}

impl State {
    fn run_systems(&mut self) {
        let mut lefty = LeftWalker {};
        lefty.run_now(&self.ecs);
        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut rltk::Rltk) {
        ctx.cls();

        Player::player_input(self, ctx);
        self.run_systems();

        let position = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();

        for (pos, render) in (&position, &renderables).join() {
            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
        }
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build()?;

    let mut gs = State { ecs: World::new() };

    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<LeftMover>();
    gs.ecs.register::<Player>();

    gs.ecs
        .create_entity()
        .with(Position { x: 40, y: 25 })
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: rltk::RGB::named(rltk::YELLOW),
            bg: rltk::RGB::named(rltk::BLACK),
        })
        .with(Player {})
        .build();

    for i in 0..10 {
        gs.ecs
            .create_entity()
            .with(Position { x: i * 7, y: 20 })
            .with(Renderable {
                glyph: rltk::to_cp437('☺'),
                fg: rltk::RGB::named(rltk::RED),
                bg: rltk::RGB::named(rltk::BLACK),
            })
            .with(LeftMover {})
            .build();
    }

    rltk::main_loop(context, gs)
}