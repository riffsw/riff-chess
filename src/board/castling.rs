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
    fn oo_blocking_lane(&self) -> Mask {
        let rook_src = self.oo_rook_src();
        let king_src = self.king_src();
        between(king_src, rook_src)
    }
    fn oo_attacking_lane(&self) -> Mask {
        let king_dest = self.oo_king_dest();
        let king_src = self.king_src();
        between(king_src, king_dest) | king_dest
    }
    fn ooo_blocking_lane(&self) -> Mask {
        let rook_src = self.ooo_rook_src();
        let king_src = self.king_src();
        between(rook_src, king_src)
    }
    fn ooo_attacking_lane(&self) -> Mask {
        let king_dest = self.ooo_king_dest();
        let king_src = self.king_src();
        between(king_dest, king_src) | king_dest
    }
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
