use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

use logsift::{Filters, Summary, process_line};

fn print_usage() {
    eprintln!(
        "usage: logsift [FILE] [--contains STR] [--exclude STR] [--since TS] [--until TS] [--summary]\n\n\
         FILE defaults to stdin ('-'). TS must look like YYYY-MM-DDTHH:MM:SS.\n\
         --summary prints total/matched line counts and per-level counts to stderr after processing."
    );
}

struct Args {
    file: Option<String>,
    filters: Filters,
    summary: bool,
}

fn parse_args(argv: Vec<String>) -> Result<Args, String> {
    let mut file = None;
    let mut filters = Filters::default();
    let mut summary = false;

    let mut it = argv.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--contains" => filters.contains = Some(it.next().ok_or("--contains needs a value")?),
            "--exclude" => filters.exclude = Some(it.next().ok_or("--exclude needs a value")?),
            "--since" => filters.since = Some(it.next().ok_or("--since needs a value")?),
            "--until" => filters.until = Some(it.next().ok_or("--until needs a value")?),
            "--summary" => summary = true,
            "-h" | "--help" => return Err("help".to_string()),
            other if !other.starts_with("--") => file = Some(other.to_string()),
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    Ok(Args {
        file,
        filters,
        summary,
    })
}

fn run(args: Args) -> io::Result<Summary> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut summary = Summary::default();

    let reader: Box<dyn BufRead> = match args.file.as_deref() {
        None | Some("-") => Box::new(BufReader::new(io::stdin())),
        Some(path) => Box::new(BufReader::new(File::open(path)?)),
    };

    for line in reader.lines() {
        let line = line?;
        if process_line(&line, &args.filters, &mut summary) {
            writeln!(out, "{line}")?;
        }
    }

    Ok(summary)
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().skip(1).collect();

    let args = match parse_args(argv) {
        Ok(a) => a,
        Err(e) => {
            if e != "help" {
                eprintln!("error: {e}");
            }
            print_usage();
            return ExitCode::from(if e == "help" { 0 } else { 2 });
        }
    };

    let want_summary = args.summary;
    match run(args) {
        Ok(summary) => {
            if want_summary {
                eprintln!(
                    "\n-- summary: {} / {} lines matched",
                    summary.matched_lines, summary.total_lines
                );
                let mut levels: Vec<_> = summary.level_counts.iter().collect();
                levels.sort_by_key(|(k, _)| k.to_string());
                for (level, count) in levels {
                    eprintln!("   {level}: {count}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
