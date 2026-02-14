// Copyright 2026 Tobin Edwards
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

#[cfg(feature = "random")]
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::Color;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GameId(u64);

impl GameId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    #[cfg(feature = "random")]
    pub fn random() -> Self {
        Self(thread_rng().gen())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GameResult {
    Win(Color, WinReason),
    Draw(DrawReason),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WinReason {
    CheckMate,
    TimeExpired,
    Resigned,
    Abandoned,
    // In Armageddon Chess, there is no draw. So if a draw
    // state is reached, Black wins
    Draw(DrawReason),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DrawReason {
    Agreed,
    StaleMate,
    Repetition,
    FiftyMoves,
    Insufficient,
}
