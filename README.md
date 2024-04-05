## TCHunt

A pretty basic reimplementation of the TCHunt utility, written in Rust.

### How does it work

It scans the folder provided as an argument for all files that match the following conditions:

- Size is evenly divisible by 512
- Size is greater than or equal to 2KB
- Shannon entropy is greater than or equal to 7.9
- The file type is not recognized by the `infer` library, though the recognized matches are printed on the TRACE log level

### Installing

- Install cargo
- Clone the repo and go into the directory
- `cargo install --path .`
- `tchunt-rs .` <- searches the current directory and recurses

#### Logging

To view more details while it is running, you can call it like this:

`RUST_LOG=tchunt_rs=trace tchunt-rs .`
