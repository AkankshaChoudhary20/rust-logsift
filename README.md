# logsift

A streaming log/CSV filter and summarizer for huge log files -- the kind
of thing you reach for `grep` + `awk` for, but purpose-built and fast on
gigabyte-scale files without loading them into memory.

## The problem this solves

`grep`-ing a multi-GB log file for errors within a time window, then
tallying how many of each severity level showed up, usually means chaining
several tools together and hoping the regex is right. `logsift` does the
filtering, time-range narrowing, and level tally in one pass, streaming
line-by-line so memory use stays flat regardless of file size.

## How it works

- Reads the file (or stdin) with a `BufReader`, one line at a time --
  never buffers the whole file.
- `--since`/`--until` filtering relies on a property of `YYYY-MM-DDTHH:MM:SS`
  timestamps: for that exact format, lexical string comparison **is**
  chronological comparison. So time filtering is a plain string compare on
  the first 19 characters of each line -- no date-parsing dependency
  needed.
- Recognizes `ERROR`/`WARN`/`INFO`/`DEBUG`/`TRACE` as whole words (not
  substrings, so `ERRORS` in a message body doesn't get miscounted) and
  tallies them for `--summary`.
- Zero external crates -- Rust standard library only.

## Usage

```bash
cargo build --release
./target/release/logsift access.log --contains ERROR --since "2024-01-05T00:00:00" --until "2024-01-10T00:00:00" --summary
```

```
usage: logsift [FILE] [--contains STR] [--exclude STR] [--since TS] [--until TS] [--summary]

FILE defaults to stdin ('-'). TS must look like YYYY-MM-DDTHH:MM:SS.
--summary prints total/matched line counts and per-level counts to stderr after processing.
```

Matched lines stream to stdout as they're found; the summary (if requested)
prints to stderr afterward, so you can pipe matched lines elsewhere while
still seeing the summary in your terminal.

## Tests

```bash
cargo test
```

11 unit tests covering filter combinations, timestamp-prefix validation
(including rejecting malformed dates), the lexical-vs-chronological
ordering assumption, whole-word level classification, and summary-count
correctness.

## Real-world scale check

Ran against a synthetic **2,000,000-line / 112 MB** log file
(`--contains ERROR --since ... --until ...  --summary`):

```
0.74s wall time, ~1.4 MB peak resident memory
34,389 / 2,000,000 lines matched
```

Memory use stayed at ~1.4 MB against a 112 MB input -- confirms it's
actually streaming, not loading the file into RAM.

## What's intentionally not here

No regex support (substring `--contains`/`--exclude` only), no gzip/zstd
decompression, no multi-file glob input. Adding any of those means picking
up a crate (`regex`, `flate2`) -- deliberately left out to keep this a
zero-dependency, fast-to-build tool.
