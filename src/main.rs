// Copyright 2021, Erwin van Eijk
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[cfg(test)]
#[macro_use]
extern crate float_cmp;
extern crate clap;
mod fbhash;

use clap::{App, Arg, SubCommand};
use fbhash::index::*;
use fbhash::query::*;
use fbhash::utils::OutputFormat;

fn main() -> std::io::Result<()> {
    let matches = App::new("fbhash")
        .version("0.1.0")
        .author("Erwin van Eijk")
        .about("Find near duplicates of files")
        .arg(
            Arg::with_name("json")
                .short('j')
                .long("json")
                .takes_value(false)
                .help("Output the results in json format"),
        )
        .arg(
            Arg::with_name("binary")
                .short('b')
                .long("binary")
                .takes_value(false)
                .help("Output the results in a binary format"),
        )
        .subcommand(
            SubCommand::with_name("index")
                .arg(
                    Arg::with_name("INPUT")
                        .required(true)
                        .index(1)
                        .help("Path to directories to process")
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("STATE_FILE")
                        .long("state")
                        .short('s')
                        .value_name("STATE_FILE")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("DATABASE_FILE")
                        .long("database")
                        .short('d')
                        .value_name("DATABASE_FILE")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("query")
                .arg(
                    Arg::with_name("RESULT_SIZE")
                        .short('n')
                        .long("number")
                        .required(false)
                        .require_equals(true)
                        .help("How many results to return")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("DATABASE_FILE")
                        .required(true)
                        .help("The database file to query against")
                        .index(1),
                )
                .arg(
                    Arg::with_name("STATE_FILE")
                        .required(true)
                        .help("The file containing the frequency states.")
                        .index(2)
                        .required(true),
                )
                .arg(
                    Arg::with_name("FILE_TO_QUERY")
                        .required(true)
                        .help("The file to query in the index")
                        .index(3)
                        .required(true)
                        .multiple(true),
                ),
        )
        .get_matches();

    let output_format = if matches.is_present("binary") {
        OutputFormat::Binary
    } else {
        OutputFormat::Json
    };
    if let Some(subcommand_matches) = matches.subcommand_matches("index") {
        let paths: Vec<_> = subcommand_matches.values_of("INPUT").unwrap().collect();
        let output_state_file = subcommand_matches
            .value_of("STATE_FILE")
            .unwrap_or("collection_state.json");
        let results_file = subcommand_matches
            .value_of("DATABASE_FILE")
            .unwrap_or("database.json");

        index_paths(&paths, output_state_file, results_file, output_format)?;
    } else if let Some(query_subcommand_matches) = matches.subcommand_matches("query") {
        let files: Vec<_> = query_subcommand_matches
            .values_of("FILE_TO_QUERY")
            .unwrap()
            .collect();
        let database_path = query_subcommand_matches.value_of("DATABASE_FILE").unwrap();
        let state_path = query_subcommand_matches.value_of("STATE_FILE").unwrap();
        let number_of_results_str = query_subcommand_matches
            .value_of("RESULT_SIZE")
            .unwrap_or("5");
        match number_of_results_str.parse::<usize>() {
            Err(v) => panic!(
                "Not a valid numerical value found for result_size argument {}",
                v
            ),
            Ok(number_of_results) => query_for_results(
                state_path,
                database_path,
                &files,
                number_of_results,
                output_format,
            )?,
        }
    }
    Ok(())
}
