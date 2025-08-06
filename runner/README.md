# runner

this is meant to be run on the computer that will be running the minecraft server.

should only be accessible locally.

## configuration

the binary can be run with the argument `--wd` to set the program's working directory.

the env var `RUNNER_PORT` controls the port of the server (default `4321`)

the env var `SERVER_DIR` should be the path to the server.

the env var `SHOW_CONSOLE` (`true` or `false`) controls whether or not the minecraft server's console is shown in the runner's stdout. (default `false`)
