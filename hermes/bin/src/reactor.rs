use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

use crate::{event_queue::event::HermesEventPayload, wasm::module::Module};

pub(crate) struct HermesReactor {
    wasm_module: Module,

    event_sender: Sender<Box<dyn HermesEventPayload>>,
    event_receiver: Receiver<Box<dyn HermesEventPayload>>,
}

impl HermesReactor {
    fn event_execution_loop(
        mut wasm_module: Module, event_receiver: Receiver<Box<dyn HermesEventPayload>>,
    ) -> anyhow::Result<()> {
        for event in event_receiver {
            wasm_module.execute_event(event.as_ref())?;
        }
        Ok(())
    }

    pub(crate) fn new(app_name: String, module_bytes: &[u8]) -> anyhow::Result<Self> {
        let wasm_module = Module::new(app_name, module_bytes)?;
        let (event_sender, event_receiver) = channel();

        Ok(Self {
            wasm_module,
            event_sender,
            event_receiver,
        })
    }

    pub(crate) fn run(self) -> anyhow::Result<()> {
        let events_thread = thread::spawn(|| {
            Self::event_execution_loop(self.wasm_module, self.event_receiver).unwrap();
        });
        Ok(())
    }
}
