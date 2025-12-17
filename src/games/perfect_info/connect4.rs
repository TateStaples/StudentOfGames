use std::cmp::Ordering;
use std::fmt::Debug;
use std::iter::{IntoIterator, Iterator};
use crate::utils::{Game, Player, Reward, TraceI};
/*
+----------------------------+
| 6 13 20 27 34 41 48 55 62 |
| 5 12 19 26 33 40 47 54 61 |
| 4 11 18 25 32 39 46 53 60 |
| 3 10 17 24 31 38 45 52 59 |
| 2  9 16 23 30 37 44 51 58 |
| 1  8 15 22 29 36 43 50 57 |
| 0  7 14 21 28 35 42 49 56 | 63
+----------------------------+
*/

const WIDTH: usize = 7;
const HEIGHT: usize = 6;


const FAB_COL: u64 = 0b1111111;
const FAB_ROW: u64 = (1 << (7 * 0))
    | (1 << (7 * 1))
    | (1 << (7 * 2))
    | (1 << (7 * 3))
    | (1 << (7 * 4))
    | (1 << (7 * 5))
    | (1 << (7 * 6))
    | (1 << (7 * 7))
    | (1 << (7 * 8));

const COLS: [u64; WIDTH] = [
    FAB_COL << (7 * 0),
    FAB_COL << (7 * 1),
    FAB_COL << (7 * 2),
    FAB_COL << (7 * 3),
    FAB_COL << (7 * 4),
    FAB_COL << (7 * 5),
    FAB_COL << (7 * 6),
];

const ROWS: [u64; HEIGHT] = [
    FAB_ROW << 0,
    FAB_ROW << 1,
    FAB_ROW << 2,
    FAB_ROW << 3,
    FAB_ROW << 4,
    FAB_ROW << 5,
];

const HEURISTIC_MAP: [[Reward; WIDTH]; HEIGHT] = [
    [3.0, 4.0, 5.0, 7.0, 5.0, 4.0, 3.0],
    [4.0, 6.0, 8.0, 10.0, 8.0, 6.0, 4.0],
    [5.0, 8.0, 11.0, 13.0, 11.0, 8.0, 5.0],
    [5.0, 8.0, 11.0, 13.0, 11.0, 8.0, 5.0],
    [4.0, 6.0, 8.0, 10.0, 8.0, 6.0, 4.0],
    [3.0, 4.0, 5.0, 7.0, 5.0, 4.0, 3.0]
];

const D1_MASK: u64 = (COLS[0] | COLS[1] | COLS[2] | COLS[3] | COLS[4] | COLS[5])
    & (ROWS[3] | ROWS[4] | ROWS[5]);
const D2_MASK: u64 = (COLS[0] | COLS[1] | COLS[2] | COLS[3] | COLS[4] | COLS[5])
    & (ROWS[0] | ROWS[1] | ROWS[2] | ROWS[3]);
const H_MASK: u64 = COLS[0] | COLS[1] | COLS[2] | COLS[3] | COLS[4] | COLS[5];
const V_MASK: u64 = ROWS[0] | ROWS[1] | ROWS[2] | ROWS[3];

const fn won(bb: u64) -> bool {
    let d1 = bb & (bb >> 6) & (bb >> 12) & (bb >> 18) & D1_MASK;
    let d2 = bb & (bb >> 8) & (bb >> 16) & (bb >> 24) & D2_MASK;
    let h = bb & (bb >> 7) & (bb >> 14) & (bb >> 21) & H_MASK;
    let v = bb & (bb >> 1) & (bb >> 2) & (bb >> 3) & V_MASK;
    v + h + d1 + d2 > 0
}

#[derive(Eq, PartialEq, Clone, Default, Hash)]
pub struct Connect4 {
    my_bb: u64,  // Some sort of bitmask
    op_bb: u64,
    height: [u8; WIDTH],
}


