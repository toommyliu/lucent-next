use std::sync::{Arc, Mutex, Once};
use tracing_subscriber::prelude::*;

#[derive(Clone, Default)]
pub struct Logs(Arc<Mutex<Vec<(tracing::Level, String)>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Logs {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        struct Message(String);
        impl tracing::field::Visit for Message {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        self.0
            .lock()
            .unwrap()
            .push((*event.metadata().level(), message.0));
    }
}

impl Logs {
    pub fn dispatch(&self) -> tracing::Dispatch {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            tracing::subscriber::set_global_default(tracing_subscriber::registry()).unwrap();
        });
        tracing::Dispatch::new(tracing_subscriber::registry().with(self.clone()))
    }

    pub fn messages(&self, level: tracing::Level) -> Vec<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|(l, _)| *l == level)
            .map(|(_, text)| text.clone())
            .collect()
    }
}
