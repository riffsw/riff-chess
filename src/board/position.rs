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

#[derive(Debug, Clone)]
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
        for color in Color::iter() {
            for square in (masks.pieces[color] & masks.kings).iter() {
                array[square.to_index()] = Some(Material::new(color, King));
            }
            for square in (masks.pieces[color] & masks.queens).iter() {
                array[square.to_index()] = Some(Material::new(color, Queen));
            }
            for square in (masks.pieces[color] & masks.rooks).iter() {
                array[square.to_index()] = Some(Material::new(color, Rook));
            }
            for square in (masks.pieces[color] & masks.bishops).iter() {
                array[square.to_index()] = Some(Material::new(color, Bishop));
            }
            for square in (masks.pieces[color] & masks.knights).iter() {
                array[square.to_index()] = Some(Material::new(color, Knight));
            }
            for square in (masks.pieces[color] & masks.pawns).iter() {
                array[square.to_index()] = Some(Material::new(color, Pawn));
            }
        }
        Self(array)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Masks {
    pieces: Pair<Mask>,
    kings: Mask,
    queens: Mask,
    rooks: Mask,
    bishops: Mask,
    knights: Mask,
    pawns: Mask,
}

impl From<&Squares> for Masks {
    fn from(value: &Squares) -> Self {
        let mut masks = Masks::empty();
        for square in Square::iter() {
            if let Some(material) = value[square] {
                masks.pieces[material.color()] |= square;
                match material.piece() {
                    King => masks.kings |= square,
                    Queen => masks.queens |= square,
                    Rook => masks.rooks |= square,
                    Bishop => masks.bishops |= square,
                    Knight => masks.knights |= square,
                    Pawn => masks.pawns |= square,
                }
            }
        }
        masks
    }
}

