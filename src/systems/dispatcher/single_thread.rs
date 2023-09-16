use crate::systems::dispatcher::UnifiedDispatcher;
use specs::{RunNow, World};

pub struct SingleThreadedDispatch<'a> {
    pub systems: Vec<Box<dyn RunNow<'a>>>,
}

impl<'a> UnifiedDispatcher for SingleThreadedDispatch<'a> {
    fn run_now(&mut self, ecs: *mut World) {
        for sys in self.systems.iter_mut() {
            sys.run_now(unsafe { &*ecs });
        }
        crate::effects::run_effects_queue(unsafe { &mut *ecs });
    }
}

macro_rules! construct_dispatcher {
    (
        $(
            (
                $type:ident,
                $name:expr,
                $deps:expr
            )
        ), *
    ) => {
        fn new_dispatch() -> Box<dyn UnifiedDispatcher + 'static> {
            let mut dispatch = SingleThreadedDispatch{
                systems: vec![]
            };

            $(
                dispatch.systems.push(Box::new($type{}));
            )*

            return Box::new(dispatch);
        }
    };
}
