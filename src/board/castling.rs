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

use serde::{Deserialize, Serialize};

use super::backrank::{BackRank, BackRanks};
use super::material::{Color, Pair};
use super::position::between;
use super::square::{File, Mask, Rank, Square};

use File::*;

pub trait Castling: AsRef<BackRank> + AsRef<CastlingRights> {
    fn oo(&self) -> bool {
        let rights: &CastlingRights = self.as_ref();
        rights.oo()
    }
    fn ooo(&self) -> bool {
        let rights: &CastlingRights = self.as_ref();
        rights.ooo()
    }
    #[inline]
    fn king_src(&self) -> Square {
        let backrank: &BackRank = self.as_ref();
        let rights: &CastlingRights = self.as_ref();
        Square::new(backrank.br_king_file(), rights.rank())
    }
    #[inline]
    fn oo_rook_src(&self) -> Square {
        let backrank: &BackRank = self.as_ref();
        let rights: &CastlingRights = self.as_ref();
        Square::new(backrank.br_rook_files()[1], rights.rank())
    }
    #[inline]
    fn oo_king_dest(&self) -> Square {
        let rights: &CastlingRights = self.as_ref();
        Square::new(FileG, rights.rank())
    }
    #[inline]
    fn oo_rook_dest(&self) -> Square {
        let rights: &CastlingRights = self.as_ref();
        Square::new(FileF, rights.rank())
    }
    #[inline]
    fn ooo_rook_src(&self) -> Square {
        let backrank: &BackRank = self.as_ref();
        let rights: &CastlingRights = self.as_ref();
        Square::new(backrank.br_rook_files()[0], rights.rank())
    }
    #[inline]
    fn ooo_king_dest(&self) -> Square {
        let rights: &CastlingRights = self.as_ref();
        Square::new(FileC, rights.rank())
    }
    #[inline]
    fn ooo_rook_dest(&self) -> Square {
        let rights: &CastlingRights = self.as_ref();
        Square::new(FileD, rights.rank())
    }
    /// The king's and the rook's `(from, to)` when castling short.
    #[inline]
    fn oo_squares(&self) -> [(Square, Square); 2] {
        [
            (self.king_src(), self.oo_king_dest()),
            (self.oo_rook_src(), self.oo_rook_dest()),
        ]
    }
    #[inline]
    fn ooo_squares(&self) -> [(Square, Square); 2] {
        [
            (self.king_src(), self.ooo_king_dest()),
            (self.ooo_rook_src(), self.ooo_rook_dest()),
        ]
    }
    /// The squares that must be empty to castle short: the king's path and the rook's
    /// path, not counting the king and rook themselves. In Chess960 either piece may
    /// already stand on its destination and the two paths may cross.
    fn oo_blocking_lane(&self) -> Mask {
        let king_src = self.king_src();
        let rook_src = self.oo_rook_src();
        (path(king_src, self.oo_king_dest()) | path(rook_src, self.oo_rook_dest()))
            & !(king_src.to_mask() | rook_src)
    }
    /// The squares the king crosses or lands on castling short; none may be attacked.
    fn oo_attacking_lane(&self) -> Mask {
        path(self.king_src(), self.oo_king_dest())
    }
    fn ooo_blocking_lane(&self) -> Mask {
        let king_src = self.king_src();
        let rook_src = self.ooo_rook_src();
        (path(king_src, self.ooo_king_dest()) | path(rook_src, self.ooo_rook_dest()))
            & !(king_src.to_mask() | rook_src)
    }
    fn ooo_attacking_lane(&self) -> Mask {
        path(self.king_src(), self.ooo_king_dest())
    }
}

/// The squares a piece crosses moving from `from` to `to` along a line, `to` included.
fn path(from: Square, to: Square) -> Mask {
    between(from, to) | to
}

pub trait CastlingMut: Castling + AsMut<CastlingRights> {
    fn update(&mut self, square: Square) {
        let king = self.king_src();
        let oo_rook = self.oo_rook_src();
        let ooo_rook = self.ooo_rook_src();
        let rights: &mut CastlingRights = self.as_mut();
        if rights.oo() && (square == king || square == oo_rook) {
            rights.clear_oo();
        }
        if rights.ooo() && (square == king || square == ooo_rook) {
            rights.clear_ooo();
        }
    }
    fn clear(&mut self) {
        let rights: &mut CastlingRights = self.as_mut();
        rights.clear();
    }
    fn clear_oo(&mut self) {
        let rights: &mut CastlingRights = self.as_mut();
        rights.clear_oo();
    }
    fn clear_ooo(&mut self) {
        let rights: &mut CastlingRights = self.as_mut();
        rights.clear_ooo();
    }
}

pub struct CastlingRightsRef<'a> {
    rights: &'a CastlingRights,
    backrank: &'static BackRank,
}

impl<'a> CastlingRightsRef<'a> {
    #[inline]
    pub fn new(rights: &'a CastlingRights, backrank: &'static BackRank) -> Self {
        Self { rights, backrank }
    }
}

impl From<&CastlingRightsRef<'_>> for &'static BackRank {
    fn from(value: &CastlingRightsRef<'_>) -> Self {
        value.backrank
    }
}