impl Masks {
    fn empty() -> Self {
        Self {
            pieces: Pair::new(Mask::empty(), Mask::empty()),
            kings: Mask::empty(),
            queens: Mask::empty(),
            rooks: Mask::empty(),
            bishops: Mask::empty(),
            knights: Mask::empty(),
            pawns: Mask::empty(),
        }
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

    pub fn our_mating_material(&self) -> MatingMaterial {
        self.mating_material(self.turn())
    }

    pub fn their_mating_material(&self) -> MatingMaterial {
        self.mating_material(!self.turn())
    }

    fn mating_material(&self, side: Color) -> MatingMaterial {
        let pieces = self.masks.pieces[side] & !self.masks.kings;
        let pawns = pieces & self.masks.pawns;
        if !pawns.is_empty() {
            return MatingMaterial::Sufficient;
        }
        let rooks = pieces & self.masks.rooks;
        if !rooks.is_empty() {
            return MatingMaterial::Sufficient;
        }
        let queens = pieces & self.masks.queens;
        if !queens.is_empty() {
            return MatingMaterial::Sufficient;
        }
        if pieces.len() > 2 {
            return MatingMaterial::Sufficient;
        }
        if pieces.len() == 2 {
            if pieces == self.masks.knights {
                return MatingMaterial::TwoKnights;
            }
            return MatingMaterial::Sufficient;
        }
        if !pieces.is_empty() {
            if pieces == self.masks.knights {
                return MatingMaterial::OneKnight;
            }
            return MatingMaterial::OneBishop;
        }
        MatingMaterial::LoneKing
    }

    pub fn apply_move(&mut self, mv: LegalMove) -> MoveId {
        self.moves_since_progress += 1;
        match mv {
            LegalMove::Standard(from, to) => {
                let material = self.remove(from).unwrap();
                let captured = self.place(to, material);
                self.en_passant = None;
                self.our_castling_mut().update(from);
                self.their_castling_mut().update(to);
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
                self.their_castling_mut().update(to);
                self.en_passant = None;
                self.moves_since_progress = 0;
            }
            LegalMove::ShortCastle => {
                let king = self.remove(self.our_king_src()).unwrap();
                let rook = self.remove(self.our_oo_rook_src()).unwrap();
                self.place(self.our_oo_king_dest(), king);
                self.place(self.our_oo_rook_dest(), rook);
                self.our_castling_mut().clear();
                self.en_passant = None;
            }
            LegalMove::LongCastle => {
                let king = self.remove(self.our_king_src()).unwrap();
                let rook = self.remove(self.our_ooo_rook_src()).unwrap();
                self.place(self.our_ooo_king_dest(), king);
                self.place(self.our_ooo_rook_dest(), rook);
                self.our_castling_mut().clear();
                self.en_passant = None;
            }
        };
        let move_id = self.next_move_id;
        self.next_move_id = move_id.next();
        move_id
    }

    pub fn apply_pre_move(&mut self, mv: PreMove) {
        // Note: it's not "our" turn, so we use "their" to refer to
        // the side performing the pre-move, and vise versa.
        match mv {
            PreMove::Standard(from, to) => {
                let material = self.remove(from).unwrap();
                self.place(to, material);
                self.their_castling_mut().update(from);
                self.our_castling_mut().update(to);
            }
            PreMove::Promoting(from, to, promotion) => {
                let mut material = self.remove(from).unwrap();
                material.set_piece(promotion.into());
                self.place(to, material);
                self.our_castling_mut().update(to);
            }
            PreMove::ShortCastle => {
                let king = self.remove(self.their_king_src()).unwrap();
                let rook = self.remove(self.their_oo_rook_src()).unwrap();
                self.place(self.their_oo_king_dest(), king);
                self.place(self.their_oo_rook_dest(), rook);
                self.their_castling_mut().clear();
            }
            PreMove::LongCastle => {
                let king = self.remove(self.their_king_src()).unwrap();
                let rook = self.remove(self.their_ooo_rook_src()).unwrap();
                self.place(self.their_ooo_king_dest(), king);
                self.place(self.their_ooo_rook_dest(), rook);
                self.their_castling_mut().clear();
            }
        }
    }

    fn place(&mut self, square: Square, material: Material) -> Option<Material> {
        let replaced = self.remove(square);
        self.squares[square] = Some(material);
        let mask = square.to_mask();
        self.masks.pieces[material.color()] |= mask;
        match material.piece() {
            King => self.masks.kings |= mask,
            Queen => self.masks.queens |= mask,
            Rook => self.masks.rooks |= mask,
            Bishop => self.masks.bishops |= mask,
            Knight => self.masks.knights |= mask,
            Pawn => self.masks.pawns |= mask,
        }
        replaced
    }
    fn remove(&mut self, square: Square) -> Option<Material> {
        if let Some(material) = self.squares[square] {
            self.squares[square] = None;
            let mask = !square.to_mask();
            self.masks.pieces[material.color()] &= mask;
            match material.piece() {
                King => self.masks.kings &= mask,
                Queen => self.masks.queens &= mask,
                Rook => self.masks.rooks &= mask,
                Bishop => self.masks.bishops &= mask,
                Knight => self.masks.knights &= mask,
                Pawn => self.masks.pawns &= mask,
            }
            return Some(material);
        }
        None
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
    #[inline]
    pub fn our_king_src(&self) -> Square {
        self.our_castling().king_src()
    }
    #[inline]
    pub fn our_oo_rook_src(&self) -> Square {
        self.our_castling().oo_rook_src()
    }
    #[inline]
    pub fn our_ooo_rook_src(&self) -> Square {
        self.our_castling().ooo_rook_src()
    }
    #[inline]
    pub fn our_oo_king_dest(&self) -> Square {
        self.our_castling().oo_king_dest()
    }
    #[inline]
    pub fn our_ooo_king_dest(&self) -> Square {
        self.our_castling().ooo_king_dest()
    }
    #[inline]
    pub fn our_oo_rook_dest(&self) -> Square {
        self.our_castling().oo_rook_dest()
    }
    #[inline]
    pub fn our_ooo_rook_dest(&self) -> Square {
        self.our_castling().ooo_rook_dest()
    }
    #[inline]
    pub fn their_king_src(&self) -> Square {
        self.their_castling().king_src()
    }
    #[inline]
    pub fn their_oo_rook_src(&self) -> Square {
        self.their_castling().oo_rook_src()
    }
    #[inline]
    pub fn their_ooo_rook_src(&self) -> Square {
        self.their_castling().ooo_rook_src()
    }
    #[inline]
    pub fn their_oo_king_dest(&self) -> Square {
        self.their_castling().oo_king_dest()
    }
    #[inline]
    pub fn their_ooo_king_dest(&self) -> Square {
        self.their_castling().ooo_king_dest()
    }
    #[inline]
    pub fn their_oo_rook_dest(&self) -> Square {
        self.their_castling().oo_rook_dest()
    }
    #[inline]
    pub fn their_ooo_rook_dest(&self) -> Square {
        self.their_castling().ooo_rook_dest()
    }
    #[inline]
    pub fn our_castling(&self) -> CastlingRightsRef<'_> {
        let turn = self.turn();
        CastlingRightsRef::new(&self.castling[turn], self.backrank)
    }
    #[inline]
    pub fn their_castling(&self) -> CastlingRightsRef<'_> {
        let turn = self.turn();
        CastlingRightsRef::new(&self.castling[!turn], self.backrank)
    }

