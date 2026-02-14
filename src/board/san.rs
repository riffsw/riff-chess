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

use anyhow::Result;
use strum::IntoEnumIterator;

use super::material::Piece;
use super::moves::{LegalMove, LegalMoves, MoveError, MoveState, Promotion};
use super::position::Pos;
use super::square::{File, Rank, Square};
use super::Turn;

use Piece::*;

/// Generate a SAN string for a legal move given the current board state.
pub fn to_san(state: &MoveState, lm: LegalMove) -> String {
    let mut s = match lm {
        LegalMove::ShortCastle => "O-O".to_string(),
        LegalMove::LongCastle => "O-O-O".to_string(),
        _ => format_move(state, lm),
    };
    s.push_str(check_suffix(state, lm));
    s
}

/// Parse a SAN string into a LegalMove given the current board state.
pub fn from_san(state: &MoveState, san: &str) -> Result<LegalMove> {
    let s = san.trim_end_matches(['+', '#']);

    // Castling (accept both O and 0)
    if s == "O-O" || s == "0-0" {
        return Ok(LegalMove::ShortCastle);
    }
    if s == "O-O-O" || s == "0-0-0" {
        return Ok(LegalMove::LongCastle);
    }

    let parsed = parse_san_components(s)?;

    // Find matching legal move by iterating all legal moves
    let color = state.turn();
    let mut matched: Option<LegalMove> = None;

    for from in Square::iter() {
        if let Some(mat) = state.contents(from) {
            if mat.color() != color || mat.piece() != parsed.piece {
                continue;
            }
            if let Some(file) = parsed.from_file {
                if from.file() != file {
                    continue;
                }
            }
            if let Some(rank) = parsed.from_rank {
                if from.rank() != rank {
                    continue;
                }
            }

            let moves = state.legal_moves(from);
            if moves.destinations().contains(parsed.to) {
                let lm = moves.get(parsed.to).unwrap();
                if let Some(expected_promo) = parsed.promotion {
                    // SAN specifies promotion — legal_moves may store this
                    // as Standard for pawns reaching the back rank
                    match lm {
                        LegalMove::Promoting(_, _, promo) if promo == expected_promo => {}
                        LegalMove::Promoting(_, _, _) => continue,
                        LegalMove::Standard(f, t) if mat.piece() == Pawn => {
                            matched = Some(LegalMove::Promoting(f, t, expected_promo));
                            continue;
                        }
                        _ => continue,
                    }
                } else if matches!(lm, LegalMove::Promoting(_, _, _)) {
                    continue;
                }
                matched = Some(lm);
            }
        }
    }

    matched.ok_or_else(|| MoveError::InvalidMove.into())
}

fn format_move(state: &MoveState, lm: LegalMove) -> String {
    let (from, to) = move_squares(lm);
    let piece = state.contents(from).unwrap().piece();
    let capture = is_capture(state, lm);

    let mut s = String::new();

    if piece == Pawn {
        if capture {
            s.push(from.file().to_char());
        }
    } else {
        s.push(piece_san_char(piece));
        disambiguate(state, piece, from, to, &mut s);
    }

    if capture {
        s.push('x');
    }

    s.push(to.file().to_char());
    s.push(to.rank().to_char());

    if let LegalMove::Promoting(_, _, promo) = lm {
        s.push('=');
        s.push(promotion_san_char(promo));
    }

    s
}

fn move_squares(lm: LegalMove) -> (Square, Square) {
    match lm {
        LegalMove::Standard(from, to)
        | LegalMove::DoubleAdvance(from, to)
        | LegalMove::EnPassant(from, to)
        | LegalMove::Promoting(from, to, _) => (from, to),
        LegalMove::ShortCastle | LegalMove::LongCastle => {
            unreachable!("castling handled separately")
        }
    }
}

fn is_capture(state: &MoveState, lm: LegalMove) -> bool {
    match lm {
        LegalMove::EnPassant(_, _) => true,
        LegalMove::ShortCastle | LegalMove::LongCastle => false,
        _ => {
            let (_, to) = move_squares(lm);
            state.contents(to).is_some()
        }
    }
}

fn disambiguate(state: &MoveState, piece: Piece, from: Square, to: Square, s: &mut String) {
    let color = state.turn();
    let mut same_file = false;
    let mut same_rank = false;
    let mut ambiguous = false;

    for sq in Square::iter() {
        if sq == from {
            continue;
        }
        if let Some(mat) = state.contents(sq) {
            if mat.color() == color && mat.piece() == piece {
                let moves = state.legal_moves(sq);
                if moves.destinations().contains(to) {
                    ambiguous = true;
                    if sq.file() == from.file() {
                        same_file = true;
                    }
                    if sq.rank() == from.rank() {
                        same_rank = true;
                    }
                }
            }
        }
    }

    if !ambiguous {
        return;
    }

    if !same_file {
        s.push(from.file().to_char());
    } else if !same_rank {
        s.push(from.rank().to_char());
    } else {
        s.push(from.file().to_char());
        s.push(from.rank().to_char());
    }
}

