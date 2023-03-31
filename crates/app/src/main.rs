#[macro_use]
extern crate log;

use clap::{arg, Command};
use env_logger::Builder;
use log::LevelFilter;

fn main() {
    let matches = Command::new("graphwalker")
                          .version("0.0.1")
                          .author("Kristian Karl")
                          .about("Model-based testing tool")
                          .arg_required_else_help(true)
                          .subcommand_required(true)
                          .arg(
                            arg!(--debug <LOG_LEVEL>)
                            .help("select the log level")
                            .default_values(["error", "warn", "info", "debug", "trace"])
                            .default_missing_value("error"),
                          )
                          .subcommand(
                            Command::new("convert")
                                    .about("Convert models between different formats. The output is written to standard outpout.")
                                    .arg(arg!(<INPUT> "The file with model(s) to convert from"))
                                    .arg(
                                      arg!(--format <FORMAT>)
                                          .help("select the format to convert into")
                                          .default_values(["json", "dot"])
                                          .default_missing_value("json"),
                                  )
                          )
                          .subcommand(
                                Command::new("offline")
                                        .about("Creates a path through the models. The output is written to standard outpout.")
                                        .arg(arg!(<INPUT> "The file with model(s) to use"))
                                    )
                          .get_matches();

    if let Some(debug) = matches.get_one::<String>("debug") {
        match debug.as_str() {
            "error" => {
                Builder::new().filter_level(LevelFilter::Error).init();
            }
            "warn" => {
                Builder::new().filter_level(LevelFilter::Warn).init();
            }
            "info" => {
                Builder::new().filter_level(LevelFilter::Info).init();
            }
            "debug" => {
                Builder::new().filter_level(LevelFilter::Debug).init();
            }
            "trace" => {
                Builder::new().filter_level(LevelFilter::Trace).init();
            }
            _ => Builder::new().filter_level(LevelFilter::Error).init(),
        }
    }

    match matches.subcommand() {
        Some(("convert", convert_matches)) => {
            let file_read_result = io::read(
                convert_matches
                    .get_one::<String>("INPUT")
                    .expect("required"),
            );
            let models = match file_read_result {
                Ok(models) => models,
                Err(error) => {
                    error!("{}", &error);
                    std::process::exit(exitcode::SOFTWARE);
                }
            };

            if let Some(format) = convert_matches.get_one::<String>("format") {
                match format.as_str() {
                    "json" => {
                        io::json::write::write(models);
                    }
                    "dot" => {
                        io::dot::write::write(models);
                    }
                    _ => {
                        error!("Output format for file is not yet implemented.");
                        std::process::exit(exitcode::SOFTWARE);
                    }
                }
            }
        }

        Some(("offline", offline_matches)) => {
            let file_read_result = io::read(
                offline_matches
                    .get_one::<String>("INPUT")
                    .expect("required"),
            );
            let models = match file_read_result {
                Ok(models) => models,
                Err(error) => {
                    error!("{}", &error);
                    std::process::exit(exitcode::SOFTWARE);
                }
            };

            let mut machine = machine::Machine::new();
            let res = machine.load_models(models);
            if res.is_err() {
                error!("{}", res.err().unwrap());
                std::process::exit(exitcode::SOFTWARE);
            }

            match machine.walk() {
                Ok(_success) => {
                    let json_str = serde_json::to_string_pretty(&machine.get_profile()).unwrap();
                    println!("{}", json_str);
                }
                Err(error) => {
                    error!("{}", &error);
                    std::process::exit(exitcode::SOFTWARE);
                }
            }
        }

        _ => {
            error!("subcommand not implemented");
            std::process::exit(exitcode::SOFTWARE);
        }
    }
    std::process::exit(exitcode::OK);
}
