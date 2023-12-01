// Copyright 2023 Tobin Edwards
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

use std::{collections::HashMap, fmt::Display};
use rand::{thread_rng, Rng};
use thiserror::Error;
use anyhow::Result;
use strum::IntoEnumIterator;
use std::ops::{Index, IndexMut};
use once_cell::sync::Lazy;
use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};

use super::square::File;
use super::material::Piece;
use File::{FileA, FileB, FileC, FileD, FileE, FileF, FileG, FileH};
use Piece::{King, Queen, Rook, Bishop, Knight};

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
    pub const STANDARD: Self = Self(0);

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
    /// Creates a standard back rank configuration suitable for the
    /// standard chess game.
    pub fn standard() -> Self {
        Self {
            pieces: [Rook, Knight, Bishop, Queen, King, Bishop, Knight, Rook],
            king: FileE,
            queen: FileD,
            rooks: [FileA, FileH],
            knights: [FileB, FileG],
            bishops: [FileC, FileF],
        }
    }

    /// Creates a shuffled back rank configuration suitable for the chess
    /// variant Chess960 (aka Fischer random chess).
    pub fn shuffled() -> Self {
        *Self::lookup(BackRankId::shuffled())

        // // Arrange non-bishops. The king must be placed between the rooks,
        // // so we'll start with three rooks and then replace the middle
        // // one with the king.
        // let mut pieces = vec![Queen, Knight, Knight, Rook, Rook, Rook];
        // pieces.shuffle(&mut rng);
        // let king_index = pieces.iter().enumerate()
        //     .filter(|&(_, &val)| val == Rook)
        //     .map(|(i, _)| i)
        //     .nth(1).unwrap();
        // let _ = std::mem::replace(&mut pieces[king_index], King);

        // // Insert bishops on differently colored squares
        // let b1 = rng.gen_range(0..4usize) * 2;
        // let b2 = rng.gen_range(0..4usize) * 2 + 1;
        // pieces.insert(max(b1, b2), Bishop);
        // pieces.insert(min(b1, b2), Bishop);
        // Self::build(pieces).expect("BackRank::shuffled(): invalid back rank")
    }

    pub fn lookup(id: BackRankId) -> &'static BackRank {
        &BACKRANKS[id]
    }

    /// Builds a BackRank instance from a sequence of pieces. Validates that
    /// exactly 8 pieces are provided (1 king, 1 queen, 2 rooks, 2 bishops and 2
    /// knights), that the bishops are on different colored squares and that the
    /// king is between the two rooks.
    ///
    /// # Arguments
    ///
    /// * `pieces` - A sequence of `Piece` values representing the desired configuration.
    ///
    /// # Returns
    ///
    /// A `Result` containing the constructed `BackRank` instance if the configuration is valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided configuration is invalid. 
    pub fn build<I>(pieces: I) -> Result<Self> 
    where
        I: IntoIterator<Item=Piece>,
    {
        let result = Self::inner_build(pieces);
        if let Ok(backrank) = result {
            if !BACKRANKS.contains(&backrank) {
                return Err(Unregistered.into());
            }
        }
        result
    }

    /// Builds a BackRank instance from a sequence of pieces. Validates that
    /// exactly 8 pieces are provided (1 king, 1 queen, 2 rooks, 2 bishops and 2
    /// knights), that the bishops are on different colored squares and that the
    /// king is between the two rooks.
    ///
    /// # Arguments
    ///
    /// * `pieces` - A sequence of `Piece` values representing the desired configuration.
    ///
    /// # Returns
    ///
    /// A `Result` containing the constructed `BackRank` instance if the configuration is valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided configuration is invalid. 
    fn inner_build<I>(pieces: I) -> Result<Self> 
    where
        I: IntoIterator<Item=Piece>,
    {
        let pieces = into_array(pieces)?;
        let mut map: HashMap<Piece, Vec<File>> = HashMap::new();
        for file in File::iter() {
            map.entry(pieces[file.to_index()])
                .or_insert_with(Vec::new)
                .push(file);
        }
        let king = into_item(map.remove(&King).unwrap_or_default())?;
        let queen = into_item(map.remove(&Queen).unwrap_or_default())?;
        let rooks = into_array(map.remove(&Rook).unwrap_or_default())?;
        let bishops = into_array(map.remove(&Bishop).unwrap_or_default())?;
        let knights = into_array(map.remove(&Knight).unwrap_or_default())?;

        if ((bishops[0] as usize) & 0x1) == ((bishops[1] as usize) & 0x1) {
            return Err(MisplacedBishop.into());
        }
        if (king as usize) < (rooks[0] as usize) || (king as usize) > (rooks[1] as usize) {
            return Err(MisplacedKing.into())
        }
        Ok(Self {pieces, king, queen, rooks, bishops, knights})
    }

    pub fn id(&self) -> BackRankId {
        // Safety: all possible backranks have been registered in `BACKRANKS`
        debug_assert!(BACKRANKS.contains(self));
        BACKRANKS[self]
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

/// Converts an iterator of values into an array of size `N`.
///
/// # Arguments
///
/// * `values` - An iterator of values to be converted into an array.
///
/// # Returns
///
/// A `Result` containing the array if `values` contains exactly `N` items
///
/// # Errors
///
/// Returns an error if `values` does not contain exactly `N` items.
fn into_array<T, I, const N: usize>(values: I) -> Result<[T; N]> 
where
    I: IntoIterator<Item=T>,
{
    let vec: Vec<T> = values.into_iter().collect();
    vec.try_into().map_err(|_| ArgError.into())
}

/// Converts an iterator of values into a single item.
///
/// # Arguments
///
/// * `values` - An iterator of values to be converted into a single item.
///
/// # Returns
///
/// A `Result` containing the single item if `values` contains exactly one item.
///
/// # Errors
///
/// Returns an error if `values` does not contain exactly one item.
fn into_item<T: Copy, I>(values: I) -> Result<T> 
where
    I: IntoIterator<Item=T>,
{
    let array: [T; 1] = into_array(values)?;
    Ok(array[0])
}

struct BackRankMatrix {
    ids: HashMap<[Piece; 8], BackRankId>,
    backranks: Vec<BackRank>,
}

impl BackRankMatrix {
    fn contains(&self, backrank: &BackRank) -> bool {
        self.ids.contains_key(&backrank.pieces)
    }
}

impl Index<BackRankId> for BackRankMatrix {
    type Output = BackRank;
    fn index(&self, id: BackRankId) -> &Self::Output {
        // Safety: all possible BackRank instances correspond to an
        // entry in this matrix
        // self.ids.get(&backrank.pieces).unwrap()
        &self.backranks[id.0]
    }
}
impl Index<&BackRank> for BackRankMatrix {
    type Output = BackRankId;
    fn index(&self, backrank: &BackRank) -> &Self::Output {
        // Safety: all possible BackRankId instances correspond to an 
        // entry in this matrix
        &self.ids[&backrank.pieces]
    }
}

static BACKRANKS: Lazy<BackRankMatrix> = Lazy::new(|| {
    const MATRIX: [[Piece; 5]; 30] = [
        [Queen, Bishop, Bishop, Knight, Knight, ],  // xBBxx 16 excluded
        [Queen, Bishop, Knight, Bishop, Knight, ],  // xBxBx 32 excluded
        [Queen, Bishop, Knight, Knight, Bishop, ],  // xBxxB 28 excluded
        [Queen, Knight, Bishop, Knight, Bishop, ],  // xxBxB 32 excluded
        [Queen, Knight, Bishop, Bishop, Knight, ],  // xxBBx 16 excluded
        [Queen, Knight, Knight, Bishop, Bishop, ],  // xxxBB 16 excluded

        [Bishop, Queen, Bishop, Knight, Knight, ],  // BxBxx 32 excluded
        [Bishop, Queen, Knight, Bishop, Knight, ],  // BxxBx 28 excluded
        [Bishop, Queen, Knight, Knight, Bishop, ],  // BxxxB 24 excluded
        [Knight, Queen, Bishop, Knight, Bishop, ],  // xxBxB 32 excluded
        [Knight, Queen, Bishop, Bishop, Knight, ],  // xxBBx 16 excluded
        [Knight, Queen, Knight, Bishop, Bishop, ],  // xxxBB 16 excluded

        [Bishop, Bishop, Queen, Knight, Knight, ],  // BBxxx 16 excluded
        [Bishop, Knight, Queen, Bishop, Knight, ],  // BxxBx 28 excluded
        [Bishop, Knight, Queen, Knight, Bishop, ],  // BxxxB 24 excluded
        [Knight, Bishop, Queen, Knight, Bishop, ],  // xBxxB 28 excluded
        [Knight, Bishop, Queen, Bishop, Knight, ],  // xBxBx 32 excluded
        [Knight, Knight, Queen, Bishop, Bishop, ],  // xxxBB 16 excluded

        [Bishop, Bishop, Knight, Queen, Knight, ],  // BBxxx 16 excluded
        [Bishop, Knight, Bishop, Queen, Knight, ],  // BxBxx 32 excluded
        [Bishop, Knight, Knight, Queen, Bishop, ],  // BxxxB 24 excluded
        [Knight, Bishop, Knight, Queen, Bishop, ],  // xBxxB 28 excluded
        [Knight, Bishop, Bishop, Queen, Knight, ],  // xBBxx 16 excluded
        [Knight, Knight, Bishop, Queen, Bishop, ],  // xxBxB 32 excluded

        [Bishop, Bishop, Knight, Knight, Queen, ],  // BBxxx 16 excluded
        [Bishop, Knight, Bishop, Knight, Queen, ],  // BxBxx 32 excluded
        [Bishop, Knight, Knight, Bishop, Queen, ],  // BxxBx 28 excluded
        [Knight, Bishop, Knight, Bishop, Queen, ],  // xBxBx 32 excluded
        [Knight, Bishop, Bishop, Knight, Queen, ],  // xBBxx 16 excluded
        [Knight, Knight, Bishop, Bishop, Queen, ],  // xxBBx 16 excluded
    ];

    const SPLITS: [[usize; 4]; 56] = [
        [5, 0, 0, 0, ],
        [4, 1, 0, 0, ],
        [4, 0, 1, 0, ],
        [4, 0, 0, 1, ],
        // 4
        [3, 2, 0, 0, ],
        [3, 1, 1, 0, ],
        [3, 1, 0, 1, ],
        [3, 0, 2, 0, ],
        // 8
        [3, 0, 1, 1, ],
        [3, 0, 0, 2, ],
        [2, 3, 0, 0, ],
        [2, 2, 1, 0, ],
        // 12
        [2, 2, 0, 1, ],
        [2, 1, 2, 0, ],
        [2, 1, 1, 1, ],
        [2, 1, 0, 2, ],
        // 16
        [2, 0, 3, 0, ],
        [2, 0, 2, 1, ],
        [2, 0, 1, 2, ],
        [2, 0, 0, 3, ],
        // 20
        [1, 4, 0, 0, ],
        [1, 3, 1, 0, ],
        [1, 3, 0, 1, ],
        [1, 2, 2, 0, ],
        // 24
        [1, 2, 1, 1, ],
        [1, 2, 0, 2, ],
        [1, 1, 3, 0, ],
        [1, 1, 2, 1, ],
        // 28
        [1, 1, 1, 2, ],
        [1, 1, 0, 3, ],
        [1, 0, 4, 0, ],
        [1, 0, 3, 1, ],
        // 32
        [1, 0, 2, 2, ],
        [1, 0, 1, 3, ],
        [1, 0, 0, 4, ],
        [0, 5, 0, 0, ],
        // 36
        [0, 4, 1, 0, ],
        [0, 4, 0, 1, ],
        [0, 3, 2, 0, ],
        [0, 3, 1, 1, ],
        // 40
        [0, 3, 0, 2, ],
        [0, 2, 3, 0, ],
        [0, 2, 2, 1, ],
        [0, 2, 1, 2, ],
        // 44
        [0, 2, 0, 3, ],
        [0, 1, 4, 0, ],
        [0, 1, 3, 1, ],
        [0, 1, 2, 2, ],
        // 48
        [0, 1, 1, 3, ],
        [0, 1, 0, 4, ],
        [0, 0, 5, 0, ],
        [0, 0, 4, 1, ],
        // 52
        [0, 0, 3, 2, ],
        [0, 0, 2, 3, ],
        [0, 0, 1, 4, ],
        [0, 0, 0, 5, ],
        // 56
    ];

    let mut backranks = Vec::new();
    let mut ids = HashMap::new();

    let standard = BackRank::standard();
    backranks.push(standard);   // standard board is at index 0
    ids.insert(standard.pieces, BackRankId(0));

    for pieces in MATRIX {
        for split in SPLITS {
            let mut start = 0;
            let mut buckets = vec![];
            for count in split.iter() {
                let end = start + count;
                // eprintln!("{:?} - {:?} - {:?}..{:?}", pieces, split, start, end);
                buckets.push(pieces[start..end].to_vec());
                start += count;
        }
            let mut backrank_pieces = Vec::new();
            backrank_pieces.append(&mut buckets[0]);
            backrank_pieces.push(Rook);
            backrank_pieces.append(&mut buckets[1]);
            backrank_pieces.push(King);
            backrank_pieces.append(&mut buckets[2]);
            backrank_pieces.push(Rook);
            backrank_pieces.append(&mut buckets[3]);
            debug_assert!(backrank_pieces.len() == 8);

            if let Ok(backrank) = BackRank::inner_build(backrank_pieces) {
                // already placed standard board at index 0
                if backrank != standard {
                    backranks.push(backrank);
                    ids.insert(backrank.pieces, BackRankId(ids.len()));
                }
            }
        }
    }
    debug_assert!(backranks.len() == 960);
    BackRankMatrix {
        ids,
        backranks,
    }
});

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_backrank_id_0_is_standard() {
        let index: usize = 0;
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
