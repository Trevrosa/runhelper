# runner

this is meant to be run on the computer that will be running the minecraft server.

should only be accessible locally.

## configuration

the binary can be run with the argument `--wd` to set the program's working directory.

### environment variables

- `SERVER_DIR` should be the path to the server.
- `RUNNER_PORT` controls the port of the server (optional, default `4321`)
- `SHOW_CONSOLE` (`true` or `false`) controls whether or not the minecraft server's console is shown in the runner's stdout. (optional, default `false`)
- `PAPER_ARGS` sets the jvm args to be used when running any paper server. args should be space-separated. (optional)

## notes

- for `forge` servers, `runner` respects the `user_jvm_args.txt` file at the server directory.
- for `paper` servers, if the file `user_jvm_args.txt` exists at the server directory, it takes precedence over `PAPER_ARGS`.
