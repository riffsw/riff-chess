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

use std::ops::Index;

use super::backrank::BackRank;
use super::material::{Color, Material};
use super::moves::MoveState;
use super::position::{MoveId, Pos, Position};
use super::square::Square;
use super::Turn;

#[allow(clippy::len_without_is_empty)]
pub trait Review {
    fn len(&self) -> usize;
    fn offset(&self) -> &MoveId;
    fn get(&self, offset: &MoveId) -> Option<&Position>;

    #[inline]
    fn at_start(&self) -> bool {
        *self.offset() == MoveId::START
    }
    #[inline]
    fn at_end(&self) -> bool {
        self.offset().value() == self.len() - 1
    }
    #[inline]
    fn first(&self) -> &Position {
        let offset = MoveId::START;
        self.get(&offset).expect("Review::first - out of bounds")
    }
    #[inline]
    fn last(&self) -> &Position {
        let offset: MoveId = MoveId::START + (self.len() - 1);
        self.get(&offset).expect("Review::last - out of bounds")
    }
    #[inline]
    fn current(&self) -> &Position {
        self.get(self.offset())
            .expect("Review::current - out of bounds")
    }
}

pub trait ReviewMut: Review {
    fn set_offset(&mut self, offset: MoveId);

    #[inline]
    fn forward(&mut self) {
        debug_assert!(self.offset().value() < self.len() - 1);
        self.set_offset(self.offset().next());
    }
    #[inline]
    fn back(&mut self) {
        debug_assert!(self.offset().value() > 0);
        self.set_offset(self.offset().prev());
    }
    #[inline]
    fn skip_to_start(&mut self) {
        let offset = MoveId::START;
        self.set_offset(offset);
    }
    #[inline]
    fn skip_to_end(&mut self) {
        let offset: MoveId = MoveId::START + (self.len() - 1);
        self.set_offset(offset);
    }
}

#[derive(Debug, Clone)]
pub struct ReviewState {
    offset: MoveId,
    history: Vec<MoveState>,
}

impl ReviewState {
    pub fn new(backrank: &'static BackRank) -> Self {
        let initial_state = MoveState::new(Position::new(backrank));
        Self {
            offset: MoveId::START,
            history: vec![initial_state],
        }
    }

    pub fn push(&mut self, state: MoveState) {
        if self.at_end() {
            self.offset = self.offset.next();
        }
        self.history.push(state);
    }
    pub fn truncate(&mut self) {
        self.history.truncate(self.offset.value() + 1);
    }
}

impl Turn for ReviewState {
    fn turn(&self) -> Color {
        self.current().turn()
    }
}

impl Review for ReviewState {
    #[inline]
    fn len(&self) -> usize {
        self.history.len()
    }
    #[inline]
    fn offset(&self) -> &MoveId {
        &self.offset
    }
    #[inline]
    fn get(&self, offset: &MoveId) -> Option<&Position> {
        let index: usize = offset.value();
        self.history.get(index).map(|state| state.as_ref())
    }
}

impl ReviewMut for ReviewState {
    #[inline]
    fn set_offset(&mut self, offset: MoveId) {
        self.offset = offset;
    }
}

impl Index<MoveId> for ReviewState {
    type Output = MoveState;
    fn index(&self, index: MoveId) -> &Self::Output {
        self.history.index(index.value())
    }
}

impl Index<&MoveId> for ReviewState {
    type Output = MoveState;
    fn index(&self, index: &MoveId) -> &Self::Output {
        self.history.index(index.value())
    }
}

impl Index<Square> for ReviewState {
    type Output = Option<Material>;
    fn index(&self, index: Square) -> &Self::Output {
        self.current().index(index)
    }
}

impl AsRef<Position> for ReviewState {
    fn as_ref(&self) -> &Position {
        self.current()
    }
}

impl Pos for ReviewState {}