impl PartialOrd for Connect4 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut self_subset_other = true;
        let mut other_subset_self = true;
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                let cell_self = self.cell(row, col);
                let cell_other = other.cell(row, col);
                match (cell_self, cell_other) {
                    (Some(p1), Some(p2)) if p1 != p2 => {return None}
                    (Some(_), None) => self_subset_other = false,
                    (None, Some(_)) => other_subset_self = false,
                    _ => (),  // If they are equal, do nothing
                }
            }
        }
        match (self_subset_other, other_subset_self) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl Debug for Connect4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut msg: String = String::new();
        if self.is_over() {
            msg += String::from(format!("{:?} won\n", self.winner())).as_str();
        } else {
            msg += String::from(format!("{:?} to play", self.active_player())).as_str();
        }

        let (my_char, op_char) = match self.active_player() {
            Player::P1 => ("B", "r"),
            Player::P2 => ("r", "B"),
            _ => unreachable!()
        };

        for row in (0..HEIGHT).rev() {
            for col in 0..WIDTH {
                let index = 1 << (row + HEIGHT * col);
                let c = if self.my_bb & index != 0 {
                    my_char
                } else if self.op_bb & index != 0 {
                    op_char
                } else {
                    "."
                };
                msg += c;
                msg += " ";
            }
            msg += "\n";
        }
        write!(f, "{}", msg)
        // write!(f, "Connect4 {{ my_bb: {:?}, op_bb: {:?}, height: {:?} }}", self.my_bb, self.op_bb, self.height)
    }
}
impl TraceI for Connect4 {}
impl Connect4 {
    fn winner(&self) -> Option<Player> {
        if won(self.my_bb) {
            Some(Player::P1)
        } else if won(self.op_bb) {
            Some(Player::P2)
        } else {
            None
        }
    }

    fn heuristic(&self) -> Reward {
        let norm = HEURISTIC_MAP.iter().map(|row| row.iter().sum::<Reward>()).sum::<Reward>();
        HEURISTIC_MAP.iter().enumerate().map(|(row, col)| {
            col.iter().enumerate().map(|(col, _)| {
                let index = 1 << (row + HEIGHT * col);
                if self.my_bb & index != 0 {
                    1.0/norm
                } else if self.op_bb & index != 0 {
                    -1.0/norm
                } else {
                    0.0
                }
            }).sum::<Reward>()
        }).sum::<Reward>()
    }
    
    fn cell(&self, row: usize, col: usize) -> Option<Player> {
        let index = 1 << (row + HEIGHT * col);
        if self.my_bb & index != 0 {
            Some(Player::P1)
        } else if self.op_bb & index != 0 {
            Some(Player::P2)
        } else {
            None
        }
    }
}

impl Game for Connect4 {
    type Action = u8;
    type State = Self;
    type Solver = DummySolver;
    type Trace = Self;

    fn new() -> Self {
        Self {
            my_bb: 0,
            op_bb: 0,
            height: [0; WIDTH],
        }
    }

    fn is_over(&self) -> bool {
        self.winner().is_some() || (0..WIDTH).all(|col| self.height[col] == HEIGHT as u8)
    }

    fn encode(&self) -> Self::State {
        self.clone()
    }
    fn decode(state: &Self::State) -> Self {
        state.clone()
    }
    fn trace(&self, player: Player) -> Self::Trace {
        self.clone()
    }

    fn active_player(&self) -> Player {
        if self.height.iter().sum::<u8>()&2==0 {Player::P1} else {Player::P2}
    }

    fn available_actions(&self) -> Vec<Self::Action> {
        (0..self.height.len() as u8).filter(|&x| self.height[x as usize] < HEIGHT as u8).collect()
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        let col: usize = (*action).into();
        debug_assert!(self.height[col] < HEIGHT as u8);
        s.my_bb ^= 1 << (self.height[col] + (HEIGHT as u8) * (col as u8));
        s.height[col] += 1;
        std::mem::swap(&mut s.my_bb, &mut s.op_bb);
        s
    }

    fn evaluate(&self) -> Reward {
        match self.winner() {
            Some(Player::P1) => 1.0,
            Some(Player::P2) => -1.0,
            _ => self.heuristic(),
        }
    }

    fn sample_position(_observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        vec![].into_iter()
    }
}
