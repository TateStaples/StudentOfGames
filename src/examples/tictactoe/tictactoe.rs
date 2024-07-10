use crate::game::{FixedGame, FixedSize, Game, HasTurnOrder};

#[derive(Clone, Copy, Debug, PartialEq, Eq, std::hash::Hash, PartialOrd, Ord)]
pub(crate) enum PlayerId {
    X,
    O,
}

impl HasTurnOrder for PlayerId {
    fn prev(&self) -> Self {
        self.next()
    }

    fn next(&self) -> Self {
        match self {
            PlayerId::O => PlayerId::X,
            PlayerId::X => PlayerId::O,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Action {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

impl From<usize> for Action {
    fn from(i: usize) -> Self {
        let row = i / 3;
        let col = i % 3;
        Self { row, col }
    }
}

impl Into<usize> for Action {
    fn into(self) -> usize {
        self.row * 3 + self.col
    }
}

#[derive(Debug, PartialEq, Eq, std::hash::Hash, Clone)]
pub struct TicTacToe {
    board: [[Option<PlayerId>; 3]; 3],
    player: PlayerId,
    turn: usize,
}

struct ActionIterator {
    game: TicTacToe,
    i: usize,
}

impl Iterator for ActionIterator {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        while self.i < 9 {
            let action: Action = self.i.into();
            self.i += 1;
            if self.game.board[action.row][action.col].is_none() {
                return Some(action);
            }
        }

        None
    }
}

impl TicTacToe {
    fn won(&self, player: PlayerId) -> bool {
        let p = Some(player);
        if self.board[0][0] == p && self.board[0][1] == p && self.board[0][2] == p {
            return true;
        }
        if self.board[1][0] == p && self.board[1][1] == p && self.board[1][2] == p {
            return true;
        }
        if self.board[2][0] == p && self.board[2][1] == p && self.board[2][2] == p {
            return true;
        }
        if self.board[0][0] == p && self.board[1][0] == p && self.board[2][0] == p {
            return true;
        }
        if self.board[0][1] == p && self.board[1][1] == p && self.board[2][1] == p {
            return true;
        }
        if self.board[0][2] == p && self.board[1][2] == p && self.board[2][2] == p {
            return true;
        }
        if self.board[0][0] == p && self.board[1][1] == p && self.board[2][2] == p {
            return true;
        }
        if self.board[0][2] == p && self.board[1][1] == p && self.board[2][0] == p {
            return true;
        }

        false
    }
}
impl FixedSize<9, 1> for TicTacToe {}
impl Game for TicTacToe {
    type PlayerId = PlayerId;
    type Action = Action;
    type ActionIterator = ActionIterator;
    type PublicInformation = [[[f32; 3]; 3]; 3];

    const MAX_TURNS: usize = 9;
    const NAME: &'static str = "TicTacToe";
    const NUM_PLAYERS: usize = 2;
    const DIMS: &'static [i64] = &[3, 3, 3];

    fn new() -> Self {
        Self {
            board: [[None; 3]; 3],
            player: PlayerId::X,
            turn: 0,
        }
    }

    fn player(&self) -> Self::PlayerId {
        self.player
    }

    fn is_over(&self) -> bool {
        self.won(self.player) || self.won(self.player.prev()) || self.turn == Self::MAX_TURNS
    }

    fn reward(&self, player_id: Self::PlayerId) -> f32 {
        if self.won(player_id) {
            1.0
        } else if self.won(player_id.next()) {
            -1.0
        } else {
            0.0
        }
    }

    fn iter_actions(&self) -> Self::ActionIterator {
        ActionIterator {
            game: self.clone(),
            i: 0,
        }
    }

    fn step(&mut self, action: &Self::Action) -> bool {
        assert!(action.row < 3);
        assert!(action.col < 3);
        assert!(self.board[action.row][action.col].is_none());
        self.board[action.row][action.col] = Some(self.player);
        self.player = self.player.next();
        self.turn += 1;
        self.is_over()
    }

    fn public_information(&self) -> Self::PublicInformation {
        let mut s = [[[0.0; 3]; 3]; 3];
        for row in 0..3 {
            for col in 0..3 {
                if let Some(p) = self.board[row][col] {
                    if p == self.player {
                        s[0][row][col] = 1.0;
                    } else {
                        s[1][row][col] = 1.0;
                    }
                } else {
                    s[2][row][col] = 1.0;
                }
            }
        }
        s
    }

    fn print(&self) {
        for row in 0..3 {
            for col in 0..3 {
                print!(
                    "{}",
                    match self.board[row][col] {
                        Some(PlayerId::X) => "x",
                        Some(PlayerId::O) => "o",
                        None => ".",
                    }
                );
            }
            println!();
        }
        println!();
    }
}
