# runner

**only tested and builds on windows**

this is meant to be run on the computer that will be running the game server.

should only be accessible locally.

## how to run

1. download the `runner` binary from [releases](https://github.com/Trevrosa/runhelper/releases) or build it from source `cargo install --path .` 
2. [configure it](#configuration), reading the [game-specific notes](#game-specific-notes)
3. use [nssm](https://nssm.cc/usage) or create your own service or run it in a terminal
4. if using nssm, ensure it only stops the `runner` by control c (in the gui, ensure only `Shutdown > Generate Control-C` is enabled)  

## configuration

the binary can be run with the argument `--wd` to set the program's working directory.

### environment variables

- `SERVER_DIR` should be the path to the game server (required)
- `SERVER_TYPE` should be the type of game server (`minecraft`, `terraria`, `satisfactory`, etc.) (optional, default `minecraft`)
- `RUNNER_PORT` controls the `runner`'s port (optional, default `4321`)
- `SHOW_CONSOLE` (`true` or `false`) controls whether or not the game server's console is shown in the `runner`'s stdout. (optional, default `false`)
- `RUST_LOG` can be set to change the [log level](https://docs.rs/tracing/latest/tracing/struct.Level.html#implementations) of the `helper` (optional, default `info`)
- `GAME_ARGS` sets the args to be used when running a game server. args must be separated with a backslash (`\`). (optional)
- `STEAM_APIKEY` sets your [steamworks web api key](https://partner.steamgames.com/doc/webapi_overview/auth) to use to search mods for tmodloader (required if `SERVER_TYPE` is `terraria`)

## game-specific notes

### minecraft

- `GAME_ARGS` sets the jvm args.
- for `paper`, `forge`, and `vanilla`, you can create `user_jvm_args.txt` at the server directory, taking precedence over `GAME_ARGS`.

### terraria

- you can create a [`terrariaConfig.txt`](https://terraria.wiki.gg/wiki/Guide:Setting_up_a_Terraria_server#Making_a_configuration_file) at the `runner`'s working directory, taking precedence over `GAME_ARGS`.