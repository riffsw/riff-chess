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

use once_cell::sync::Lazy;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::{Add, AddAssign, Index, IndexMut, Sub, SubAssign};
use strum::IntoEnumIterator;

use super::backrank::{BackRank, BackRankId, BackRanks};
use super::castling::{
    Castling, CastlingMut, CastlingRights, CastlingRightsMut, CastlingRightsRef,
};
use super::material::{Color, Material, Pair, Piece};
use super::moves::{LegalMove, PreMove};
use super::square::{Direction, File, Mask, Rank, Square};
use super::Turn;

use Color::*;
use Piece::*;
use Rank::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MoveId(u16);

impl MoveId {
    pub const START: MoveId = MoveId(0);

    #[inline]
    pub fn new(move_count: u16, turn: Color) -> Self {
        match turn {
            White => Self(move_count * 2),
            Black => Self(move_count * 2 + 1),
        }
    }
    #[inline]
    pub fn turn(&self) -> Color {
        const TURNS: [Color; 2] = [White, Black];
        let index = self.value() % 2;
        TURNS[index]
    }
    #[inline]
    pub fn value(&self) -> usize {
        self.0 as usize
    }
    #[inline]
    pub fn move_count(&self) -> usize {
        self.value() / 2
    }
    #[inline]
    pub fn move_number(&self) -> usize {
        1 + self.move_count()
    }
    #[inline]
    pub fn at_start(&self) -> bool {
        self.0 == 0
    }
    #[inline]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
    #[inline]
    pub fn prev(self) -> Self {
        Self(self.0 - 1)
    }
}

impl Default for MoveId {
    #[inline]
    fn default() -> Self {
        MoveId::START
    }
}

impl Sub for MoveId {
    type Output = usize;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.value() - rhs.value()
    }
}

impl<T: Into<usize>> Add<T> for MoveId {
    type Output = MoveId;
    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into() as u16)
    }
}
impl<T: Into<usize>> AddAssign<T> for MoveId {
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into() as u16;
    }
}

impl<T: Into<usize>> Sub<T> for MoveId {
    type Output = MoveId;
    fn sub(self, rhs: T) -> Self::Output {
        Self(self.0 - rhs.into() as u16)
    }
}
impl<T: Into<usize>> SubAssign<T> for MoveId {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs.into() as u16;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatingMaterial {
    Sufficient,
    TwoKnights,
    OneKnight,
    OneBishop,
    LoneKing,
}

/// What stands on each square. A cache of [`Masks`], which is the board's canonical form:
/// the masks are what a position is hashed and serialized by, and `Squares` exists so that
/// "what is on e4?" is one load rather than eight bit tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Squares([Option<Material>; 64]);

impl Squares {
    fn empty() -> Self {
        Self([None; 64])
    }
}

impl Index<Square> for Squares {
    type Output = Option<Material>;
    fn index(&self, index: Square) -> &Self::Output {
        &self.0[index.to_index()]
    }
}

impl IndexMut<Square> for Squares {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self.0[index.to_index()]
    }
}

impl From<&Masks> for Squares {
    fn from(masks: &Masks) -> Self {
        let mut array = [None; 64];
        for square in Square::iter() {
            array[square.to_index()] = masks.get(square);
        }
        Self(array)
    }
}

/// The board as bitboards: one mask per color and one per kind of piece.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Masks {
    pieces: Pair<Mask>,
    kinds: [Mask; 6],
}

impl From<&Squares> for Masks {
    fn from(squares: &Squares) -> Self {
        let mut masks = Masks::empty();
        for square in Square::iter() {
            if let Some(material) = squares[square] {
                masks.set(square, material);
            }
        }
        masks
    }
}

impl Masks {
    fn empty() -> Self {
        Self {
            pieces: Pair::new(Mask::empty(), Mask::empty()),
            kinds: [Mask::empty(); 6],
        }
    }

    #[inline]
    fn set(&mut self, square: Square, material: Material) {
        self.pieces[material.color()] |= square;
        self[material.piece()] |= square;
    }

    #[inline]
    fn clear(&mut self, square: Square, material: Material) {
        let mask = !square.to_mask();
        self.pieces[material.color()] &= mask;
        self[material.piece()] &= mask;
    }

    fn get(&self, square: Square) -> Option<Material> {
        let color = Color::iter().find(|color| self.pieces[*color].contains(square))?;
        let piece = Piece::iter().find(|piece| self[*piece].contains(square))?;
        Some(Material::new(color, piece))
    }
}

