use tracing::{error, trace};

enum Command {
    Send { payload: super::Payload },
}

type CommandSender = tokio::sync::mpsc::Sender<Command>;

type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

pub struct Handle {
    cmd_tx: CommandSender,
}

impl Handle {
    pub(super) fn send(&self, payload: super::Payload) -> Result<bool, super::Error> {
        self.cmd_tx
            .blocking_send(Command::Send { payload })
            .map_err(|_| 0u32)?;

        // TODO: Proper return type
        Ok(true)
    }
}

pub fn spawn() -> Handle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    Handle { cmd_tx }
}

fn executor(mut cmd_rx: CommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build();

    let rt = match res {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = ?err, "Failed to start Http Request Runtime Extension background thread");
            return;
        },
    };

    trace!("Created Tokio runtime for Http Request Runtime Extension");

    rt.block_on(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                Command::Send { payload: _ } => todo!(),
            }
        }
    });
}
