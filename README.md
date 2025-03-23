## Build

To build the cli tool, move into the pardi folder and run `cargo build --release`. This command generates an optimized build in the `target/release` folder

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



## Note

This file has been testes only on Linux environment, as I do not have access at the moment to a windows or macOS machine.