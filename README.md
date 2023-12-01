# riff_chess
A Rust library for interactive chess apps

`riff_chess` is a Rust library designed to support building interactive
chess apps. It's a work-in-progress and currently supports game logic for
both standard chess and chess960. I do have plans to build a front-end 
(using Dioxus) and a back-end (using SurrealDb) all in Rust. 

## Features

`riff_chess` includes the following features:

- [x] Standard chess rules
- [x] Chess960 rules
- [x] Automatic handling of pre-moves
- [x] Three-fold repetition rule enforcement
- [ ] Five-fold repetition rule (to be implemented)
- [x] Fifty-move rule
- [x] Recognition of insufficient mating material (using chess.com's heuristics)
- [ ] Time Controls (to be implemented)
- [x] Engine mode (plays both sides and runs on server)
- [x] Player mode (plays one side and runs on client)
- [x] Review prior positions
- [ ] Take backs (to be implemented)
- [ ] Other chess variants like Crazyhouse, 3-Check, etc. (to be implemented)


## Getting Started

Include `riff_chess` in your Cargo.toml:

```toml
[dependencies]
riff_chess = "0.1.0"