fn check_suffix(state: &MoveState, lm: LegalMove) -> &'static str {
    let pos: &super::position::Position = state.as_ref();
    let mut pos = pos.clone();
    pos.apply_move(lm);
    let next = MoveState::new(pos);
    if next.is_check() {
        if next.has_any_legal_move() {
            "+"
        } else {
            "#"
        }
    } else {
        ""
    }
}

fn piece_san_char(piece: Piece) -> char {
    match piece {
        King => 'K',
        Queen => 'Q',
        Rook => 'R',
        Bishop => 'B',
        Knight => 'N',
        Pawn => unreachable!("pawns have no SAN prefix"),
    }
}

fn promotion_san_char(promo: Promotion) -> char {
    match promo {
        Promotion::Queen => 'Q',
        Promotion::Rook => 'R',
        Promotion::Bishop => 'B',
        Promotion::Knight => 'N',
    }
}

fn piece_from_san_char(c: char) -> Option<Piece> {
    match c {
        'K' => Some(King),
        'Q' => Some(Queen),
        'R' => Some(Rook),
        'B' => Some(Bishop),
        'N' => Some(Knight),
        _ => None,
    }
}

fn promotion_from_san_char(c: char) -> Option<Promotion> {
    match c {
        'Q' => Some(Promotion::Queen),
        'R' => Some(Promotion::Rook),
        'B' => Some(Promotion::Bishop),
        'N' => Some(Promotion::Knight),
        _ => None,
    }
}

struct ParsedSan {
    piece: Piece,
    from_file: Option<File>,
    from_rank: Option<Rank>,
    to: Square,
    promotion: Option<Promotion>,
}

