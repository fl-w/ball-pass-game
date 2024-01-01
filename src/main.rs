use crate::server::GameServer;
use argh::FromArgs;
use futures_util::future::{AbortHandle, Abortable};
use std::error::Error;

mod client;
mod encoding;
mod events;
mod game;
mod message;
mod server;
mod session;
mod utils;

/// A socket-based client-server system to play a virtual ball.
#[derive(FromArgs)]
struct Opt {
    #[argh(subcommand)]
    cmd: Option<SubOpt>,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum SubOpt {
    Client(client::CliOpts),
    Server(server::CliOpts),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli: Opt = argh::from_env();

    // set default command to 'client'
    let cmd = cli
        .cmd
        .unwrap_or_else(|| SubOpt::Client(client::CliOpts::default()));

    match cmd {
        SubOpt::Client(opt) => {
            let addr = opt.server_addr;
            let mut app = client::start(addr).await;

            // listen for ctrl_c
            let tx = app.sender();
            let ctrlc = async move {
                let _ = tokio::signal::ctrl_c().await;

                println!("✨ Ctrl-C received. Stopping..");
                tx.send_with_urgency(client::Event::CtrlC)
            };

            let (ctrlc_abort_handle, abort_registration) = AbortHandle::new_pair();
            tokio::spawn(Abortable::new(ctrlc, abort_registration));

            app.run_loop().await;
            ctrlc_abort_handle.abort();
        }

        SubOpt::Server(opts) => {
            let addr = format!("127.0.0.1:{}", opts.port);
            let server = GameServer::default();

            // listen for ctrl_c
            let tx = server.sender().clone();
            let ctrlc = async move {
                let _ = tokio::signal::ctrl_c().await;

                println!("✨ Ctrl-C received. Stopping..");
                tx.send_with_urgency(server::Event::CtrlC)
            };

            let (ctrlc_abort_handle, abort_registration) = AbortHandle::new_pair();
            tokio::spawn(Abortable::new(ctrlc, abort_registration));

            server.run(&addr).await?;
            ctrlc_abort_handle.abort();
        }
    };

    Ok(())
}