impl Index<Piece> for Masks {
    type Output = Mask;
    #[inline]
    fn index(&self, piece: Piece) -> &Mask {
        &self.kinds[piece as usize]
    }
}

impl IndexMut<Piece> for Masks {
    #[inline]
    fn index_mut(&mut self, piece: Piece) -> &mut Mask {
        &mut self.kinds[piece as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionKey {
    turn: Color,
    en_passant: Option<Square>,
    castling: Pair<CastlingRights>,
    masks: Masks,
}

#[derive(Debug, Clone)]
pub struct Position {
    squares: Squares,
    masks: Masks,
    backrank: &'static BackRank,
    castling: Pair<CastlingRights>,
    en_passant: Option<Square>,
    next_move_id: MoveId,
    moves_since_progress: u8,
}

impl Default for Position {
    fn default() -> Self {
        Self::new(BackRankId::default().into())
    }
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(6)?;
        tuple.serialize_element(&self.masks)?;
        tuple.serialize_element(&self.backrank.id())?;
        tuple.serialize_element(&self.castling)?;
        tuple.serialize_element(&self.en_passant)?;
        tuple.serialize_element(&self.next_move_id)?;
        tuple.serialize_element(&self.moves_since_progress)?;
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PositionVisitor;
        impl<'de> serde::de::Visitor<'de> for PositionVisitor {
            type Value = (
                Masks,
                BackRankId,
                Pair<CastlingRights>,
                Option<Square>,
                MoveId,
                u8,
            );
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a Position struct condensed into a 6-element tuple")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let masks = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("Missing elements"))?;
                let backrank_id = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("Missing elements"))?;
                let castling = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("Missing elements"))?;
                let en_passant = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("Missing elements"))?;
                let next_move_id = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("Missing elements"))?;
                let moves_since_progress = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::custom("Missing elements"))?;
                Ok((
                    masks,
                    backrank_id,
                    castling,
                    en_passant,
                    next_move_id,
                    moves_since_progress,
                ))
            }
        }
        let (masks, backrank_id, castling, en_passant, next_move_id, moves_since_progress) =
            deserializer.deserialize_tuple(7, PositionVisitor)?;
        let squares = (&masks).into();
        let backrank = BackRank::lookup(backrank_id);
        Ok(Position {
            squares,
            masks,
            backrank,
            castling,
            en_passant,
            next_move_id,
            moves_since_progress,
        })
    }
}

impl Position {
    pub fn new(backrank: &'static BackRank) -> Self {
        let position = Self {
            squares: Squares::empty(),
            masks: Masks::empty(),
            backrank,
            castling: Pair::default(),
            en_passant: None,
            next_move_id: MoveId(0),
            moves_since_progress: 0,
        };
        position.init()
    }

    fn init(mut self) -> Self {
        self.init_file(self.backrank.king(), King);
        self.init_file(self.backrank.queen(), Queen);
        for file in self.backrank.rooks() {
            self.init_file(file, Rook);
        }
        for file in self.backrank.bishops() {
            self.init_file(file, Bishop);
        }
        for file in self.backrank.knights() {
            self.init_file(file, Knight);
        }
        self
    }

    fn init_file(&mut self, file: File, piece: Piece) {
        const PAWN_RANKS: Pair<Rank> = Pair::new(Rank2, Rank7);
        const BACK_RANKS: Pair<Rank> = Pair::new(Rank1, Rank8);
        for color in Color::iter() {
            let square = Square::new(file, PAWN_RANKS[color]);
            let material = Material::new(color, Pawn);
            let _ = self.place(square, material);
            let square = Square::new(file, BACK_RANKS[color]);
            let material = Material::new(color, piece);
            let _ = self.place(square, material);
        }
    }

    pub fn key(&self) -> PositionKey {
        PositionKey {
            turn: self.turn(),
            en_passant: self.en_passant,
            castling: self.castling,
            masks: self.masks,
        }
    }

    pub fn squares(&self) -> &Squares {
        &self.squares
    }

    pub fn masks(&self) -> &Masks {
        &self.masks
    }

    pub fn backrank(&self) -> &BackRank {
        self.backrank
    }

    pub fn move_number(&self) -> usize {
        self.next_move_id.move_number()
    }

