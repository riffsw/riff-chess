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

use strum_macros::EnumIter;
use strum::IntoEnumIterator;
use std::fmt;
use std::ops::{Add, Sub, Not, BitOr, BitAnd, BitOrAssign, BitAndAssign, Deref};
use std::ops::{Index, IndexMut};
use serde::{Deserialize, Serialize};

use super::material::Color;

use Color::*;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Square {
    A8, B8, C8, D8, E8, F8, G8, H8,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A1, B1, C1, D1, E1, F1, G1, H1,
}

use Square::{
    A8, B8, C8, D8, E8, F8, G8, H8,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A1, B1, C1, D1, E1, F1, G1, H1,
};

impl Square {
    #[inline]
    pub const fn new(file: File, rank: Rank) -> Self {
        Self::from_index(rank.to_index() * 8 + file.to_index())
    }

    #[inline]
    pub const fn from_index(index: usize) -> Self {
        const VALUES: [Square; 64] = [
            A8, B8, C8, D8, E8, F8, G8, H8,
            A7, B7, C7, D7, E7, F7, G7, H7,
            A6, B6, C6, D6, E6, F6, G6, H6,
            A5, B5, C5, D5, E5, F5, G5, H5,
            A4, B4, C4, D4, E4, F4, G4, H4,
            A3, B3, C3, D3, E3, F3, G3, H3,
            A2, B2, C2, D2, E2, F2, G2, H2,
            A1, B1, C1, D1, E1, F1, G1, H1,
        ];
        debug_assert!(index < 64);
        VALUES[index]
    }
    #[inline]
    pub fn from_string(name: &str) -> Self {
        Self::try_from_string(name).expect("Square::from_string: invalid format")
    }    
    #[inline]
    pub const fn from_chars(f: char, r: char) -> Self {
        Self::new(File::from_char(f), Rank::from_char(r))
    }
    #[inline]
    pub fn try_from_string(name: &str) -> Option<Self> {
        let mut chars = name.chars();
        let f = chars.next()?;
        let r = chars.next()?;
        Self::try_from_chars(f, r)
    }
    #[inline]
    pub fn try_from_chars(f: char, r: char) -> Option<Self> {
        let file = File::try_from_char(f)?;
        let rank = Rank::try_from_char(r)?;
        Some(Self::new(file, rank))
    }
    
    #[inline]
    pub const fn to_index(&self) -> usize { 
        *self as usize
    }
    #[inline]
    pub const fn to_mask(&self) -> Mask { 
        Mask::new(0x1 << (63 - self.to_index()))
    }
    #[inline]
    pub const fn file_index(&self) -> usize {
        self.to_index() % 8
    }
    #[inline]
    pub const fn rank_index(&self) -> usize {
        self.to_index() / 8
    }
    #[inline]
    pub const fn file(&self) -> File {
        File::from_index(self.file_index())
    }
    #[inline]
    pub const fn rank(&self) -> Rank {
        Rank::from_index(self.rank_index())
    }
 }

 impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}{})", self.file(), self.rank())
    }
}

impl From<Square> for usize {
    fn from(value: Square) -> Self {
        value.to_index()
    }
}

 #[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
pub enum File {
    FileA, FileB, FileC, FileD, FileE, FileF, FileG, FileH,
}

use File::{
    FileA, FileB, FileC, FileD, FileE, FileF, FileG, FileH,
};

impl File {
    #[inline]
    pub const fn from_index(index: usize) -> Self {
        const VALUES: [File; 8] = [
            FileA, FileB, FileC, FileD, FileE, FileF, FileG, FileH,
        ];
        debug_assert!(index < 8);
        VALUES[index]
    }
    #[inline]
    pub fn from_string(name: &str) -> Self {
        Self::from_char(name.chars().next().expect("input string too short"))
    }    
    #[inline]
    pub const fn from_char(c: char) -> Self {
        Self::from_index((c as usize) - ('a' as usize))
    }
    #[inline]
    pub fn try_from_string(name: &str) -> Option<Self> {
        Self::try_from_char(name.chars().next()?)
    }
    #[inline]
    pub const fn try_from_char(c: char) -> Option<Self> {
        match c {
            'a' | 'A' => Some(FileA),
            'b' | 'B' => Some(FileB),
            'c' | 'C' => Some(FileC),
            'd' | 'D' => Some(FileD),
            'e' | 'E' => Some(FileE),
            'f' | 'F' => Some(FileF),
            'g' | 'G' => Some(FileG),
            'h' | 'H' => Some(FileH),
            _ => None,
        }
    }

    #[inline]
    pub const fn to_index(&self) -> usize { 
        *self as usize 
    }
    #[inline]
    pub const fn to_mask(&self) -> Mask {
        Mask::new(u64::from_be_bytes([0x1 << (7 - self.to_index()); 8]))
    }
    #[inline]
    pub fn range(start: File, end: File) -> impl Iterator<Item=File> {
        let start_index = start.to_index();
        let end_index = end.to_index();
        (start_index..end_index).map(File::from_index)
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const VALUES: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        write!(f, "({})", VALUES[self.to_index()])
    }
}

