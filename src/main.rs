// This file is part of fbhash
//
// Copyright (C) 2025 Erwin J. van Eijk
//
// Licensed under the EUPL, Version 1.2 or – as soon they will be approved by the European Commission - subsequent versions of the EUPL.
// You may not use this work except in compliance with the License.
// You may obtain a copy of the License at:
// https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12

#[cfg(test)]
extern crate pretty_assertions;
#[cfg(test)]
#[macro_use]
extern crate float_cmp;
extern crate clap;
mod fbhash;

use clap::{arg, value_parser, Arg, ArgAction, Command};
use fbhash::index::*;
use fbhash::query::*;
use fbhash::utils::{Configuration, OutputFormat};
use std::path::PathBuf;

fn file_arguments() -> Vec<clap::Arg> {
    vec![
        arg!(-d --database <DATABASE_FILE>)
            .value_parser(value_parser!(PathBuf))
            .default_value("database.json"),
        arg!(-s --state <STATE_FILE>)
            .value_parser(value_parser!(PathBuf))
            .default_value("state.json"),
    ]
}

fn main() -> std::io::Result<()> {
    let matches = Command::new("fbhash")
        .version("0.1.0")
        .author("Erwin van Eijk")
        .about("Find near duplicates of files")
        .subcommand_required(true)
        .arg(
            arg!(-j --json "Output the results in json format")
                .conflicts_with("binary")
                .action(ArgAction::SetTrue),
        )
        .arg(
            arg!(-b --binary "Output the results in binary format")
                .conflicts_with("json")
                .action(ArgAction::SetTrue),
        )
        .arg(arg!(-q --quiet "Suppress all output but the end result").action(ArgAction::SetTrue))
        .subcommand(
            Command::new("index").args(file_arguments()).arg(
                arg!(<INPUT> ... "Path to directories to process")
                    .required(true)
                    .value_parser(value_parser!(PathBuf))
                    .num_args(1..)
                    .action(ArgAction::Append),
            ),
        )
        .subcommand(
            Command::new("query")
                .arg(
                    Arg::new("RESULT_SIZE")
                        .short('n')
                        .long("number")
                        .required(false)
                        .value_parser(value_parser!(usize))
                        .help("How many results to return")
                        .action(ArgAction::Append),
                )
                .args(file_arguments())
                .arg(
                    Arg::new("FILE_TO_QUERY")
                        .required(true)
                        .help("The file to query in the index")
                        .required(true)
                        .value_parser(value_parser!(PathBuf))
                        .action(ArgAction::Append)
                        .num_args(1..),
                ),
        )
        .get_matches();

    let output_format = if matches.get_flag("binary") {
        OutputFormat::Binary
    } else {
        OutputFormat::Json
    };
    let quiet =
        matches.get_flag("quiet") || !console::user_attended() || !console::user_attended_stderr();
    let config = Configuration::new(output_format, quiet);

    if let Some(subcommand_matches) = matches.subcommand_matches("index") {
        let paths: Vec<&PathBuf> = subcommand_matches
            .get_many::<PathBuf>("INPUT")
            .unwrap()
            .collect::<Vec<&PathBuf>>();
        let output_state_file = subcommand_matches
            .get_one::<PathBuf>("state")
            .expect("collection_state.json");
        let results_file = subcommand_matches
            .get_one::<PathBuf>("database")
            .expect("database.json");

        index_paths(paths.as_slice(), output_state_file, results_file, &config)?;
    } else if let Some(query_subcommand_matches) = matches.subcommand_matches("query") {
        let files: Vec<&PathBuf> = query_subcommand_matches
            .get_many::<PathBuf>("FILE_TO_QUERY")
            .unwrap()
            .collect::<Vec<&PathBuf>>();
        let database_path = query_subcommand_matches
            .get_one::<PathBuf>("database")
            .unwrap();
        let state_path = query_subcommand_matches
            .get_one::<PathBuf>("state")
            .unwrap();
        let number_of_results = *query_subcommand_matches
            .get_one::<usize>("RESULT_SIZE")
            .unwrap_or(&(5_usize));
        query_for_results(
            state_path,
            database_path,
            &files,
            number_of_results,
            &config,
        )?;
    }
    Ok(())
}