    pub fn moves_since_progress(&self) -> usize {
        self.moves_since_progress as usize
    }

    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    pub fn mating_material(&self, side: Color) -> MatingMaterial {
        let pieces = self.masks.pieces[side] & !self.masks[King];
        let pawns = pieces & self.masks[Pawn];
        if !pawns.is_empty() {
            return MatingMaterial::Sufficient;
        }
        let rooks = pieces & self.masks[Rook];
        if !rooks.is_empty() {
            return MatingMaterial::Sufficient;
        }
        let queens = pieces & self.masks[Queen];
        if !queens.is_empty() {
            return MatingMaterial::Sufficient;
        }
        if pieces.len() > 2 {
            return MatingMaterial::Sufficient;
        }
        if pieces.len() == 2 {
            if pieces == self.masks[Knight] {
                return MatingMaterial::TwoKnights;
            }
            return MatingMaterial::Sufficient;
        }
        if !pieces.is_empty() {
            if pieces == self.masks[Knight] {
                return MatingMaterial::OneKnight;
            }
            return MatingMaterial::OneBishop;
        }
        MatingMaterial::LoneKing
    }

    pub fn apply_move(&mut self, mv: LegalMove) -> MoveId {
        let side = self.turn();
        self.moves_since_progress += 1;
        match mv {
            LegalMove::Standard(from, to) => {
                let material = self.remove(from).unwrap();
                let captured = self.place(to, material);
                self.en_passant = None;
                self.castling_mut(side).update(from);
                self.castling_mut(!side).update(to);
                if captured.is_some() || material.piece() == Pawn {
                    self.moves_since_progress = 0;
                }
            }
            LegalMove::EnPassant(from, to) => {
                let material = self.remove(from).unwrap();
                let target = Square::new(to.file(), from.rank());
                let _ = self.remove(target).unwrap();
                self.place(to, material);
                self.en_passant = None;
                self.moves_since_progress = 0;
            }
            LegalMove::DoubleAdvance(from, to) => {
                let target = between(from, to).iter().next().unwrap();
                let material = self.remove(from).unwrap();
                self.place(to, material);
                self.en_passant = Some(target);
                self.moves_since_progress = 0;
            }
            LegalMove::Promoting(from, to, promotion) => {
                let mut material = self.remove(from).unwrap();
                material.set_piece(promotion.into());
                self.place(to, material);
                self.castling_mut(!side).update(to);
                self.en_passant = None;
                self.moves_since_progress = 0;
            }
            LegalMove::ShortCastle => {
                self.castle(side, self.castling(side).oo_squares());
                self.en_passant = None;
            }
            LegalMove::LongCastle => {
                self.castle(side, self.castling(side).ooo_squares());
                self.en_passant = None;
            }
        };
        let move_id = self.next_move_id;
        self.next_move_id = move_id.next();
        move_id
    }

    pub fn apply_pre_move(&mut self, mv: PreMove) {
        // a pre-move is made by the side that is *not* on move
        let side = !self.turn();
        match mv {
            PreMove::Standard(from, to) => {
                let material = self.remove(from).unwrap();
                self.place(to, material);
                self.castling_mut(side).update(from);
                self.castling_mut(!side).update(to);
            }
            PreMove::Promoting(from, to, promotion) => {
                let mut material = self.remove(from).unwrap();
                material.set_piece(promotion.into());
                self.place(to, material);
                self.castling_mut(!side).update(to);
            }
            PreMove::ShortCastle => self.castle(side, self.castling(side).oo_squares()),
            PreMove::LongCastle => self.castle(side, self.castling(side).ooo_squares()),
        }
    }

    /// Move `side`'s king and rook to their castling squares and spend its rights.
    fn castle(&mut self, side: Color, [king, rook]: [(Square, Square); 2]) {
        let king_material = self.remove(king.0).unwrap();
        let rook_material = self.remove(rook.0).unwrap();
        self.place(king.1, king_material);
        self.place(rook.1, rook_material);
        self.castling_mut(side).clear();
    }

    fn place(&mut self, square: Square, material: Material) -> Option<Material> {
        let replaced = self.remove(square);
        self.squares[square] = Some(material);
        self.masks.set(square, material);
        replaced
    }
    fn remove(&mut self, square: Square) -> Option<Material> {
        let material = self.squares[square].take()?;
        self.masks.clear(square, material);
        Some(material)
    }
}

impl Turn for Position {
    #[inline]
    fn turn(&self) -> Color {
        self.next_move_id.turn()
    }
}
impl Index<Square> for Position {
    type Output = Option<Material>;
    #[inline]
    fn index(&self, index: Square) -> &Self::Output {
        &self.squares[index]
    }
}

