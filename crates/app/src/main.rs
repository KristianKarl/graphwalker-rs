#[macro_use]
extern crate log;

use clap::{arg, Command};
use env_logger::{Builder, Target};
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
                                        .arg(arg!(--seed <NUMBER>)
                                            .help("seeds the generator with NUMBER to get predictable outputs")
                                        )
                                    )
                          .subcommand(
                                Command::new("online")
                                        .about("Starts a REST service. The generated path is fetched through the REST API.")
                                        .arg(arg!(<INPUT> "The file with model(s) to use"))
                                        .arg(arg!(--seed <NUMBER>)
                                            .help("seeds the generator with NUMBER to get predictable outputs")
                                        )
                                        .arg(arg!(--port <NUMBER>)
                                            .help("the port number of the REST service")
                                            .default_value("9090")
                                        )
                                    )
                          .get_matches();

    if let Some(debug) = matches.get_one::<String>("debug") {
        match debug.as_str() {
            "error" => {
                Builder::new()
                    .filter_level(LevelFilter::Error)
                    .target(Target::Stdout)
                    .init();
            }
            "warn" => {
                Builder::new()
                    .filter_level(LevelFilter::Warn)
                    .target(Target::Stdout)
                    .init();
            }
            "info" => {
                Builder::new()
                    .filter_level(LevelFilter::Info)
                    .target(Target::Stdout)
                    .init();
            }
            "debug" => {
                Builder::new()
                    .filter_level(LevelFilter::Debug)
                    .target(Target::Stdout)
                    .init();
            }
            "trace" => {
                Builder::new()
                    .filter_level(LevelFilter::Trace)
                    .target(Target::Stdout)
                    .init();
            }
            _ => Builder::new()
                .filter_level(LevelFilter::Error)
                .target(Target::Stdout)
                .init(),
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
                        let res = io::json_write::write(models);
                        match res {
                            Ok(_) => {}
                            Err(why) => {
                                error!("{:?}", why);
                                std::process::exit(exitcode::SOFTWARE);
                            }
                        }
                    }
                    "dot" => {
                        io::dot_write::write(models);
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
                error!("{:?}", res.err());
                std::process::exit(exitcode::SOFTWARE);
            }

            if let Some(number_str) = offline_matches.get_one::<String>("seed") {
                match number_str.parse::<u64>() {
                    Ok(number) => machine.seed(number),
                    Err(error) => {
                        error!("{}", &error);
                        std::process::exit(exitcode::SOFTWARE);
                    }
                };
            }

            match machine.walk() {
                Ok(()) => std::process::exit(exitcode::OK),
                Err(error) => {
                    error!("{}", &error);
                    std::process::exit(exitcode::SOFTWARE);
                }
            }
        }

        Some(("online", offline_matches)) => {
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
                error!("{:?}", res.err());
                std::process::exit(exitcode::SOFTWARE);
            }

            if let Some(number_str) = offline_matches.get_one::<String>("seed") {
                match number_str.parse::<u64>() {
                    Ok(number) => machine.seed(number),
                    Err(error) => {
                        error!("{}", &error);
                        std::process::exit(exitcode::SOFTWARE);
                    }
                };
            }

            match machine.reset() {
                Ok(()) => rest::run_rest_service(machine),
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
