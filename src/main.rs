use proxy::{Base64, Pipeline, PipelineStep, STDioStep, WebsocketDestination, WebsocketSource};
use std::time::Duration;

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  -s define step           
  -h, --help     Print help

Steps:
  stdio:
  ws://
  b64:fw b64:bw
";

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", USAGE);
        std::process::exit(0);
    }

    let mut forward_steps: Vec<Box<dyn PipelineStep>> = Vec::new();
    loop {
        let step = pargs.opt_value_from_str::<&str, String>("-s").unwrap();
        if step == None {
            break;
        }
        let step = step.unwrap();
        println!("{step}");

        let res: Vec<String> = step.split(":").map(|s| s.to_string()).collect();
        let protocol = Some(res.get(0).unwrap().as_str());
        let config = Some(res.get(1).unwrap().as_str());
        match protocol {
            Some("stdio") => {
                forward_steps.push(Box::new(STDioStep::new()));
            }
            Some("ws-l") => forward_steps.push(Box::new(WebsocketSource::new(step.as_str()))),
            Some("ws") => forward_steps.push(Box::new(WebsocketDestination::new(step.as_str()))),
            Some("b64") => forward_steps.push(Box::new(Base64::new(config))),
            None | _ => {
                print!("unknown step : {}", step);
            }
        }
    }

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    let mut pipeline = Pipeline::new(forward_steps, Some(1024)).unwrap();
    #[allow(while_true)]
    while true {
        pipeline.read_source().unwrap();
        pipeline.read_destination().unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
}