impl AsRef<BackRank> for Position {
    fn as_ref(&self) -> &BackRank {
        self.backrank
    }
}

impl AsRef<Self> for Position {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl BackRanks for Position {}

impl Pos for Position {}

impl Position {
    /// `side`'s castling rights, read with the back rank they refer to.
    #[inline]
    pub fn castling(&self, side: Color) -> CastlingRightsRef<'_> {
        CastlingRightsRef::new(&self.castling[side], self.backrank)
    }
    #[inline]
    pub fn castling_mut(&mut self, side: Color) -> CastlingRightsMut<'_> {
        CastlingRightsMut::new(&mut self.castling[side], self.backrank)
    }
}

pub trait Pos: Turn + AsRef<Position> {
    #[inline]
    fn contents(&self, square: Square) -> &Option<Material> {
        let pos: &Position = self.as_ref();
        &pos.squares.0[square.to_index()]
    }
    #[inline]
    fn white(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.pieces[Color::White]
    }
    #[inline]
    fn black(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.pieces[Color::Black]
    }
    #[inline]
    fn kings(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks[King]
    }
    #[inline]
    fn queens(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks[Queen]
    }
    #[inline]
    fn rooks(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks[Rook]
    }
    #[inline]
    fn bishops(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks[Bishop]
    }
    #[inline]
    fn knights(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks[Knight]
    }
    #[inline]
    fn pawns(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks[Pawn]
    }
    #[inline]
    fn occupied_by(&self, color: Color) -> Mask {
        match color {
            White => self.white(),
            Black => self.black(),
        }
    }

    #[inline]
    fn king(&self, side: Color) -> Square {
        let mask = self.occupied_by(side) & self.kings();
        debug_assert!(mask.len() == 1);
        mask.iter().next().unwrap()
    }
    #[inline]
    fn is_vacant(&self, square: Square) -> bool {
        self.contents(square).is_none()
    }
    #[inline]
    fn is_occupied(&self, square: Square) -> bool {
        self.contents(square).is_some()
    }
    #[inline]
    fn vacant(&self) -> Mask {
        !self.occupied()
    }
    #[inline]
    fn occupied(&self) -> Mask {
        self.white() | self.black()
    }
    #[inline]
    fn ours(&self) -> Mask {
        self.occupied_by(self.turn())
    }
    #[inline]
    fn theirs(&self) -> Mask {
        self.occupied_by(!self.turn())
    }
    #[inline]
    fn horizontals(&self) -> Mask {
        self.rooks() | self.queens()
    }
    #[inline]
    fn diagonals(&self) -> Mask {
        self.bishops() | self.queens()
    }
    #[inline]
    fn line_pieces(&self) -> Mask {
        self.horizontals() | self.diagonals()
    }
}

#[inline]
pub(super) fn blocked(from: Square, to: Square) -> Mask {
    let index = from.to_index() * 64 + to.to_index();
    SQUARES_SHIELDED[index] | to.to_mask()
}

#[inline]
pub(super) fn shielded(from: Square, to: Square) -> Mask {
    let index = from.to_index() * 64 + to.to_index();
    SQUARES_SHIELDED[index]
}

#[inline]
pub(super) fn between(from: Square, to: Square) -> Mask {
    let index = from.to_index() * 64 + to.to_index();
    SQUARES_BETWEEN[index]
}

pub(super) static SQUARES_BETWEEN: Lazy<[Mask; 64 * 64]> = Lazy::new(|| {
    // Returns a mask of squares between `start` and `end` (exclusive of both)
    // if they are not equal and in a line. Otherwise returns an empty mask.
    fn squares_between(start: Square, end: Square) -> Mask {
        let mut mask = Mask::empty();
        if let Some(step) = (end - start).to_unit() {
            if let Some(mut start) = start + step {
                while start != end {
                    mask |= start.to_mask();
                    // Safety: calling `unwrap` here is safe because
                    // `step` is a unit from start to end and we'll
                    // always hit `end` before dropping off the edge
                    start = (start + step).unwrap();
                }
            }
        }
        mask
    }

    let mut array = [Mask::empty(); 64 * 64];
    let mut visited = HashSet::new();
    for start in Square::iter() {
        let start_index = start.to_index();
        for end in Square::iter() {
            if start == end {
                continue;
            }
            let end_index = end.to_index();
            let index1 = start_index * 64 + end_index;
            let index2: usize = end_index * 64 + start_index;
            if !visited.contains(&index1) {
                visited.insert(index1);
                visited.insert(index2);
                if ALL_LINES[start_index].contains(end) {
                    // Safety: `start` and `end` are in a line with each
                    // other and are not equal
                    let mask = squares_between(start, end);
                    array[index1] = mask;
                    array[index2] = mask;
                }
            }
        }
    }
    array
});