impl AsRef<BackRank> for CastlingRightsRef<'_> {
    fn as_ref(&self) -> &BackRank {
        self.backrank
    }
}
impl AsRef<CastlingRights> for CastlingRightsRef<'_> {
    fn as_ref(&self) -> &CastlingRights {
        self.rights
    }
}
impl Castling for CastlingRightsRef<'_> {}

pub struct CastlingRightsMut<'a> {
    rights: &'a mut CastlingRights,
    backrank: &'static BackRank,
}

impl<'a> CastlingRightsMut<'a> {
    #[inline]
    pub fn new(rights: &'a mut CastlingRights, backrank: &'static BackRank) -> Self {
        Self { rights, backrank }
    }
}

impl AsRef<BackRank> for CastlingRightsMut<'_> {
    fn as_ref(&self) -> &BackRank {
        self.backrank
    }
}
impl AsRef<CastlingRights> for CastlingRightsMut<'_> {
    fn as_ref(&self) -> &CastlingRights {
        self.rights
    }
}
impl AsMut<CastlingRights> for CastlingRightsMut<'_> {
    fn as_mut(&mut self) -> &mut CastlingRights {
        self.rights
    }
}

impl Castling for CastlingRightsMut<'_> {}

impl CastlingMut for CastlingRightsMut<'_> {}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastlingRights {
    color: Color,
    oo: bool,
    ooo: bool,
}

impl CastlingRights {
    pub fn new(color: Color, oo: bool, ooo: bool) -> Self {
        Self { color, oo, ooo }
    }
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }
    #[inline]
    pub fn oo(&self) -> bool {
        self.oo
    }
    #[inline]
    pub fn ooo(&self) -> bool {
        self.ooo
    }
    #[inline]
    pub fn rank(&self) -> Rank {
        Rank::back_rank(self.color)
    }
    pub fn clear(&mut self) {
        self.oo = false;
        self.ooo = false;
    }
    pub fn clear_oo(&mut self) {
        self.oo = false;
    }
    pub fn clear_ooo(&mut self) {
        self.ooo = false;
    }
}

impl Default for Pair<CastlingRights> {
    fn default() -> Self {
        Pair::new(
            CastlingRights::new(Color::White, true, true),
            CastlingRights::new(Color::Black, true, true),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BackRankId, LegalMove, LegalMoves, Material, MoveState, Position};
    use strum::IntoEnumIterator;
    use Square::*;

    /// A back rank with the king on `king` and rooks on `rooks`, found by searching the
    /// 960; a white position with rank 1 cleared except for those three pieces, and
    /// `occupied` filled with bishops.
    fn position(king: File, rooks: [File; 2], occupied: &[Square]) -> Position {
        let id = (0..960usize)
            .map(|i| BackRankId::try_from(i).unwrap())
            .find(|id| {
                let br: &'static BackRank = (*id).into();
                br.king() == king && br.rooks() == rooks
            })
            .expect("such a back rank exists");
        let mut pos = Position::new(id.into());
        for file in File::iter() {
            if file != king && !rooks.contains(&file) {
                pos = pos.set_contents(Square::new(file, Rank::Rank1), None);
            }
        }
        for square in occupied {
            pos = pos.set_contents(*square, Some(Material::WB));
        }
        pos
    }

    fn offers(pos: Position, from: Square, mv: LegalMove) -> bool {
        MoveState::new(pos).legal_moves(from).values().any(|m| *m == mv)
    }

    #[test]
    fn test_960_long_castle_needs_the_destinations_clear() {
        // king b1, rook a1: nothing lies between them, but c1 and d1 must be empty
        assert!(offers(position(FileB, [FileA, FileH], &[]), B1, LegalMove::LongCastle));
        assert!(!offers(position(FileB, [FileA, FileH], &[C1]), B1, LegalMove::LongCastle));
        assert!(!offers(position(FileB, [FileA, FileH], &[D1]), B1, LegalMove::LongCastle));
    }

    #[test]
    fn test_960_short_castle_with_the_king_already_home() {
        // king g1, rook h1: the king stays put and the rook needs f1
        assert!(offers(position(FileG, [FileA, FileH], &[]), G1, LegalMove::ShortCastle));
        assert!(!offers(position(FileG, [FileA, FileH], &[F1]), G1, LegalMove::ShortCastle));
    }

    #[test]
    fn test_960_castling_pieces_do_not_block_each_other() {
        // king f1, rook g1: they swap, crossing each other's square
        assert!(offers(position(FileF, [FileA, FileG], &[]), F1, LegalMove::ShortCastle));
        // king c1, rook a1: the king stays, the rook crosses b1 and lands on d1
        assert!(offers(position(FileC, [FileA, FileH], &[]), C1, LegalMove::LongCastle));
        assert!(!offers(position(FileC, [FileA, FileH], &[B1]), C1, LegalMove::LongCastle));
    }

    #[test]
    fn test_standard_castling_lanes_are_unchanged() {
        let pos = position(FileE, [FileA, FileH], &[]);
        assert!(offers(pos.clone(), E1, LegalMove::ShortCastle));
        assert!(offers(pos, E1, LegalMove::LongCastle));
        assert!(!offers(position(FileE, [FileA, FileH], &[B1]), E1, LegalMove::LongCastle));
    }
}
