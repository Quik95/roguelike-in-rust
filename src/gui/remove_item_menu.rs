use crate::components::Equipped;
use crate::gui::{get_item_display_name, item_result_menu, ItemMenuResult};
use crate::State;
use rltk::{BTerm as Rltk, DrawBatch};
use specs::{Entity, Join, WorldExt};

pub fn remove_item_menu(gs: &State, ctx: &Rltk) -> (ItemMenuResult, Option<Entity>) {
    let mut draw_batch = DrawBatch::new();

    let player_entity = gs.ecs.fetch::<Entity>();
    let backpack = gs.ecs.read_storage::<Equipped>();
    let entities = gs.ecs.entities();

    let mut items = vec![];
    (&entities, &backpack)
        .join()
        .filter(|item| item.1.owner == *player_entity)
        .for_each(|item| {
            items.push((item.0, get_item_display_name(&gs.ecs, item.0)));
        });

    let result = item_result_menu(
        &mut draw_batch,
        "Remove which item?",
        items.len(),
        &items,
        ctx.key,
    );

    draw_batch.submit(6000).expect("Batched draw failed");
    result
}
