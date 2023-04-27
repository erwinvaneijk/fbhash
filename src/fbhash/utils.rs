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
