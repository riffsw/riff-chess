// Copyright 2024 Tobin Edwards
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

use std::fmt::Display;
use rand::{thread_rng, Rng};
use thiserror::Error;
use anyhow::Result;
use std::ops::{Index, IndexMut};
use once_cell::sync::Lazy;
use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};

use super::square::File;
use super::material::Piece;
use Piece::{King, Queen, Rook, Bishop, Knight, Pawn};

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum BackRankError {
    #[error("Expecting 1 king, 1 queen, and 2 of each other piece")]
    ArgError,
    #[error("Bishops must be placed on different colored squares")]
    MisplacedBishop,
    #[error("King must be placed between rooks")]
    MisplacedKing,
    #[error("Back rank id is out of range (expecting 0..960)")]
    OutOfRange,
    #[error("Internal error: backrank not registered")]
    Unregistered,
}

use BackRankError::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackRankId(usize);

impl BackRankId {
    pub const STANDARD: Self = Self(518);

    pub fn shuffled() -> Self {
        let index = thread_rng().gen_range(0..960usize);
        Self(index)
    }

    pub fn try_from<I: Into<usize>>(index: I) -> Result<Self> {
        let index: usize = index.into();
        if index >= 960 {
            return Err(OutOfRange.into());
        }
        Ok(Self(index))
    }
}

impl Default for BackRankId {
    fn default() -> Self {
        Self::STANDARD
    }
}

impl From<BackRankId> for &'static BackRank {
    fn from(id: BackRankId) -> Self {
        BackRank::lookup(id)
    }
}

impl Display for BackRankId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
pub trait BackRanks: AsRef<BackRank> {
    #[inline]
    fn br_pieces(&self) -> [Piece; 8] {
        let backrank: &BackRank = self.as_ref();
        backrank.pieces
    }
    #[inline]
    fn br_king_file(&self) -> File{
        let backrank: &BackRank = self.as_ref();
        backrank.king
    }
    #[inline]
    fn br_queen_file(&self) -> File {
        let backrank: &BackRank = self.as_ref();
        backrank.queen
    }
    #[inline]
    fn br_rook_files(&self) -> [File; 2] {
        let backrank: &BackRank = self.as_ref();
        backrank.rooks
    }
    #[inline]
    fn br_bishop_files(&self) -> [File; 2] {
        let backrank: &BackRank = self.as_ref();
        backrank.bishops
    }
    #[inline]
    fn br_knight_files(&self) -> [File; 2] {
        let backrank: &BackRank = self.as_ref();
        backrank.knights
    }
}


/// Represents a configuration of pieces on a chessboard's back rank.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq)]
pub struct BackRank {
    id: BackRankId,
    pieces: [Piece; 8],
    king: File,
    queen: File,
    rooks: [File; 2],
    bishops: [File; 2],
    knights: [File; 2],
}

impl PartialEq for BackRank {
    fn eq(&self, other: &Self) -> bool {
        self.pieces == other.pieces
    }
}

impl Hash for BackRank {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pieces.hash(state);
    }
}

