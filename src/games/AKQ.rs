use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use crate::games::AKQ::AkqAction::{Bet, Call, Deal, Fold};
use crate::games::AKQ::AkqCard::{A, K, Q};
use crate::utils::*;

/// --------- AKQ Poker (Kuhn) with explicit Random dealer ---------
/// Start state: `to_move = Player::Random`, which chooses one of 6 `Deal` actions.
/// After dealing, public history is empty (code = 0) and P1 acts.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum AkqCard { A, K, Q }

impl std::fmt::Debug for AkqCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { AkqCard::A => write!(f, "A"), AkqCard::K => write!(f, "K"), AkqCard::Q => write!(f, "Q") }
    }
}

impl PartialOrd for AkqCard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use AkqCard::*;
        let r = match (*self, *other) {
            (A, A) | (K, K) | (Q, Q) => Ordering::Equal,
            (A, _) => Ordering::Greater,
            (K, A) => Ordering::Less,
            (K, Q) => Ordering::Greater,
            (Q, A) | (Q, K) => Ordering::Less,
        };
        Some(r)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum AkqAction {
    // Chance action (only legal for Player::Random at start)
    Deal(AkqCard, AkqCard), // (p1_card, p2_card)
    // Player actions
    Bet, Call, Fold,
}

impl Debug for AkqAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use AkqAction::*;
        match self {
            Deal(a, b) => write!(f, "Deal({:?},{:?})", a, b),
            Bet   => write!(f, "Bet"),
            Call  => write!(f, "Call"),
            Fold  => write!(f, "Fold"),
        }
    }
}
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum PublicState {
    Predeal,
    Postdeal,
    Call,
    Bet,
    CallCall,
    BetCall,
    BetFold,
    CallBet,
    CallBetCall,
    CallBetFold,
}

impl PartialOrd for PublicState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Less = Before, Equal = Same, Greater = After
        use PublicState::*;
        if self == other { return Some(Ordering::Equal); }
        match (self, other) {
            (Predeal, _) | (Postdeal, _)
            | (Call, CallCall) | (Call, CallBet) | (Call, CallBetCall) | (Call, CallBetFold)
            | (Bet, BetCall) | (Bet, BetFold)
            | (CallBet, CallBetCall) | (CallBet, CallBetFold)
            => Some(Ordering::Less),
            (_, Predeal) | (_, Postdeal)
            | (CallCall, Call) | (CallBet, Call) | (CallBetCall, Call) | (CallBetFold, Call)
            | (BetCall, Call) | (BetCall, BetCall) | (BetCall, BetFold)
            | (CallBetCall, CallBet) | (CallBetCall, CallBet)
            => Some(Ordering::Greater),
            _ => None,
        }
    }
}
impl Default for PublicState { fn default() -> Self { PublicState::Predeal } }
impl PublicState {
    pub fn is_terminal_code(&self) -> bool {
        matches!(self, PublicState::CallCall | PublicState::BetCall | PublicState::BetFold | PublicState::CallBetCall | PublicState::CallBetFold)
    }
    
    pub fn push_action(&self, a: &AkqAction) -> PublicState {
        use AkqAction::*;
        match (self, a) {
            (PublicState::Postdeal, Bet  ) => PublicState::Bet,   // "" + b
            (PublicState::Postdeal, Call ) => PublicState::Call,
            (PublicState::Call, Bet  ) => PublicState::CallBet,   // "c" + b  => "cb"
            (PublicState::Call, Call ) => PublicState::CallCall,
            (PublicState::Bet, Call ) => PublicState::BetCall,   // "b" + c  => "bc" (T)
            (PublicState::Bet, Fold ) => PublicState::BetFold,   // "b" + f  => "bf" (T)
            (PublicState::CallBet, Call ) => PublicState::CallBetCall,   // "cb"+ c  => "cbc" (T)
            (PublicState::CallBet, Fold ) => PublicState::CallBetFold,   // "cb"+ f  => "cbf" (T)
            (PublicState::Predeal, Deal(_, _)) => PublicState::Postdeal,
            _ => panic!("Illegal betting action {:?} at history {:?}", a, self),
        }
    }
    pub fn player(&self) -> Player {
        match self {
            PublicState::Predeal => Player::Chance,
            PublicState::Postdeal => Player::P1,
            PublicState::Call => Player::P2,
            PublicState::Bet => Player::P2,
            PublicState::CallCall | PublicState::BetCall | PublicState::BetFold | PublicState::CallBetCall | PublicState::CallBetFold => Player::Chance, // terminal; no one to act
            PublicState::CallBet => Player::P1,
        }
    }
}