    #[inline]
    pub fn our_castling_mut(&mut self) -> CastlingRightsMut<'_> {
        let turn = self.turn();
        CastlingRightsMut::new(&mut self.castling[turn], self.backrank)
    }
    #[inline]
    pub fn their_castling_mut(&mut self) -> CastlingRightsMut<'_> {
        let turn = self.turn();
        CastlingRightsMut::new(&mut self.castling[!turn], self.backrank)
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
        pos.masks.kings
    }
    #[inline]
    fn queens(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.queens
    }
    #[inline]
    fn rooks(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.rooks
    }
    #[inline]
    fn bishops(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.bishops
    }
    #[inline]
    fn knights(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.knights
    }
    #[inline]
    fn pawns(&self) -> Mask {
        let pos: &Position = self.as_ref();
        pos.masks.pawns
    }
    #[inline]
    fn occupied_by(&self, color: Color) -> Mask {
        match color {
            White => self.white(),
            Black => self.black(),
        }
    }

    #[inline]
    fn our_king(&self) -> Square {
        let mask = self.ours() & self.kings();
        debug_assert!(mask.len() == 1);
        mask.iter().next().unwrap()
    }
    #[inline]
    fn our_queens(&self) -> Mask {
        self.ours() & self.queens()
    }
    #[inline]
    fn our_rooks(&self) -> Mask {
        self.ours() & self.rooks()
    }
    #[inline]
    fn our_bishops(&self) -> Mask {
        self.ours() & self.bishops()
    }
    #[inline]
    fn our_knights(&self) -> Mask {
        self.ours() & self.knights()
    }
    #[inline]
    fn our_pawns(&self) -> Mask {
        self.ours() & self.pawns()
    }
    #[inline]
    fn our_line_pieces(&self) -> Mask {
        self.theirs() & self.line_pieces()
    }
    #[inline]
    fn their_king(&self) -> Square {
        let mask = self.theirs() & self.kings();
        debug_assert!(mask.len() == 1);
        mask.iter().next().unwrap()
    }
    #[inline]
    fn their_queens(&self) -> Mask {
        self.theirs() & self.queens()
    }
    #[inline]
    fn their_rooks(&self) -> Mask {
        self.theirs() & self.rooks()
    }
    #[inline]
    fn their_bishops(&self) -> Mask {
        self.theirs() & self.bishops()
    }
    #[inline]
    fn their_knights(&self) -> Mask {
        self.theirs() & self.knights()
    }
    #[inline]
    fn their_pawns(&self) -> Mask {
        self.theirs() & self.pawns()
    }
    #[inline]
    fn their_line_pieces(&self) -> Mask {
        self.theirs() & self.line_pieces()
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
    use Square::*;

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
