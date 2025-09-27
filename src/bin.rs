use std::env;
use std::process::ExitCode;
use pact_stub_server_cli::handle_command_args;

#[tokio::main]
async fn main() -> Result<(), ExitCode> {
    let args: Vec<String> = env::args().collect();
    handle_command_args(args).await
}