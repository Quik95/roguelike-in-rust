use crate::systems::UnifiedDispatcher;
use specs::{Dispatcher, World};

pub struct MultiThreadedDispatcher {
    pub dispatcher: Dispatcher<'static, 'static>,
}

impl UnifiedDispatcher for MultiThreadedDispatcher {
    fn run_now(&mut self, ecs: *mut World) {
        self.dispatcher.dispatch(unsafe { &mut *ecs });
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
            use specs::DispatcherBuilder;

            let dispatcher = DispatcherBuilder::new()
            $(
                .with($type{}, $name, $deps)
            )*
            .build();

            let dispatch = MultiThreadedDispatcher {
                dispatcher
            };

            return Box::new(dispatch);
        }
    };
}
