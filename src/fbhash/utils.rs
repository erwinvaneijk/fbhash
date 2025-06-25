// This file is part of fbhash
//
// Copyright (C) 2025 Erwin J. van Eijk
//
// Licensed under the EUPL, Version 1.2 or – as soon they will be approved by the European Commission - subsequent versions of the EUPL.
// You may not use this work except in compliance with the License.
// You may obtain a copy of the License at:
// https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use indicatif::{ProgressBar, ProgressStyle};

#[derive(Clone, Debug, Copy)]
pub enum OutputFormat {
    Json,
    Binary,
}

#[derive(Clone, Debug, Copy)]
pub struct Configuration {
    pub output_format: OutputFormat,
    pub quiet: bool,
}

impl Configuration {
    #[allow(dead_code)]
    pub fn new(output_format: OutputFormat, quiet: bool) -> Configuration {
        Configuration {
            output_format,
            quiet,
        }
    }
}

pub fn create_progress_bar(size: u64, config: &Configuration) -> ProgressBar {
    if !config.quiet {
        let style = ProgressStyle::default_bar()
            .template("[{elapsed_precise} {eta}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap();
        ProgressBar::new(size).with_style(style)
    } else {
        ProgressBar::hidden()
    }
}
