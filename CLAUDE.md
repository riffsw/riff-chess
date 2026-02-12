# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build --verbose                                            # Build with default features (includes random)
cargo build --no-default-features                                # Build without rand (WASM-compatible)
cargo build --target wasm32-unknown-unknown --no-default-features # Verify WASM compilation
cargo test --verbose                                             # Run all tests (54 tests)
cargo test <test_name>                                           # Run a single test by name
cargo test -- --nocapture                                        # Run tests with stdout visible
```

CI runs `cargo build --verbose && cargo test --verbose` on pushes/PRs to main.

## Cargo Features

- `default = ["random"]` — includes `rand` and `getrandom` for `BackRankId::shuffled()`, `GameId::random()`, `EngineBoard::shuffled()`
- `random` — enables random generation. Disable with `default-features = false` for WASM targets where `rand` doesn't compile without `getrandom/js`.

## Architecture

**riff-chess** is a Rust library implementing chess game logic for both standard chess and Chess960 (960 starting positions).

### Core Design: Generic Board

The central type is `Board<T>` which is generic over play mode:

- **`Board<EngineMode>`** — Server-side: plays both sides, tracks three-fold repetition, determines game results (checkmate, stalemate, draws)
- **`Board<PlayerMode>`** — Client-side: plays one color, supports pre-moves (speculative moves before opponent responds), and game review (navigating move history)

### Key Modules

- **`board/position.rs`** — `Position` holds core board state: piece placement, turn, castling rights, en passant square, move counters. `MoveId` (u16) encodes both move count and whose turn it is.
- **`board/moves.rs`** — Legal move generation. `MoveState` wraps a position with precomputed attack maps and pinned piece tracking. Move types: `LegalMove` (validated), `PreMove` (speculative).
- **`board/square.rs`** — `Square` (64 variants A1–H8), `Rank`, `File`, and `Mask` (u64 bitboard). Precomputed lookup tables via `once_cell::Lazy` for knight moves, king moves, lines, and between-square masks.
- **`board/material.rs`** — `Material` = `Piece` + `Color`. `Pair<T>` generic for white/black symmetric data. Insufficient mating material detection.
- **`board/backrank.rs`** — Chess960 back rank generation. `BackRankId` (0–959) maps to piece placement. Standard chess = ID 518.
- **`board/castling.rs`** — Castling rights and validation (works for both standard and Chess960 piece positions).
- **`board/play.rs`** — `EngineMode` and `PlayerMode` state, play logic, result detection.
- **`board/review.rs`** — `ReviewState` for navigating through game history.
- **`game/mod.rs`** — `GameId`, `GameResult`, `WinReason`, `DrawReason` enums.

### Patterns

- **Bitboard-based**: Squares and move sets represented as u64 masks for efficient computation.
- **Trait composition**: `Castling`, `BackRanks`, `Pos`, `LegalMoves`, `Review` traits compose board capabilities.
- **Precomputed tables**: Knight destinations, king destinations, lines between squares are lazily computed once and cached.
- **Serde throughout**: Most types derive `Serialize`/`Deserialize` for API/persistence use.
- **Replay methods**: `EngineBoard::replay()` and `PlayerBoard::replay()` reconstruct board state from a move list (for reconnection/persistence).