impl AsRef<Self> for BackRank {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl BackRanks for BackRank {}

impl BackRank {
    /// Creates a new back rank configuration with provided backrank id.
    /// 
    /// Backrank id should be in the range 0..=959 and uniquely determines
    /// the backrank position according to the algorithm defined here:
    /// https://en.wikipedia.org/wiki/Fischer_random_chess_numbering_scheme
    fn new(id: usize) -> Self {
        debug_assert!(id < 960);
        let mut n = id % 960;
        let mut extract = |size: usize| {
            let result = n % size;
            n /= size;
            result
        };
        let mut pieces = [Pawn; 8];

        // place bishops on different colored squares
        let mut bishops = [
            File::from_index(extract(4)*2+1), // light square
            File::from_index(extract(4)*2),   // dark square
        ];
        bishops.sort();
        pieces[bishops[0] as usize] = Bishop;
        pieces[bishops[1] as usize] = Bishop;

        let mut place = |piece: Piece, mut skip_count: usize| {
            #[allow(clippy::needless_range_loop)]
            for i in 0..8 {
                if pieces[i] == Pawn {
                    if skip_count == 0 {
                        pieces[i] = piece;
                        return File::from_index(i);
                    }
                    skip_count -= 1;
                }
            }
            unreachable!()
        };

        // place queen on one of 6 remaining empty slots
        let queen = place(Queen, extract(6));

        // place knights on two of 5 remaining empty slots
        const SKIP_TABLE: [(usize, usize); 10] = [
            (0, 0), (0, 1), (0, 2), (0, 3),
            (1, 1), (1, 2), (1, 3),
            (2, 2), (2, 3),
            (3, 3),
        ];
        let (skip1, skip2) = SKIP_TABLE[extract(10)];
        let knights = [
            place(Knight, skip1), 
            place(Knight, skip2), 
        ];

        // place rooks on first and third of 3 empty slots
        let rooks = [
            place(Rook, 0),
            place(Rook, 1),
        ];

        // place king on last remaining empty slot
        let king = place(King, 0);

        Self { id: BackRankId(id % 960), pieces, king, queen, rooks, bishops, knights }
    }

    /// Creates a standard back rank configuration suitable for the
    /// standard chess game.
    pub fn standard() -> Self {
        *Self::lookup(BackRankId::STANDARD)
    }

    /// Creates a shuffled back rank configuration suitable for the chess
    /// variant Chess960 (aka Fischer random chess).
    pub fn shuffled() -> Self {
        *Self::lookup(BackRankId::shuffled())
    }

    pub fn lookup(id: BackRankId) -> &'static BackRank {
        &BACKRANKS[id.0]
    }

    pub fn id(&self) -> BackRankId {
        self.id
    }
    pub fn king(&self) -> File {
        self.king
    }
    pub fn queen(&self) -> File {
        self.queen
    }
    pub fn rooks(&self) -> [File; 2] {
        self.rooks
    }
    pub fn bishops(&self) -> [File; 2] {
        self.bishops
    }
    pub fn knights(&self) -> [File; 2] {
        self.knights
    }
}

impl Index<File> for BackRank {
    type Output = Piece;
    fn index(&self, file: File) -> &Self::Output {
        &self.pieces[file.to_index()]
    }
}

impl IndexMut<File> for BackRank {
    fn index_mut(&mut self, file: File) -> &mut Self::Output {
        &mut self.pieces[file.to_index()]
    }
}

static BACKRANKS: Lazy<Vec<BackRank>> = Lazy::new(|| {
    let mut result = Vec::new();
    for id in 0..960 {
        result.push(BackRank::new(id));
    }
    result
});

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use super::*;
    use super::File::*;

    #[test]
    fn test_backrank_id_518_is_standard() {
        let index: usize = 518;
        assert!(BackRankId::try_from(index).is_ok());
        let backrank: &BackRank = BackRankId::try_from(index).unwrap().into();
        assert_eq!(backrank.king(), FileE);
        assert_eq!(backrank.queen(), FileD);
        assert_eq!(backrank.rooks(), [FileA, FileH]);
        assert_eq!(backrank.knights(), [FileB, FileG]);
        assert_eq!(backrank.bishops(), [FileC, FileF]);
    }
    #[test]
    fn test_backrank_id_959_is_valid() {
        let index: usize = 959;
        assert!(BackRankId::try_from(index).is_ok());
    }
    #[test]
    fn test_backrank_id_960_is_invalid() {
        let index: usize = 960;
        assert!(BackRankId::try_from(index).is_err());
    }
    #[test]
    fn test_backranks_are_unique() {
        let mut visited = HashSet::new();
        for index in 0..960usize {
            let id = BackRankId::try_from(index)
                .expect("index should be less than 960");
            let backrank: &BackRank = id.into();
            assert!(!visited.contains(backrank));
            visited.insert(backrank);
        }
    }
}
