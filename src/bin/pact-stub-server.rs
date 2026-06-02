use std::env;
use std::process::ExitCode;
use mimalloc::MiMalloc;
use pact_stub_server::handle_command_args;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), ExitCode> {
    let args: Vec<String> = env::args().collect();
    handle_command_args(args).await
}