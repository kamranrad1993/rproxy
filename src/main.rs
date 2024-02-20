use proxy::{
    Base64Decoder, Base64Encoder, Pipeline, PipelineStep, STDioStep, WebsocketDestination,
    WebsocketSource,
};

const USAGE: &'static str = "
Usage: 
  proxy [OPTIONS]

Options:
  -f define forward step           
  -b define backward step           
  -h, --help     Print help
";

fn read_steps(flag: &str) -> Vec<Box<dyn PipelineStep>>{
    let mut pargs = pico_args::Arguments::from_env();
    let mut steps: Vec<Box<dyn PipelineStep>> = Vec::new();
    loop {
        let step = pargs.opt_value_from_str::<&str, String>("-f").unwrap();
        if step == None {
            break;
        }
        let step = step.unwrap();

        let res: Vec<String> = step.split(":").map(|s| s.to_string()).collect();
        let protocol = Some(res.get(0).unwrap().as_str());
        match protocol {
            Some("stdio") => {
                steps.push(Box::new(STDioStep::new()));
            }
            Some("ws-l") => steps.push(Box::new(WebsocketSource::new(step.as_str()))),
            Some("ws") => steps.push(Box::new(WebsocketDestination::new(step.as_str()))),
            Some("b64-enc") => steps.push(Box::new(Base64Encoder::new(None))),
            Some("b64-dec") => steps.push(Box::new(Base64Decoder::new(None))),
            None | _ => {
                print!("unknown step : {}", step);
            }
        }
    }
    steps
}

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", USAGE);
        std::process::exit(0);
    }

    for i in pargs.finish(){
        println!("{}", i.into_string().unwrap());
    }


    // let mut forward_steps: Vec<Box<dyn PipelineStep>> = Vec::new();
    // loop {
    //     let step = pargs.opt_value_from_str::<&str, String>("-s").unwrap();
    //     if step == None {
    //         break;
    //     }
    //     let step = step.unwrap();
    //     println!("{step}");

    //     let res: Vec<String> = step.split(":").map(|s| s.to_string()).collect();
    //     let protocol = Some(res.get(0).unwrap().as_str());
    //     match protocol {
    //         Some("stdio") => {
    //             forward_steps.push(Box::new(STDioStep::new()));
    //         }
    //         Some("ws-l") => forward_steps.push(Box::new(WebsocketSource::new(step.as_str()))),
    //         Some("ws") => forward_steps.push(Box::new(WebsocketDestination::new(step.as_str()))),
    //         Some("b64-enc") => forward_steps.push(Box::new(Base64Encoder::new(None))),
    //         Some("b64-dec") => forward_steps.push(Box::new(Base64Decoder::new(None))),
    //         None | _ => {
    //             print!("unknown step : {}", step);
    //         }
    //     }
    // }

    // // let mut forward_steps = read_steps("-f");
    // // let mut backward_steps = read_steps("-b");

    // let remaining = pargs.finish();
    // if !remaining.is_empty() {
    //     eprintln!("Warning: unused arguments left: {:?}.", remaining);
    // }

    // let mut pipeline = Pipeline::new(forward_steps, backward_steps, Some(1024)).unwrap();
    // while true {
    //     pipeline.read_source().unwrap();
    //     pipeline.read_destination().unwrap();
    // }
}