fn parse_san_components(s: &str) -> Result<ParsedSan> {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    if len < 2 {
        return Err(MoveError::InvalidMove.into());
    }

    // Check for promotion suffix: "=Q", "=R", "=B", "=N"
    let (chars, promotion) = if len >= 3 && chars[len - 2] == '=' {
        let promo = promotion_from_san_char(chars[len - 1])
            .ok_or(MoveError::InvalidMove)?;
        (&chars[..len - 2], Some(promo))
    } else {
        (&chars[..], None)
    };

    let len = chars.len();
    if len < 2 {
        return Err(MoveError::InvalidMove.into());
    }

    // Destination is always the last two chars: file + rank
    let dest_file = File::try_from_char(chars[len - 2]).ok_or(MoveError::InvalidMove)?;
    let dest_rank = Rank::try_from_char(chars[len - 1]).ok_or(MoveError::InvalidMove)?;
    let to = Square::new(dest_file, dest_rank);

    // Everything before destination is: [Piece][disambiguation][x]
    let prefix = &chars[..len - 2];

    // Strip capture marker
    let prefix = if prefix.last() == Some(&'x') {
        &prefix[..prefix.len() - 1]
    } else {
        prefix
    };

    // Determine piece and disambiguation
    let (piece, disambig) = if prefix.is_empty() {
        // Pawn move: "e4", "xd5" (after stripping x)
        (Pawn, &prefix[..0])
    } else if let Some(p) = piece_from_san_char(prefix[0]) {
        (p, &prefix[1..])
    } else {
        // Must be pawn with file disambiguation: "exd5"
        (Pawn, prefix)
    };

    // Parse disambiguation
    let (from_file, from_rank) = match disambig.len() {
        0 => (None, None),
        1 => {
            if let Some(f) = File::try_from_char(disambig[0]) {
                (Some(f), None)
            } else if let Some(r) = Rank::try_from_char(disambig[0]) {
                (None, Some(r))
            } else {
                return Err(MoveError::InvalidMove.into());
            }
        }
        2 => {
            let f = File::try_from_char(disambig[0]).ok_or(MoveError::InvalidMove)?;
            let r = Rank::try_from_char(disambig[1]).ok_or(MoveError::InvalidMove)?;
            (Some(f), Some(r))
        }
        _ => return Err(MoveError::InvalidMove.into()),
    };

    Ok(ParsedSan {
        piece,
        from_file,
        from_rank,
        to,
        promotion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use Square::*;

    // ---- to_san tests ----

    #[test]
    fn test_pawn_advance() {
        let state = MoveState::default();
        assert_eq!(to_san(&state, LegalMove::DoubleAdvance(E2, E4)), "e4");
        assert_eq!(to_san(&state, LegalMove::Standard(E2, E3)), "e3");
    }

    #[test]
    fn test_pawn_capture() {
        let position = Position::default()
            .set_contents(E4, Some(Material::WP))
            .set_contents(E2, None)
            .set_contents(D5, Some(Material::BP));
        let state = MoveState::new(position);
        assert_eq!(to_san(&state, LegalMove::Standard(E4, D5)), "exd5");
    }

    #[test]
    fn test_pawn_promotion() {
        // White pawn on B7 can promote to B8 (cleared) or capture A8 (black rook)
        let position = Position::default()
            .set_contents(B7, Some(Material::WP))
            .set_contents(B8, None);
        let state = MoveState::new(position);
        assert_eq!(
            to_san(&state, LegalMove::Promoting(B7, B8, Promotion::Queen)),
            "b8=Q"
        );
        assert_eq!(
            to_san(&state, LegalMove::Promoting(B7, A8, Promotion::Knight)),
            "bxa8=N"
        );
    }

    #[test]
    fn test_knight_move() {
        let state = MoveState::default();
        assert_eq!(to_san(&state, LegalMove::Standard(G1, F3)), "Nf3");
    }

    #[test]
    fn test_knight_disambiguation_by_file() {
        // Two knights that can both reach the same square
        let position = Position::default()
            .set_contents(A3, Some(Material::WN))
            .set_contents(C3, Some(Material::WN))
            .set_contents(B1, None);
        let state = MoveState::new(position);
        let san_a = to_san(&state, LegalMove::Standard(A3, B5));
        let san_c = to_san(&state, LegalMove::Standard(C3, B5));
        assert_eq!(san_a, "Nab5");
        assert_eq!(san_c, "Ncb5");
    }

    #[test]
    fn test_knight_disambiguation_by_rank() {
        // Two knights on the same file (G), both can reach E2
        let position = Position::default()
            .set_contents(G1, Some(Material::WN))
            .set_contents(G3, Some(Material::WN))
            .set_contents(E2, None);
        let state = MoveState::new(position);
        let san = to_san(&state, LegalMove::Standard(G1, E2));
        assert_eq!(san, "N1e2");
    }

    #[test]
    fn test_castling() {
        let position = Position::default()
            .set_contents(F1, None)
            .set_contents(G1, None);
        let state = MoveState::new(position);
        assert_eq!(to_san(&state, LegalMove::ShortCastle), "O-O");

        let position = Position::default()
            .set_contents(B1, None)
            .set_contents(C1, None)
            .set_contents(D1, None);
        let state = MoveState::new(position);
        assert_eq!(to_san(&state, LegalMove::LongCastle), "O-O-O");
    }

    #[test]
    fn test_en_passant() {
        let position = Position::default()
            .set_en_passant(Some(D6))
            .set_contents(D5, Some(Material::BP))
            .set_contents(E5, Some(Material::WP))
            .set_contents(E2, None);
        let state = MoveState::new(position);
        assert_eq!(to_san(&state, LegalMove::EnPassant(E5, D6)), "exd6");
    }

    #[test]
    fn test_check_suffix() {
        // Set up a position where Qh5 gives check
        // Simplest: scholar's mate setup fragment
        let position = Position::default()
            .set_contents(D1, None) // clear queen from d1
            .set_contents(H5, Some(Material::WQ)); // put queen on h5 aimed at f7/e8
        let state = MoveState::new(position);
        let san = to_san(&state, LegalMove::Standard(H5, F7));
        assert_eq!(san, "Qxf7+");
    }

    #[test]
    fn test_checkmate_suffix() {
        // Fool's mate: 1. f3 e5 2. g4 Qh4#
        let mut state = MoveState::default();
        state.apply_move(LegalMove::Standard(F2, F3));
        state.apply_move(LegalMove::DoubleAdvance(E7, E5));
        state.apply_move(LegalMove::DoubleAdvance(G2, G4));
        // Now black plays Qh4#
        let san = to_san(&state, LegalMove::Standard(D8, H4));
        assert_eq!(san, "Qh4#");
    }

    #[test]
    fn test_piece_capture() {
        let position = Position::default()
            .set_contents(C4, Some(Material::WB))
            .set_contents(F1, None);
        let state = MoveState::new(position);
        // Bishop on C4 can capture f7 pawn
        assert_eq!(to_san(&state, LegalMove::Standard(C4, F7)), "Bxf7+");
    }

    // ---- from_san tests ----

    #[test]
    fn test_parse_pawn_advance() {
        let state = MoveState::default();
        assert_eq!(from_san(&state, "e4").unwrap(), LegalMove::DoubleAdvance(E2, E4));
        assert_eq!(from_san(&state, "e3").unwrap(), LegalMove::Standard(E2, E3));
    }

    #[test]
    fn test_parse_knight_move() {
        let state = MoveState::default();
        assert_eq!(from_san(&state, "Nf3").unwrap(), LegalMove::Standard(G1, F3));
    }

    #[test]
    fn test_parse_castling() {
        let position = Position::default()
            .set_contents(F1, None)
            .set_contents(G1, None);
        let state = MoveState::new(position);
        assert_eq!(from_san(&state, "O-O").unwrap(), LegalMove::ShortCastle);
        // Also accept 0-0
        assert_eq!(from_san(&state, "0-0").unwrap(), LegalMove::ShortCastle);
    }

    #[test]
    fn test_parse_castling_long() {
        let position = Position::default()
            .set_contents(B1, None)
            .set_contents(C1, None)
            .set_contents(D1, None);
        let state = MoveState::new(position);
        assert_eq!(from_san(&state, "O-O-O").unwrap(), LegalMove::LongCastle);
        assert_eq!(from_san(&state, "0-0-0").unwrap(), LegalMove::LongCastle);
    }

    #[test]
    fn test_parse_with_check_suffix() {
        let position = Position::default()
            .set_contents(D1, None)
            .set_contents(H5, Some(Material::WQ));
        let state = MoveState::new(position);
        assert_eq!(
            from_san(&state, "Qxf7+").unwrap(),
            LegalMove::Standard(H5, F7)
        );
    }

    #[test]
    fn test_parse_promotion() {
        let position = Position::default()
            .set_contents(B7, Some(Material::WP))
            .set_contents(B8, None);
        let state = MoveState::new(position);
        assert_eq!(
            from_san(&state, "b8=Q").unwrap(),
            LegalMove::Promoting(B7, B8, Promotion::Queen)
        );
        assert_eq!(
            from_san(&state, "b8=N").unwrap(),
            LegalMove::Promoting(B7, B8, Promotion::Knight)
        );
    }

    #[test]
    fn test_parse_pawn_capture() {
        let position = Position::default()
            .set_contents(E4, Some(Material::WP))
            .set_contents(E2, None)
            .set_contents(D5, Some(Material::BP));
        let state = MoveState::new(position);
        assert_eq!(
            from_san(&state, "exd5").unwrap(),
            LegalMove::Standard(E4, D5)
        );
    }

    #[test]
    fn test_parse_invalid_san() {
        let state = MoveState::default();
        assert!(from_san(&state, "Zz9").is_err());
        assert!(from_san(&state, "").is_err());
        assert!(from_san(&state, "x").is_err());
    }

    #[test]
    fn test_parse_disambiguation() {
        let position = Position::default()
            .set_contents(A3, Some(Material::WN))
            .set_contents(C3, Some(Material::WN))
            .set_contents(B1, None);
        let state = MoveState::new(position);
        assert_eq!(
            from_san(&state, "Nab5").unwrap(),
            LegalMove::Standard(A3, B5)
        );
        assert_eq!(
            from_san(&state, "Ncb5").unwrap(),
            LegalMove::Standard(C3, B5)
        );
    }

    // ---- round-trip tests ----

    #[test]
    fn test_round_trip_opening_moves() {
        let state = MoveState::default();
        let moves = vec![
            LegalMove::DoubleAdvance(E2, E4),
            LegalMove::Standard(G1, F3),
            LegalMove::DoubleAdvance(D2, D4),
            LegalMove::Standard(E2, E3),
        ];
        for lm in moves {
            let san = to_san(&state, lm);
            let parsed = from_san(&state, &san).unwrap();
            assert_eq!(parsed, lm, "round-trip failed for SAN: {}", san);
        }
    }

    #[test]
    fn test_round_trip_game_sequence() {
        // Play a few moves and verify round-trip at each step
        let mut state = MoveState::default();
        let moves = vec![
            LegalMove::DoubleAdvance(E2, E4),   // e4
            LegalMove::DoubleAdvance(E7, E5),   // e5
            LegalMove::Standard(G1, F3),        // Nf3
            LegalMove::Standard(B8, C6),        // Nc6
        ];
        for lm in moves {
            let san = to_san(&state, lm);
            let parsed = from_san(&state, &san).unwrap();
            assert_eq!(parsed, lm, "round-trip failed for SAN: {}", san);
            state.apply_move(lm);
        }
    }
}
