use std::slice::SliceIndex;

use proxy::{
    Base64Decoder, Base64Encoder, Pipeline, PipelineStep, STDioStep, WebsocketDestination,
    WebsocketSource, PipelineDirection
};

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  -f define forward step           
  -b define backward step           
  -h, --help     Print help
";

// struct Args {
//     buffer_size: usize,
//     forward_steps: Vec<Box<dyn PipelineStep>>,
//     backward_steps: Vec<Box<dyn PipelineStep>>,
// }

// enum StepDirecion {
//     Forward,
//     Backward,
//     Both,
//     Unknown,
// }

// fn read_step_direction(step: String) -> (String, StepDirecion) {
//     let direction = step.get(0..2).unwrap();
//     let mut step_direction = StepDirecion::Unknown;
//     let step = String::from(&step[1..]);
//     match direction {
//         "f:" => {
//             step_direction = StepDirecion::Forward;
//         }
//         "b:" => {
//             step_direction = StepDirecion::Backward;
//         }
//         "c:" => {
//             step_direction = StepDirecion::Both;
//         }
//         _ => {
//             step_direction = StepDirecion::Unknown;
//         }
//     }

//     (step, step_direction)
// }

// fn read_step(step: String) -> Option<(StepDirecion, Box<dyn PipelineStep>)> {
//     let (step, direction) = read_step_direction(step);
//     let res: Vec<String> = step.split(":").map(|s| s.to_string()).collect();
//     let protocol = Some(res.get(0).unwrap().as_str());
//     match protocol {
//         Some("stdio") => Some((direction, Box::new(STDioStep::new()))),
//         Some("ws-l") => Some((direction, Box::new(WebsocketSource::new(step.as_str())))),
//         Some("ws") => Some((
//             direction,
//             Box::new(WebsocketDestination::new(step.as_str())),
//         )),
//         Some("b64-enc") => Some((direction, Box::new(Base64Encoder::new(None)))),
//         Some("b64-dec") => Some((direction, Box::new(Base64Decoder::new(None)))),
//         None | _ => {
//             print!("unknown step : {}", step);
//             None
//         }
//     }
// }

// fn read_args() -> Option<Args> {
//     let mut pargs = pico_args::Arguments::from_env();

//     if pargs.contains(["-h", "--help"]) {
//         print!("{}", USAGE);
//         std::process::exit(0);
//     }

//     let mut args = Args {
//         backward_steps: Vec::new(),
//         forward_steps: Vec::new(),
//         buffer_size: 1024,
//     };

//     loop {
//         let arg = pargs.opt_free_from_str::<String>();
//         match arg {
//             Ok(arg) => {
//                 if arg == None {
//                     break;
//                 }
//                 let arg = arg.unwrap();
//                 if arg.starts_with('-') {
//                 } else {
//                     let (direction, step) = read_step(arg).unwrap();
//                     match direction {
//                         StepDirecion::Forward => args.forward_steps.push(step),
//                         StepDirecion::Backward => args.backward_steps.push(step),
//                         StepDirecion::Unknown => {
//                             panic!("{}", "unknown direction");
//                         }
//                         StepDirecion::Both => {
//                             args.forward_steps.push(step);
//                             args.backward_steps.push(step);
//                         }
//                     }
//                 }
//             }
//             Err(e) => {
//                 panic!("{e}");
//             }
//         }
//     }
//     Some(args)
// }

fn main() {
    // let args = read_args().unwrap();
    // let mut pipeline = Pipeline::new(
    //     args.forward_steps,
    //     args.backward_steps,
    //     Some(args.buffer_size),
    // )
    // .unwrap();
    // while true {
    //     pipeline.read_source().unwrap();
    //     pipeline.read_destination().unwrap();
    // }

    let mut pargs = pico_args::Arguments::from_env();
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
            Some("b64-enc") => forward_steps.push(Box::new(Base64Encoder::new(config, None))),
            Some("b64-dec") => forward_steps.push(Box::new(Base64Decoder::new( config, None))),
            None | _ => {
                print!("unknown step : {}", step);
            }
        }
    }

    // let mut forward_steps = read_steps("-f");
    // let mut backward_steps = read_steps("-b");

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    let mut pipeline = Pipeline::new(forward_steps,  Some(1024)).unwrap();
    while true {
        pipeline.read_source().unwrap();
        pipeline.read_destination().unwrap();
    }
}
