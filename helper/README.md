# helper

this is meant to run on the other computer, and is accessible to the internet

the `/static` files can be precompressed with [brotli](https://github.com/google/brotli)

## configuration

the env var `RUNNER_ADDR` should be the (local) address of the runner.

the env var `RUNNER_PORT` should be the port of the runner (default `4321`)

the env var `PHYS_ADDR` should be set to the physical (mac) address of the runner, written in hexadecimal bytes separated by `-`. example: `00-1A-2B-3C-4D-5E`
