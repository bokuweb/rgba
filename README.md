# rusty-gba

[![GitHub Actions Status](https://github.com/bokuweb/rusty-gba/workflows/Continuous%20Integration/badge.svg)](https://github.com/bokuweb/rusty-gba/actions)

## setup

```
$ brew install sdl2
```

Please add following to .bashrc if M1

```
export LIBRARY_PATH="$LIBRARY_PATH:$(brew --prefix)/lib"
```

```
cargo run ./fixtures/hello/hello.gba