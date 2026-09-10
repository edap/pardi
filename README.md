# Pardi

A command line tool to catalog DICOM files. For every file it finds, it extracts a configurable set of DICOM fields (by default Patient ID and Patient Name) plus the file path.

## Build

To build the cli tool, move into the pardi folder and run `cargo build --release`. This command generates an optimized build in the `target/release` folder

To build the tool with [rayon](https://docs.rs/rayon/latest/rayon/) enabled, run `cargo build --release --features rayon`


## Usage

After the build process, in `target/release` you can find the `pardi` binary. Run it with `./pardi`. See the next sections for the available options.

## Options

- `--path` = Specify the path where to look for DICOM files. By default it tries to scan the current directory

Example: `./target/release/pardi --path /home/user/data`

- `--format` = The format of the output, json and csv are available. Default json

Example: `./target/release/pardi --path data --format csv`

- `--debug` = Print error messages for unprocessable files. Default false.

Example: `./target/release/pardi --path data --debug`

- `--output` = Save the catalog to a file. If no output option is speficied, it prints the catalogue on screen.

Example: `./target/release/pardi --path data --format json --output catalog.json`

- `--tag` = A DICOM field to extract, given by its standard dictionary alias (e.g. `PatientID`, `PatientName`, `StudyDate`, `Modality`, `SeriesInstanceUID`...). Repeatable. If omitted, defaults to `PatientID` and `PatientName`. The requested field names become the CSV column headers / JSON object keys, in the order given.

Example, extracting patient ID together with the modality and study date of each file: `./target/release/pardi --path data --format csv --tag PatientID --tag Modality --tag StudyDate`

Any alias known to the DICOM standard data dictionary works; see the [`dicom-dictionary-std`](https://docs.rs/dicom-dictionary-std) documentation for the full list, or inspect a sample file with a DICOM dump tool to see which attributes it carries. If a requested field is missing from a given file, that file is skipped and reported as an error (visible with `--debug`).

## Benchmarks

The project has a [Criterion](https://docs.rs/criterion) benchmark suite in `benches/catalog_benchmark.rs` covering DICOM file parsing.

Run it with:

```
cargo bench
```

HTML reports (with plots) are written to `target/criterion/report/index.html`.