impl Add<isize> for File {
    type Output = Option<Self>;
    fn add(self, rhs: isize) -> Self::Output {
        match self.to_index().checked_add_signed(rhs) {
            Some(i) if i < 8 => Some(Self::from_index(i)),
            _ => None,
        }
    }
}
impl Sub for File {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.to_index().wrapping_sub(rhs.to_index()) as isize
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
pub enum Rank {
    Rank8, Rank7, Rank6, Rank5, Rank4, Rank3, Rank2, Rank1,
}

use Rank::{
    Rank8, Rank7, Rank6, Rank5, Rank4, Rank3, Rank2, Rank1,
};

impl Rank {
    #[inline]
    pub fn is_back_rank(&self, color: Color) -> bool {
        Self::back_rank(color) == *self
    }

    #[inline]
    pub const fn back_rank(color: Color) -> Self {
        match color {
            White => Rank1,
            Black => Rank8,
        }
    }
    #[inline]
    pub const fn from_index(index: usize) -> Self {
        const VALUES: [Rank; 8] = [
            Rank8, Rank7, Rank6, Rank5, Rank4, Rank3, Rank2, Rank1,
        ];
        debug_assert!(index < 8);
        VALUES[index]
    }
    #[inline]
    pub fn from_string(name: &str) -> Self {
        Self::from_char(name.chars().next().expect("input string too short"))
    }    
    #[inline]
    pub const fn from_char(c: char) -> Self {
        Self::from_index(8 - ((c as usize) - ('0' as usize)))
    }
    #[inline]
    pub fn try_from_string(name: &str) -> Option<Self> {
        Self::try_from_char(name.chars().next()?)
    }
    #[inline]
    pub fn try_from_char(c: char) -> Option<Self> {
        match c {
            '1' => Some(Rank1),
            '2' => Some(Rank2),
            '3' => Some(Rank3),
            '4' => Some(Rank4),
            '5' => Some(Rank5),
            '6' => Some(Rank6),
            '7' => Some(Rank7),
            '8' => Some(Rank8),
            _ => None,
        }
    }
    #[inline]
    pub const fn to_index(&self) -> usize { 
        *self as usize 
    }
    #[inline]
    pub const fn to_mask(&self) -> Mask {
        Mask::new(0xff << ((7 - self.to_index()) * 8))
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", 8 - self.to_index())
    }
}

impl Add<isize> for Rank {
    type Output = Option<Self>;
    fn add(self, rhs: isize) -> Self::Output {
        match self.to_index().checked_add_signed(rhs) {
            Some(i) if i < 8 => Some(Self::from_index(i)),
            _ => None,
        }
    }
}

impl Sub for Rank {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.to_index().wrapping_sub(rhs.to_index()) as isize
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Offset {
    pub x: isize,
    pub y: isize,
}

impl Offset {
    pub const fn new(x: isize, y: isize) -> Self {
        Self {x, y}
    }

