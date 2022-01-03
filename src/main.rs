mod graph;
mod io;

#[macro_use]
extern crate log;

use clap::{arg, Arg, App, AppSettings};
use env_logger::Builder;
use log::LevelFilter;



fn main() {

    let matches = App::new("graphwalker")
                          .version("0.0.1")
                          .author("Kristian Karl")
                          .about("Model-based testing tool")
                          .setting(AppSettings::ArgRequiredElseHelp)
                          .arg(
                            Arg::new("debug")
                            .short('d')
                            .long("debug")
                            .help("select the log level")
                            .possible_values(["error", "warn", "info", "debug", "trace"])
                            .default_value("error"),
                          )
                          .subcommand(
                              App::new("convert")
                                      .about("Convert models between different formats. The output is written to standard outpout.")
                                      .arg(arg!(<INPUT> "The file with model(s) to convert from"))
                                      .setting(AppSettings::ArgRequiredElseHelp)
                                      .arg(
                                        Arg::new("format")
                                            .short('f')
                                            .long("format")
                                            .help("select the format to convert into")
                                            .possible_values(["json", "dot"])
                                            .default_value("json"),
                                    )
                          )
                          .get_matches();
    
    match matches.value_of("debug") {
        Some("error") => {
            Builder::new().filter_level(LevelFilter::Error).init();
        }
        Some("warn") => {
            Builder::new().filter_level(LevelFilter::Warn).init();
        }
        Some("info") => {
            Builder::new().filter_level(LevelFilter::Info).init();
        }
        Some("debug") => {
            Builder::new().filter_level(LevelFilter::Debug).init();
        }
        Some("trace") => {
            Builder::new().filter_level(LevelFilter::Trace).init();
        }
        _ => (Builder::new().filter_level(LevelFilter::Error).init()),                
    }                


    match matches.subcommand() {
        Some(("convert", convert_matches)) => {

            let models = io::read::read(convert_matches.value_of("INPUT").expect("required"));
            
            match convert_matches.value_of("format") {
                Some("json") => {
                    io::json::write::write(models);
                }
                Some("dot") => {
                     io::dot::write::write(models);      
                }
                _ => (),                
            }
        }
        _ => (),
    }
}
