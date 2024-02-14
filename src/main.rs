use clap::{command, ArgMatches, Command};
use docopt::Docopt;
#[allow(unused_imports)]
use proxy::{cmd::Cmd, Pipeline, PipelineStep, STDioStep, WebsocketDestination, WebsocketSource};

fn make_cmd() -> ArgMatches {
    let mut commands: Command = command!();
    commands = Pipeline::get_cmd(commands).unwrap();
    commands = STDioStep::get_cmd(commands).unwrap();
    commands = WebsocketSource::get_cmd(commands).unwrap();
    commands = WebsocketDestination::get_cmd(commands).unwrap();

    return commands.get_matches();
}

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  --stdin       stdio           
  --websocket   websocket    
  -h, --help     Print help
  -v, --version  Print version
";

// let argv = || vec!["cp", "-a", "file1", "file2", "dest/"];

fn main() {
    let argv = || vec!["-in", "-ws", "-ws-l"];
    let args = Docopt::new(USAGE)
        // .and_then(|dopt| dopt.parse())
        .and_then(|dopt| {
            dopt.parse()
        })
        .unwrap();
    // .unwrap_or_else(|e| e.exit());
    println!("{:?}", args);

    // let args = make_cmd();
    // for id in args.ids() {
    //     println!("{}", id)
    // }

    // let mut steps: Vec<Box<dyn PipelineStep>> = Vec::new();
    // steps.push(Box::new(STDioStep::new()));
    // // steps.push(Box::new(STDioStep::new()));
    // steps.push(Box::new(WebsocketDestination::new("ws://127.0.0.1:6666")));
    // let mut pipeline = Pipeline::new(steps, Some(1024)).unwrap();
    // while true {
    //     pipeline.read_source().unwrap();
    //     pipeline.read_destination().unwrap();
    // }
}
