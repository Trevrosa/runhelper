# runhelper

this project contains a `helper` and a `runner`.

## the `helper`
the `helper` is meant to be exposed to the internet and lets users control `runner` limitedly.

## the `runner`
the `runner` only talks to the `helper` through the local network. it runs a minecraft server on demand and broadcasts system metrics and the server console via websocket.

## `runner` & `helper`
the `helper` runs at all times, while the `runner` runs sometimes.

when requested to, a `helper` can wake a `runner`.

the `helper` then facilitates communication from the internet to the `runner`.

## running them
for more information, see each crate's readme ([`helper`](./helper/README.md) & [`runner`](./runner/README.md))
