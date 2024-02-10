use clap::{command, ArgMatches, Command};
#[allow(unused_imports)]
use proxy::{cmd::Cmd, Pipeline, PipelineStep, STDioStep, WebsocketDestination, WebsocketSource};

// #[derive(EnumIter)]
// enum Steps {
//     StdioStep(STDioStep),
//     WebsocketSource(WebsocketSource),
//     WebsocketDestination(WebsocketDestination),
// }

// enum StepsCmd{
//     StdioStep(STDioStep::get_cmd())
// }

fn make_cmd() -> ArgMatches {
    let mut commands: Command = command!();
    commands = Pipeline::get_cmd(commands).unwrap();
    commands = STDioStep::get_cmd(commands).unwrap();
    commands = WebsocketSource::get_cmd(commands).unwrap();
    commands = WebsocketDestination::get_cmd(commands).unwrap();

    return commands.get_matches();
}

fn main() {
    let args = make_cmd();
    println!("Hello, world!");
}