    pub fn to_unit(self) -> Option<Self> {
        let (x, y) = match (self.x, self.y) {
            (0, 0) => return None,
            (x, y) if x == 0 || y == 0 || x.abs() == y.abs() => (x.signum(), y.signum()),
            _ => return None,
        };
        Some(Self{x, y})
    }


}

impl Add<Offset> for Square {
    type Output = Option<Square>;
    fn add(self, rhs: Offset) -> Self::Output {
        let file = (self.file() + rhs.x)?;
        let rank = (self.rank() + rhs.y)?;
        Some(Square::new(file, rank))
    }
}
impl Add<&Offset> for Square {
    type Output = Option<Square>;
    fn add(self, rhs: &Offset) -> Self::Output {
        let file = (self.file() + rhs.x)?;
        let rank = (self.rank() + rhs.y)?;
        Some(Square::new(file, rank))
    }
}

impl Sub for Square {
    type Output = Offset;
    fn sub(self, rhs: Self) -> Self::Output {
        Offset::new(self.file() - rhs.file(), self.rank() - rhs.rank())
    }
}

impl Index<Square> for [Mask; 64] {
    type Output = Mask;
    fn index(&self, square: Square) -> &Self::Output {
        &self[square.to_index()]
    }
}

impl IndexMut<Square> for [Mask; 64] {
    fn index_mut(&mut self, square: Square) -> &mut Self::Output {
        &mut self[square.to_index()]
    }
}

impl Index<Square> for [Option<Mask>; 64] {
    type Output = Option<Mask>;
    fn index(&self, square: Square) -> &Self::Output {
        &self[square.to_index()]
    }
}

impl IndexMut<Square> for [Option<Mask>; 64] {
    fn index_mut(&mut self, square: Square) -> &mut Self::Output {
        &mut self[square.to_index()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum Direction {
    UpLeft,
    Up,
    UpRight,
    Left,
    Right,
    DownLeft,
    Down,
    DownRight,
}

use Direction::{
    UpLeft,
    Up,
    UpRight,
    Left,
    Right,
    DownLeft,
    Down,
    DownRight,
};

impl Direction {
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        matches!(*self, Up | Left | Right | Down)
    }
    #[inline]
    pub fn is_diagonal(&self) -> bool {
        matches!(*self, UpLeft | UpRight | DownLeft | DownRight)
    }
    pub fn horizontals() -> impl Iterator<Item=Self> {
        [Up, Left, Right, Down].into_iter()
    }
    pub fn diagonals() -> impl Iterator<Item=Self> {
        [UpLeft, UpRight, DownLeft, DownRight].into_iter()
    }
}

impl From<Direction> for Offset {
    fn from(value: Direction) -> Self {
        match value {
            UpLeft => Self::new(-1, -1),
            Up => Self::new(0, -1),
            UpRight => Self::new(1, -1),
            Left => Self::new(-1, 0),
            Right => Self::new(1, 0),
            DownLeft => Self::new(-1, 1),
            Down => Self::new(0, 1),
            DownRight => Self::new(1, 1),
        }
    }
}

impl Add<Direction> for Square {
    type Output = Option<Square>;
    fn add(self, rhs: Direction) -> Self::Output {
        let offset: Offset = rhs.into();
        self + offset
    }
}


#[derive(Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mask(u64);

impl Mask {
    #[inline]
    pub const fn new(val: u64) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn all() -> Self {
        Self(!0)
    }

    pub fn from_squares<I>(squares: I) -> Self 
    where
        I: IntoIterator<Item=Square>,
    {
        squares.into_iter()
            .map(|square| square.to_mask())
            .reduce(|m1, m2| m1 | m2)
            .unwrap_or_default()
    }

    #[inline]
    pub(crate) const fn inner(&self) -> u64 {
        self.0
    }
    
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }
    
    #[inline]
    pub const fn get(&self, square: Square) -> bool {
        let mask = 0x1 << (63 - square.to_index());
        (self.0 & mask) != 0
    }
    
    #[inline]
    pub fn set(&mut self, square: Square) {
        let mask = 0x1 << (63 - square.to_index());
        self.0 |= mask;
    }
    
    #[inline]
    pub fn reset(&mut self, square: Square) {
        let mask = 0x1 << (63 - square.to_index());
        self.0 &= !mask;
    }
    
    #[inline]
    pub fn set_if(&mut self, square: Square, cond: bool) {
        let mask = (cond as u64) << (63 - square.to_index());
        self.0 |= mask;
    }
    
    #[inline]
    pub fn reset_if(&mut self, square: Square, cond: bool) {
        let mask = (cond as u64) << (63 - square.to_index());
        self.0 &= !mask;
    }
    
    #[inline]
    pub const fn contains(&self, square: Square) -> bool {
        (self.0 & square.to_mask().0) != 0
    }

    pub fn iter(&self) -> MaskIter {
        MaskIter(self.0)
    }

}

impl fmt::Debug for Mask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in Rank::iter() {
            for file in File::iter() {
                let square = Square::new(file, rank);
                write!(f, "{}", if self.get(square) { "#" } else { "." })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
impl Deref for Mask {
    type Target = u64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Sub<Square> for Mask {
    type Output = Self;
    fn sub(self, rhs: Square) -> Self::Output {
        Self(self.0 & !rhs.to_mask().inner())
    }
}

impl Sub for Mask {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl Not for Mask {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
impl BitOr for Mask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Mask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitOr<Square> for Mask {
    type Output = Self;

    fn bitor(self, rhs: Square) -> Self {
        Self(self.0 | rhs.to_mask().0)
    }
}

impl BitOrAssign<Square> for Mask {
    fn bitor_assign(&mut self, rhs: Square) {
        self.0 |= rhs.to_mask().0;
    }
}

impl BitAnd for Mask {
    type Output = Self;
    
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Mask {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitAnd<Square> for Mask {
    type Output = Self;
    
    fn bitand(self, rhs: Square) -> Self {
        Self(self.0 & rhs.to_mask().0)
    }
}

impl BitAndAssign<Square> for Mask {
    fn bitand_assign(&mut self, rhs: Square) {
        self.0 &= rhs.to_mask().0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaskIter(u64);

impl MaskIter {
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub const  fn len(&self) -> usize {
        self.0.count_ones() as usize
    }
    pub const fn after(self, square: Square) -> Self {
        let mask = square.to_mask().inner() - 1;
        Self(self.0 & mask)
    }
    pub const fn before(self, square: Square) -> Self {
        let coord_mask = square.to_mask().inner();
        let mask = !(coord_mask | (coord_mask -1));
        Self(self.0 & mask)
    }
}

impl Iterator for MaskIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 != 0 {
            let square = Square::from_index(self.0.leading_zeros() as usize);
            self.0 &= !square.to_mask().inner();
            return Some(square);
        }
        None
    }
}

impl DoubleEndedIterator for MaskIter {

    fn next_back(&mut self) -> Option<Self::Item> {
        if self.0 != 0 {
            let square = Square::from_index(63 - self.0.trailing_zeros() as usize);
            self.0 &= !square.to_mask().inner();
            return Some(square);
        }
        None
    }
}