pub(super) static SQUARES_SHIELDED: Lazy<[Mask; 64 * 64]> = Lazy::new(|| {
    // Returns a mask of squares between `end` (exclusive) and the edge of
    // the board if we draw a line from `start` through `end`. Returns `None`
    // if `start` and `end` are equal or not in a line.
    fn squares_shielded(start: Square, end: Square) -> Mask {
        let mut mask = Mask::empty();
        if let Some(step) = (end - start).to_unit() {
            let mut next = end + step;
            while next.is_some() {
                let square = next.unwrap();
                mask |= square.to_mask();
                next = square + step;
            }
        }
        mask
    }

    let mut array = [Mask::empty(); 64 * 64];
    for start in Square::iter() {
        let start_index = start.to_index();
        for end in Square::iter() {
            if start == end {
                continue;
            }
            let end_index = end.to_index();
            let index = start_index * 64 + end_index;
            if ALL_LINES[start_index].contains(end) {
                // Safety: `start` and `end` are in a line with each
                // other and are not equal
                let mask = squares_shielded(start, end);
                array[index] = mask;
            }
        }
    }
    array
});

pub(super) static HORIZONTALS: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        let mask = square.file().to_mask() | square.rank().to_mask();
        array[square.to_index()] = mask;
    }
    array
});

pub(super) static DIAGONALS: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        let mut mask = square.to_mask();
        Direction::diagonals().for_each(|dir| {
            let mut next = square + dir;
            loop {
                let Some(sq) = next else { break };
                mask |= sq.to_mask();
                next = sq + dir;
            }
        });
        array[square.to_index()] = mask;
    }
    array
});

pub(super) static ALL_LINES: Lazy<[Mask; 64]> = Lazy::new(|| {
    let mut array = [Mask::default(); 64];
    for square in Square::iter() {
        array[square] = HORIZONTALS[square] | DIAGONALS[square];
    }
    array
});

#[cfg(test)]
impl Position {
    pub fn set_contents(mut self, square: Square, value: Option<Material>) -> Self {
        self.squares[square] = value;
        self.masks = (&self.squares).into();
        self
    }
    pub fn set_en_passant(mut self, value: Option<Square>) -> Self {
        self.en_passant = value;
        self
    }
    pub fn clear_white_oo(mut self) -> Self {
        self.castling[White].clear_oo();
        self
    }
    pub fn clear_white_ooo(mut self) -> Self {
        self.castling[White].clear_ooo();
        self
    }
    pub fn clear_black_oo(mut self) -> Self {
        self.castling[Black].clear_oo();
        self
    }
    pub fn clear_black_ooo(mut self) -> Self {
        self.castling[Black].clear_ooo();
        self
    }
    pub fn set_next_move_id(mut self, value: MoveId) -> Self {
        self.next_move_id = value;
        self
    }
    pub fn set_moves_since_progress(mut self, value: u8) -> Self {
        self.moves_since_progress = value;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LegalMoves, MoveState};
    use Square::*;

    /// Both views of the board agree after every move of a long random game. The walk is
    /// seeded, so a failure reproduces.
    #[test]
    fn test_squares_follow_masks() {
        let mut state = MoveState::default();
        let mut seed: u64 = 0x5EED;
        let mut plies = 0;
        for _ in 0..400 {
            let pos: &Position = state.as_ref();
            let moves: Vec<LegalMove> = pos
                .ours()
                .iter()
                .flat_map(|from| state.legal_moves(from).values().copied().collect::<Vec<_>>())
                .collect();
            if moves.is_empty() {
                break;
            }
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let mv = moves[(seed >> 33) as usize % moves.len()];
            state.apply_move(mv);
            plies += 1;

            let pos: &Position = state.as_ref();
            assert_eq!(Squares::from(pos.masks()), *pos.squares(), "after {mv:?}");
            assert_eq!(Masks::from(pos.squares()), *pos.masks(), "after {mv:?}");
        }
        assert!(plies > 100, "the walk ended after {plies} plies; pick another seed");
    }

