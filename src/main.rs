#[allow(unused_imports)]
use proxy::source;
use clap::{command, value_parser, Arg, ArgAction, ArgMatches, Command};


fn make_cmd() -> ArgMatches {
    let matches = command!()
        .arg_required_else_help(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("server")
                .about("run in server mode")
                .arg(
                    Arg::new("redis server address")
                        .short('r')
                        .long("redis_addr")
                        .action(ArgAction::Append)
                        .required(true),
                )
                .arg(
                    Arg::new("redis server port")
                        .short('p')
                        .long("redis_port")
                        .action(ArgAction::Append)
                        .required(true)
                        .value_parser(value_parser!(u16)),
                )
                .arg(
                    Arg::new("listen port")
                        .short('l')
                        .long("listen_port")
                        .action(ArgAction::Append)
                        .required(true)
                        .value_parser(value_parser!(u16)),
                ), // .arg(arg!([NAME]))
        )
        .subcommand(
            Command::new("client")
                .about("run in client mode")
                .arg(
                    Arg::new("server address")
                        .short('a')
                        .long("server_addr")
                        .action(ArgAction::Append)
                        .required(true),
                )
                .arg(
                    Arg::new("server port")
                        .short('p')
                        .long("server_port")
                        .action(ArgAction::Append)
                        .required(true)
                        .value_parser(value_parser!(u16)),
                )
                .arg(
                    Arg::new("unique identity")
                        .short('i')
                        .long("identity")
                        .action(ArgAction::Append)
                        .required(true)
                        .value_parser(value_parser!(u16)),
                ), // .arg(arg!([NAME]))
        )
        .get_matches();
    matches
}

fn main() {
    make_cmd();
    println!("Hello, world!");
}
