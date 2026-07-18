# helper

this is meant to run on the other computer, and is accessible to the internet

## how to run
 
1. download the `helper` binary from [releases](https://github.com/Trevrosa/runhelper/releases) or build it from source `cargo install --path .`
2. [configure it](#configuration)
3. if running as a service, ensure the `/static` directory is at the working directory
4. the `/static` files should be precompressed with [brotli](https://github.com/google/brotli) (`brotli -8 static/*`)

## configuration

### authentication

there are three auth levels: [`unauthed`](./src/api/mod.rs:35), [`basic`](./src/api/mod.rs:72), and [`stop`](./src/api/mod.rs:81).

`unauthed` does not require authentication, any user that has access to the `helper` is at this auth level.

`basic` requires the user to input the `BASIC_TOKEN`, this should be given to players of the server

`stop` required the user to input the `STOP_TOKEN`, this should be given to trusted users/players of the server

### environment variables

- `RUNNER_ADDR` should be the (local) address of the `runner`. (required)
- `PHYS_ADDR` should be set to the physical (mac) address of the `runner`, written in hexadecimal bytes separated by `-`. example: `00-1A-2B-3C-4D-5E` (required)
- `BASIC_TOKEN` is the token that gives access to [basic](./src/api/mod.rs:72) functions (required)
- `STOP_TOKEN` is the token that gives access to [stop/wake](./src/api/mod.rs:81) functions (required)
- `RUNNER_PORT` should be the port of the `runner` (optional, default `4321`)
- `HELPER_PORT` can be used to set the port of the `helper` (optional, default `1234`)
- `RUST_LOG` can be set to change the [log level](https://docs.rs/tracing/latest/tracing/struct.Level.html#implementations) of the `helper` (optional, default `info`)

## ai use

ai was only used for the [static web-app](./static)