    #[test]
    fn test_diagonals() {
        let mask = DIAGONALS[C5];
        assert!(mask.contains(C5));
        assert!(mask.contains(A3));
        assert!(mask.contains(A7));
        assert!(mask.contains(F8));
        assert!(mask.contains(G1));
        assert!(!mask.contains(C6));
        assert!(!mask.contains(C4));
        assert!(!mask.contains(B5));
        assert!(!mask.contains(D5));
    }
    #[test]
    fn test_horizontals() {
        let mask = HORIZONTALS[G2];
        assert!(mask.contains(G2));
        assert!(mask.contains(G1));
        assert!(mask.contains(G8));
        assert!(mask.contains(A2));
        assert!(mask.contains(H2));
        assert!(!mask.contains(H1));
        assert!(!mask.contains(F1));
        assert!(!mask.contains(F3));
        assert!(!mask.contains(H3));
    }
    #[test]
    fn test_all_lines() {
        let mask = ALL_LINES[D3];
        assert!(mask.contains(D3));
        assert!(mask.contains(D1));
        assert!(mask.contains(D8));
        assert!(mask.contains(A3));
        assert!(mask.contains(H3));
        assert!(mask.contains(B1));
        assert!(mask.contains(A6));
        assert!(mask.contains(F1));
        assert!(mask.contains(H7));
        assert!(!mask.contains(A1));
    }
    #[test]
    fn test_between_a3_and_e3() {
        let from = A3;
        let to = E3;
        let mask = between(from, to);
        assert_eq!(mask.len(), 3);
        assert!(!mask.contains(A3));
        assert!(mask.contains(B3));
        assert!(mask.contains(C3));
        assert!(mask.contains(D3));
        assert!(!mask.contains(E3));
    }
    #[test]
    fn test_between_c2_and_c8() {
        let from = C2;
        let to = C8;
        let mask = between(from, to);
        assert_eq!(mask.len(), 5);
        assert!(!mask.contains(C2));
        assert!(mask.contains(C3));
        assert!(mask.contains(C4));
        assert!(mask.contains(C5));
        assert!(mask.contains(C6));
        assert!(mask.contains(C7));
        assert!(!mask.contains(C8));
    }
    #[test]
    fn test_between_a1_and_d4() {
        let from = A1;
        let to = D4;
        let mask = between(from, to);
        assert_eq!(mask.len(), 2);
        assert!(!mask.contains(A1));
        assert!(mask.contains(B2));
        assert!(mask.contains(C3));
        assert!(!mask.contains(D4));
    }
    #[test]
    fn test_between_h3_and_f5() {
        let from = H3;
        let to = F5;
        let mask = between(from, to);
        assert_eq!(mask.len(), 1);
        assert!(!mask.contains(H3));
        assert!(mask.contains(G4));
        assert!(!mask.contains(F5));
    }
    #[test]
    fn test_between_g4_and_f5() {
        let from = G4;
        let to = F5;
        let mask = between(from, to);
        assert_eq!(mask.len(), 0);
        assert!(!mask.contains(G4));
        assert!(!mask.contains(F5));
    }
    #[test]
    fn test_between_a1_and_h5() {
        let from = A1;
        let to = H5;
        let mask = between(from, to);
        assert_eq!(mask.len(), 0);
        assert!(!mask.contains(A1));
        assert!(!mask.contains(H5));
    }
    #[test]
    fn test_shielded_from_a8_by_a7() {
        let from = A8;
        let to = A7;
        let mask = shielded(from, to);
        assert_eq!(mask.len(), 6);
        assert!(!mask.contains(A8));
        assert!(!mask.contains(A7));
        assert!(mask.contains(A6));
        assert!(mask.contains(A1));
    }
    #[test]
    fn test_shielded_from_a7_by_a8() {
        let from = A7;
        let to = A8;
        let mask = shielded(from, to);
        assert_eq!(mask.len(), 0);
    }
    #[test]
    fn test_blocked_from_a8_by_a7() {
        let from = A8;
        let to = A7;
        let mask = blocked(from, to);
        assert_eq!(mask.len(), 7);
        assert!(!mask.contains(A8));
        assert!(mask.contains(A7));
        assert!(mask.contains(A6));
        assert!(mask.contains(A1));
    }
    #[test]
    fn test_blocked_from_a7_by_a8() {
        let from = A7;
        let to = A8;
        let mask = blocked(from, to);
        assert_eq!(mask.len(), 1);
        assert!(mask.contains(A8));
        assert!(!mask.contains(A7));
    }
}
