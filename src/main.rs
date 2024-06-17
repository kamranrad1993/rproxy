use openssl::conf;
use proxy::{
    Base64, Entry, Pipeline, PipelineStep, STDioEntry, STDioStep, TCPEntry, TCPStep,
    WebsocketDestination, WebsocketEntry, WssDestination, RSult, TcpEntryNonBlocking,
    WSEntryNonBlocking
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  -e entry
  -s define step 
  -t loop_time(default is 10ms)          
  -h, --help     Print help

Entries:
  ws://
  stdio:
  tcp://

Steps:
  stdio:
  ws://
  b64:fw b64:bw
  tcp://
  salt:fw-len salf:bw-len
";

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", USAGE);
        std::process::exit(0);
    }

    let mut steps: Vec<Box<dyn PipelineStep>> = Vec::new();
    loop {
        let step = pargs.opt_value_from_str::<&str, String>("-s").unwrap();
        if step == None {
            break;
        }
        let step = step.unwrap();
        println!("step : {step}");

        let res: Vec<String> = step.split(":").map(|s| s.to_string()).collect();
        let protocol = Some(res.get(0).unwrap().as_str());
        let config = Some(res.get(1).unwrap().as_str());
        match protocol {
            Some("stdio") => {
                steps.push(Box::new(STDioStep::new()));
            }
            Some("ws") => steps.push(Box::new(WebsocketDestination::new(step.as_str()))),
            Some("wss") => steps.push(Box::new(WssDestination::new(step.as_str()))),
            Some("b64") => steps.push(Box::new(Base64::new(config))),
            Some("tcp") => steps.push(Box::new(TCPStep::new(step.as_str()))),
            Some("salt") => steps.push(Box::new(RSult::new(config))),
            None | _ => {
                print!("unknown step : {}", step);
            }
        }
    }
    let pipeline = Pipeline::new(steps, Some(1024));

    let loop_time: u64 = pargs.opt_value_from_str::<&str, u64>("-t").unwrap().unwrap_or(10);

    let entry = pargs.opt_value_from_str::<&str, String>("-e").unwrap();
    if entry == None {
        panic!("no entry defined");
    }
    let entry = entry.unwrap();
    println!("entry : {entry}");

    let res: Vec<String> = entry.split(":").map(|s| s.to_string()).collect();
    let protocol = Some(res.get(0).unwrap().as_str());
    let config = Some(res.get(1).unwrap().as_str());
    match protocol {
        Some("ws") => {
            let mut entry = WSEntryNonBlocking::new(entry, pipeline, loop_time);
            entry.listen();
        }
        Some("stdio") => {
            let mut entry = STDioEntry::new(String::new(), pipeline, loop_time);
            entry.listen();
        }
        Some("tcp") => {
            let mut entry = TcpEntryNonBlocking::new(entry, pipeline, loop_time);
            entry.listen();
        }
        None | _ => {
            panic!("unknown entry : {}", entry);
        }
    }

    // let remaining = pargs.finish();
    // if !remaining.is_empty() {
    //     eprintln!("Warning: unused arguments left: {:?}.", remaining);
    // }
}
