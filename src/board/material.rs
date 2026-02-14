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
use std::hash::{Hash, Hasher};
use std::ops::{Index, IndexMut, Not};
use strum_macros::Display;
use strum_macros::EnumIter;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material {
    color: Color,
    piece: Piece,
}

impl Material {
    pub const WK: Self = Self {
        color: White,
        piece: King,
    };
    pub const WQ: Self = Self {
        color: White,
        piece: Queen,
    };
    pub const WR: Self = Self {
        color: White,
        piece: Rook,
    };
    pub const WB: Self = Self {
        color: White,
        piece: Bishop,
    };
    pub const WN: Self = Self {
        color: White,
        piece: Knight,
    };
    pub const WP: Self = Self {
        color: White,
        piece: Pawn,
    };

    pub const BK: Self = Self {
        color: Black,
        piece: King,
    };
    pub const BQ: Self = Self {
        color: Black,
        piece: Queen,
    };
    pub const BR: Self = Self {
        color: Black,
        piece: Rook,
    };
    pub const BB: Self = Self {
        color: Black,
        piece: Bishop,
    };
    pub const BN: Self = Self {
        color: Black,
        piece: Knight,
    };
    pub const BP: Self = Self {
        color: Black,
        piece: Pawn,
    };

    #[inline]
    pub const fn new(color: Color, piece: Piece) -> Self {
        Self { color, piece }
    }

    #[inline]
    pub const fn white(piece: Piece) -> Self {
        Self::new(White, piece)
    }

    #[inline]
    pub const fn black(piece: Piece) -> Self {
        Self::new(Black, piece)
    }

    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    #[inline]
    pub fn piece(&self) -> Piece {
        self.piece
    }

    #[inline]
    pub fn set_piece(&mut self, piece: Piece) {
        self.piece = piece
    }

    #[inline]
    pub fn to_index(&self) -> usize {
        self.color.to_index() * 2 + self.piece.to_index()
    }
}

use Color::{Black, White};

#[derive(Debug, Serialize, Deserialize, Display, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const fn to_index(&self) -> usize {
        *self as usize
    }
}

impl Not for Color {
    type Output = Self;

    #[inline]
    fn not(self) -> Self {
        match self {
            White => Black,
            Black => White,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pair<T>((T, T));

impl<T> Pair<T> {
    pub const fn new(white: T, black: T) -> Self {
        Self((white, black))
    }
}

impl<T> Pair<T> {
    pub fn white(&self) -> &T {
        &self.0 .0
    }
    pub fn white_mut(&mut self) -> &mut T {
        &mut self.0 .0
    }
    pub fn black(&self) -> &T {
        &self.0 .1
    }
    pub fn black_mut(&mut self) -> &mut T {
        &mut self.0 .1
    }
    pub fn to_tuple(&self) -> &(T, T) {
        &self.0
    }
}

impl<T: Hash> Hash for Pair<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.white().hash(state);
        self.black().hash(state);
    }
}

impl<T> Index<&Color> for Pair<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: &Color) -> &Self::Output {
        match index {
            White => self.white(),
            Black => self.black(),
        }
    }
}

impl<T> IndexMut<&Color> for Pair<T> {
    #[inline(always)]
    fn index_mut(&mut self, index: &Color) -> &mut Self::Output {
        match index {
            White => self.white_mut(),
            Black => self.black_mut(),
        }
    }
}

impl<T> Index<Color> for Pair<T> {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: Color) -> &Self::Output {
        match index {
            White => self.white(),
            Black => self.black(),
        }
    }
}

impl<T> IndexMut<Color> for Pair<T> {
    #[inline(always)]
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        match index {
            White => self.white_mut(),
            Black => self.black_mut(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Display, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}
use Piece::{Bishop, King, Knight, Pawn, Queen, Rook};

impl Piece {
    pub const fn from_index(index: usize) -> Self {
        debug_assert!(index < 6);
        const PIECE_MAP: [Piece; 6] = [Pawn, Knight, Bishop, Rook, Queen, King];
        PIECE_MAP[index]
    }

    pub fn to_index(&self) -> usize {
        *self as usize
    }
    pub fn is_king(&self) -> bool {
        matches!(*self, King)
    }
    pub fn is_queen(&self) -> bool {
        matches!(*self, Queen)
    }
    pub fn is_rook(&self) -> bool {
        matches!(*self, Rook)
    }
    pub fn is_bishop(&self) -> bool {
        matches!(*self, Bishop)
    }
    pub fn is_knight(&self) -> bool {
        matches!(*self, Knight)
    }
    pub fn is_pawn(&self) -> bool {
        matches!(*self, Pawn)
    }
}