/// History codes (public-only):
/// ""=0, "c"=1, "b"=2, "cc"=3, "bc"=4, "bf"=5, "cb"=6, "cbc"=7, "cbf"=8
#[derive(Clone, Eq, PartialEq, Default, Debug, Hash)]
pub struct AkqTrace {
    pub code: PublicState,
    /// The querying player’s private card (their infoset), if any
    pub my_card: Option<AkqCard>,
}
impl PartialOrd for AkqTrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.my_card.is_some() && other.my_card.is_some() && self.my_card != other.my_card {
            return None
        }
        self.code.partial_cmp(&other.code)
    }
}
impl TraceI for AkqTrace {}

#[derive(Clone, Debug, Hash)]
pub struct Akq {
    pub p1: Option<AkqCard>,
    pub p2: Option<AkqCard>,
    pub code: PublicState,        // public betting history code
}

impl Game for Akq {
    type State = Self;
    type Action = AkqAction;
    type Trace  = AkqTrace;

    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn new() -> Self {
        // Start before dealing: Random acts first.
        Self { p1: None, p2: None, code: Default::default() }
    }

    fn trace(&self, player: Player) -> Self::Trace {
        match player { 
            Player::P1 => AkqTrace { code: self.code.clone(), my_card: self.p1 },
            Player::P2 => AkqTrace { code: self.code.clone(), my_card: self.p2 },
            Player::Chance => AkqTrace {code: self.code.clone(), my_card: None}
        }
    }

    fn active_player(&self) -> Player { self.code.player() }

    fn available_actions(&self) -> Vec<Self::Action> {
        match self.code {
            PublicState::Predeal => vec![
                Deal(A, K),
                Deal(A, Q),
                Deal(K, A),
                Deal(K, Q),
                Deal(Q, A),
                Deal(Q, K),
            ],
            PublicState::Postdeal | PublicState::Call => vec![Call, Bet],
            PublicState::Bet | PublicState::CallBet => vec![Call, Fold],
            _ => vec![], // Terminal states have no available actions
        }
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();

        // Predeal: only Random may play a Deal action
        if self.p1.is_none() || self.p2.is_none() {
            if let Deal(c1, c2) = *action {
                assert_eq!(self.active_player(), Player::Chance, "Only Random can Deal.");
                assert_ne!(c1, c2, "Cannot deal duplicate cards.");
                s.p1 = Some(c1);
                s.p2 = Some(c2);
                s.code = PublicState::Postdeal;
                return s;
            } else {
                panic!("Non-deal action attempted before deal: {:?}", action);
            }
        }

        // Betting phase
        debug_assert!(!self.code.is_terminal_code(), "Cannot play from terminal.");
        s.code = self.code.push_action(action);
        s
    }

    fn is_over(&self) -> bool {
        // Over only when dealt AND reached terminal history
        self.p1.is_some() && self.p2.is_some() && self.code.is_terminal_code()
    }

    fn evaluate(&self) -> Reward {
        // Terminal payoff for P1; non-terminal = 0.0
        if !self.is_over() { return 0.0; }
        let (p1, p2) = (self.p1.unwrap(), self.p2.unwrap());
        
        let score = if p1 > p2 { 1.0 } else { -1.0 };

        match self.code {
            PublicState::CallCall => 1.0 * score,
            PublicState::BetCall => 2.0 * score,
            PublicState::BetFold => 1.0,  // P1 bet, P2 folded
            PublicState::CallBetCall => 2.0 * score,
            PublicState::CallBetFold => -1.0, // P2 bet, P1 folded
            _ => 0.0,
        }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        let player = observation_history.code.player();
        let AkqTrace { code, my_card } = observation_history;
        match (player, code.clone()) { 
            (Player::Chance, PublicState::Predeal) => vec![Akq::new()].into_iter(),
            (Player::P1, _) => { 
                vec![A, K, Q].into_iter()
                    .filter(|x| x != &my_card.unwrap())
                    .map(|x| Akq{p1: my_card, p2: Some(x), code: code.clone()})
                    .collect::<Vec<_>>().into_iter()
            }
            (Player::P2, _) => { 
                vec![A, K, Q].into_iter()
                    .filter(|x| x != &my_card.unwrap())
                    .map(|x| Akq{p2: my_card, p1: Some(x), code: code.clone()})
                    .collect::<Vec<_>>().into_iter()
            }
            _ => unreachable!("You should not be here")
        }
    }
}
