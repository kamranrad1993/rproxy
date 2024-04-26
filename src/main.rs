use proxy::{
    Base64, Entry, Pipeline, PipelineStep, STDioStep, WebsocketDestination, WebsocketEntry,
    WssDestination,
};
use std::{str::FromStr, sync::{Arc, Mutex}, thread, time::Duration};

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  -e entry
  -s define step           
  -h, --help     Print help

Entries:
  ws://

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
            None | _ => {
                print!("unknown step : {}", step);
            }
        }
    }
    let mut pipeline = Pipeline::new(steps, Some(1024));

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
            // let mut entry = Arc::new(Mutex::new(WebsocketEntry::new(entry, pipeline)));
            let mut entry = WebsocketEntry::new(entry, pipeline);
            entry.listen();

            // let cloned_entry = entry.clone();
            // let listener_thread = thread::spawn(move || {
            //     let mut locked_entry = cloned_entry.lock().unwrap();
            //     locked_entry.listen();
            // });
            
            // loop {
            //     let mut locked_entry = entry.lock().unwrap();
            //     locked_entry.read();
            //     locked_entry.write();
            // }

            // let read_write_thread = thread::spawn(move || {
            //     let mut locked_entry = entry.lock().unwrap();
            //     loop {
            //         locked_entry.read();
            //         locked_entry.write();
            //     }
            // });

            // listener_thread.join().unwrap();
            // read_write_thread.join().unwrap();
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
