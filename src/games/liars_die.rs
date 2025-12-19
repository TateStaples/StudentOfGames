//! # Liar's Dice (Liar's Die)
//!
//! An imperfect information poker-like game where players bid on dice values.
//! Each player has 5 dice and cannot see opponent's values. Players must either
//! raise the bid or call "liar" on the opponent's claim. A great benchmark game
//! for testing imperfect information game solvers.

use crate::games::liars_die::LiarsDieAction::{BullShit, Deal, Raise};
use crate::utils::*;
use rand::seq::IndexedRandom;
use std::cmp::Ordering;
use std::fmt::Debug;

const DICE_PER_PLAYER: usize = 5;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub enum Die {
    Two, Three, Four, Five, Six, One
} 
const ALL_DIE: [Die; 6] = [Die::One, Die::Two, Die::Three, Die::Four, Die::Five, Die::Six];

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum LiarsDieAction {
    Raise(Die, u8),
    Deal(Vec<Die>, Vec<Die>),
    BullShit
}
impl PartialOrd for LiarsDieAction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use LiarsDieAction::*;
        match (self, other) {
            (BullShit, BullShit) => Some(Ordering::Equal),
            (Raise(d1, c1), Raise(d2, c2)) => {
                match c1.cmp(c2) { 
                    Ordering::Equal => d1.partial_cmp(&d2),
                    _ => Some(c1.cmp(c2))
                }
            },
            _ => None
        }
    }
}
#[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct LiarsDieTrace {
    pub bet_history: Vec<LiarsDieAction>,
    pub my_dice: Vec<Die>
}
impl PartialOrd for LiarsDieTrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.my_dice != other.my_dice { 
            return match (self.my_dice.len(), other.my_dice.len()) {
                (0, _) => Some(Ordering::Less),
                (_, 0) => Some(Ordering::Greater),
                _ => None
            }
        }
        
        if self.bet_history == other.bet_history {Some(Ordering::Equal)}
        else if self.bet_history.starts_with(&other.bet_history) {Some(Ordering::Greater)}
        else if other.bet_history.starts_with(&self.bet_history) {Some(Ordering::Less)}
        else {None}
    }
}
impl TraceI for LiarsDieTrace {}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct LiarsDie {
    p1: Vec<Die>,
    p2: Vec<Die>,
    bet_history: Vec<LiarsDieAction>
}

impl LiarsDie {
    fn total_die(&self) -> u8 {
        (self.p1.len() + self.p2.len()) as u8
    }
    
    fn pre_deal(&self) -> bool {
        self.bet_history.is_empty() && self.p1.is_empty() && self.p2.is_empty()
    }
}

impl Game for LiarsDie{
    type State = Self;
    type Solver = DummySolver;
    type Action = LiarsDieAction;
    type Trace = LiarsDieTrace;

    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn new() -> Self {
        LiarsDie {
            p1: vec![],
            p2: vec![],
            bet_history: vec![]
        }
    }

    fn trace(&self, player: Player) -> Self::Trace {
        match player { 
            Player::P1 => {
                LiarsDieTrace {
                    my_dice: self.p1.clone(),
                    bet_history: self.bet_history.clone()
                }
            }, Player::P2 => {
                LiarsDieTrace {
                    my_dice: self.p2.clone(),
                    bet_history: self.bet_history.clone()
                }
            }, _ => unreachable!()
        }
    }

    fn active_player(&self) -> Player {
        if self.pre_deal() { Player::Chance }
        else if self.bet_history.len()%2 == 0 { Player::P1 }
        else { Player::P2 }
    }

    fn available_actions(&self) -> Vec<Self::Action> {
        let mut res = vec![];
        if self.pre_deal() {
            // Provide a generic random deal plus all single-die explicit deals for tests
            res.push(Deal(vec![], vec![]));
            for d1 in ALL_DIE.iter() {
                for d2 in ALL_DIE.iter() {
                    res.push(Deal(vec![d1.clone()], vec![d2.clone()]));
                }
            }
            return res;
        }
        if !self.bet_history.is_empty() {
            res.push(BullShit)
        }
        let last = self.bet_history.last();
        let max_count = std::cmp::max(6, self.p1.len() + self.p2.len());
        for count in 1..=max_count {
            for die in ALL_DIE.clone() {
                if die == Die::One {continue;}
                let action = Raise(die, count as u8);
                if let Some(last) = last {
                    if matches!(last.partial_cmp(&action), Some(Ordering::Less)) {
                        res.push(action)   
                    }
                } else {
                    res.push(action)
                }
            }
        }
        res
    }

    fn play(&self, action: &Self::Action) -> Self {
        debug_assert!(self.bet_history.is_empty() || action.partial_cmp(self.bet_history.last().unwrap()).unwrap_or(Ordering::Greater) == Ordering::Greater);
        if let Deal(p1, p2) = action {
            // When invoked with empty dice vectors, sample a full 5-dice deal for each player
            let (p1_deal, p2_deal) = if p1.is_empty() && p2.is_empty() {
                let mut rng = rand::rng();
                let sample_hand = |rng: &mut _| {
                    (0..DICE_PER_PLAYER)
                        .map(|_| ALL_DIE.choose(rng).unwrap().clone())
                        .collect::<Vec<_>>()
                };
                (sample_hand(&mut rng), sample_hand(&mut rng))
            } else {
                // Allow explicitly provided deals of any length (tests use single-die hands)
                (p1.clone(), p2.clone())
            };

            Self {
                p1: p1_deal,
                p2: p2_deal,
                bet_history: vec![]
            }
        } else {
            let mut bet_history = self.bet_history.clone();
            bet_history.push(action.clone());
            Self {
                p1: self.p1.clone(),
                p2: self.p2.clone(),
                bet_history
            }
        }
        
    }

    fn is_over(&self) -> bool {
        let max = Raise(Die::Six, self.total_die());
        self.bet_history.last() == Some(&BullShit)
        || self.bet_history.last() == Some(&max)
    }
    

    fn evaluate(&self) -> Reward {
        if self.is_over() { 
            let last_bet_idx = self.bet_history.iter().rposition(|x| matches!(x, Raise(_, _))).expect("Over before any bets");
            let last_bet = &self.bet_history[last_bet_idx];
            let last_player = if last_bet_idx%2==0 {Player::P1} else {Player::P2};
            let win_score = if last_player == Player::P1 {1.0} else {-1.0};
            // println!("Eval Debug: {:?}", (last_bet_idx, last_bet, last_player, win_score));
            match last_bet {
                Raise(d, c) => {
                    let p1_c = self.p1.iter().filter(|&x| x==d || x==&Die::One).count();
                    let p2_c = self.p2.iter().filter(|&x| x==d || x==&Die::One).count();
                    let win = (*c as usize) <= p1_c + p2_c;
                    if win {win_score} else {-win_score}
                }
                _ => unreachable!()
            }
        } else {
            // match self.active_player() {
            //     Player::P1 => self.p1.len() as Reward - (self.total_die() as Reward)/2.0,
            //     Player::P2 => self.p2.len() as Reward - (self.total_die() as Reward)/2.0,
            //     _ => 0.0
            // }
            // neural::nn_eval(self)  // DISABLED: neural module no longer in liars_die.rs
            0.0  // Placeholder - use neural.rs module instead
        }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        let Self::Trace { bet_history, my_dice } = observation_history;
        if bet_history.is_empty() && my_dice.is_empty() {
            return vec![Self::new()].into_iter();
        }

        const SAMPLE_COUNT: usize = 128;
        let mut rng = rand::rng();
        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let opp_hand = (0..DICE_PER_PLAYER)
                .map(|_| ALL_DIE.choose(&mut rng).unwrap().clone())
                .collect::<Vec<_>>();
            samples.push(Self {
                p1: my_dice.clone(),
                p2: opp_hand,
                bet_history: bet_history.clone(),
            });
        }
        samples.into_iter()
    }
}

// DISABLED: EncodeToTensor implementation - now using state_encoding module in neural.rs
// impl<B: Backend> EncodeToTensor<B> for LiarsDie {
//     fn encode_tensor(&self, device: &B::Device, _perspective: Player) -> burn::tensor::Tensor<B, 1> {
//         // Encoding functions now in neural::state_encoding module
//         use burn::tensor::Tensor as BurnTensor;
//         todo!("Use neural::state_encoding functions instead")
//     }
//     
//     const INPUT_SIZE: usize = 156;
// }

// DISABLED: Old neural module - replaced by new neural.rs with state_encoding
/*
mod neural {
    use crate::games::liars_die::*;
    use crate::games::resources::model_55_joker::Model;
    use std::cell::UnsafeCell;
    use std::sync::OnceLock;
    use burn::backend::ndarray::{NdArray as B, NdArrayDevice as Device};
    // If you switch to WGPU later: use burn::backend::wgpu::{Wgpu as B, WgpuDevice as Device};

    // ----- constants inferred from your Python calc_args -----
    pub const SIDES: usize = 6;
    pub const D1: usize = 5; // dice for P1
    pub const D2: usize = 5; // dice for P2
    pub const MAX_DICE_PER_PLAYER: usize = 5;

    // Calculated Hyperparameters

    pub const D_PUB_BASE: usize = (D1 + D2) * SIDES; // 60 calls (count-major, 10 counts × 6 faces)
    pub const LIE_ACTION: usize = D_PUB_BASE;        // 60
    pub const CUR_INDEX: usize = D_PUB_BASE + 1;     // 61
    pub const D_PUB_PER_PLAYER: usize = CUR_INDEX + 1; // 62
    pub const D_PUB: usize = 2 * D_PUB_PER_PLAYER;     // 124

    pub const D_PRI_BASE: usize = MAX_DICE_PER_PLAYER * SIDES; // 30 (5 slots × 6 faces)
    pub const PRI_INDEX: usize = D_PRI_BASE;                    // 30
    pub const D_PRI: usize = D_PRI_BASE + 2;                    // 32 (two perspective bits)

    // ----- helpers -----
    fn die_to_face_idx(d: &Die) -> usize {  // TODO: their network assumes you can raise to betting ones
        match d {
            Die::One => 0,
            Die::Two => 1,
            Die::Three => 2,
            Die::Four => 3,
            Die::Five => 4,
            Die::Six => 5,
        }
    }

    // count-major action id: (count-1)*SIDES + (face-1)
    fn raise_action_id(face_idx: usize, count: usize) -> usize {
        (count - 1) * SIDES + face_idx// count in 1..=D1+D2 (10), face_idx in 0..=5
    }

    // ----- encoders aligned to your Python layout -----
    pub fn encode_public(g: &LiarsDie) -> [f32; D_PUB] {
        let mut x = [0.0f32; D_PUB];

        // 1) CUR_INDEX: 1 at "current player" segment
        let cur = match g.active_player() {
            Player::P1 => 0,
            Player::P2 => 1,
            _ => 0, // pre-deal/chance → default to P1 segment like Game.make_state()
        };
        x[cur * D_PUB_PER_PLAYER + CUR_INDEX] = 1.0;

        // 2) Past actions (per-player segment)
        // bet_history[0] by P1, [1] by P2, ...
        for (i, a) in g.bet_history.iter().enumerate() {
            let player_seg = (i % 2) * D_PUB_PER_PLAYER;
            match a {
                Raise(face, count) => {
                    let f = die_to_face_idx(face);
                    let c = (*count as usize).clamp(1, D1 + D2); // 1..10
                    let aid = raise_action_id(f, c);
                    debug_assert!(aid < D_PUB_BASE);
                    if aid < D_PUB_BASE {
                        x[player_seg + aid] = 1.0;
                    }
                }
                BullShit => {
                    x[player_seg + LIE_ACTION] = 1.0;
                }
                Deal(_, _) => {
                    // Deals aren't represented in the public one-hot action space
                }
            }
        }

        x
    }

    pub fn encode_private(g: &LiarsDie) -> [f32; D_PRI] {
        let mut x = [0.0f32; D_PRI];

        // Perspective = active player (adjust if you trained on a fixed perspective)
        let (me, pidx) = match g.active_player() {
            Player::P1 => (&g.p1, 0),
            Player::P2 => (&g.p2, 1),
            _ => (&g.p1, 0),
        };
        // Set perspective one-hot
        x[PRI_INDEX + pidx] = 1.0;

        // New encoding (face × slots): for each face, set first 'count' slots to 1
        // Slots per face = MAX_DICE_PER_PLAYER = 5
        // Index: (face_idx)*MAX_DICE_PER_PLAYER + slot_idx
        // Count dice by face:
        let mut counts = [0usize; SIDES];
        for d in me {
            counts[die_to_face_idx(d)] += 1;
        }
        for face_idx in 0..SIDES {
            let c = counts[face_idx].min(MAX_DICE_PER_PLAYER);
            for slot in 0..c {
                x[face_idx * MAX_DICE_PER_PLAYER + slot] = 1.0;
            }
        }

        x
    }

    // ----- eval with explicit device & rank-1 tensors -----

    // Wrap a T so we can stick it in statics without Sync.
    struct NotSync<T>(UnsafeCell<T>);
    unsafe impl<T> Sync for NotSync<T> {} // <-- YOU PROMISE no concurrent access

    static DEVICE: OnceLock<NotSync<Device>> = OnceLock::new();
    static MODEL:  OnceLock<NotSync<Model<B>>> = OnceLock::new();

    #[inline]
    fn device() -> &'static Device {
        let ns = DEVICE.get_or_init(|| NotSync(UnsafeCell::new(Device::default())));
        // SAFETY: caller must ensure single-threaded or external synchronization.
        unsafe { &*ns.0.get() }
    }

    #[inline]
    fn model_mut() -> &'static mut Model<B> {
        let ns = MODEL.get_or_init(|| {
            let dev = device();
            NotSync(UnsafeCell::new(Model::<B>::new(dev)))
        });
        // SAFETY: caller must ensure single-threaded or external synchronization.
        unsafe { &mut *ns.0.get() }
    }

    pub fn nn_eval(g: &LiarsDie) -> Reward {
        use burn::tensor::Tensor as BurnTensor;
        let device = Device::default();
        let pub_feat = encode_public(g);
        let priv_feat = encode_private(g);

        // Create tensors using from_floats
        let pub_tensor = BurnTensor::from_floats(pub_feat.as_slice(), &device);
        let priv_tensor = BurnTensor::from_floats(priv_feat.as_slice(), &device);

        // Use the constructor your generated file provides:
        // common options are new(&Device), init(&Device), or default()
        let model = model_mut();//Model::<B>::new(&DEVICE);

        let out = model.forward(priv_tensor, pub_tensor); // -> Tensor<B,1> with 1 scalar
        let data = out.into_data();
        let val = data.as_slice::<f32>().unwrap()[0];
        val as Reward
    }

}
*/

// Neural network encoding implementation (DISABLED: neural module removed)
// impl<B: burn::tensor::backend::Backend> crate::utils::EncodeToTensor<B> for LiarsDie {
//     const INPUT_SIZE: usize = 94; // D_PRI (32) + D_PUB (62)
//     
//     fn encode_tensor(&self, device: &B::Device, perspective: Player) -> burn::tensor::Tensor<B, 1> {
//         use burn::tensor::Tensor;
//         use crate::neural::state_encoding::*;
//         
//         // Encode private and public states
//         let (priv_features, pub_features) = encode_liars_die_state(self, perspective);
//         
//         // Concatenate private and public features
//         let mut all_features = Vec::with_capacity(Self::INPUT_SIZE);
//         all_features.extend(priv_features);
//         all_features.extend(pub_features);
//         
//         Tensor::from_floats(all_features.as_slice(), device)
//     }
// }
// 
// /// Encode Liar's Dice state for neural network
// /// Returns (private_features, public_features)
// fn encode_liars_die_state(state: &LiarsDie, perspective: Player) -> (Vec<f32>, Vec<f32>) {
//     const D_PRI: usize = 32;  // max_dice=5 * sides=6 + 2
//     const D_PUB: usize = 62;  // 2 * (max_actions + 1 + 1)
//     
//     let mut priv_features = vec![0.0; D_PRI];
//     let mut pub_features = vec![0.0; D_PUB];
//     
//     // Encode private dice (one-hot encoding of each die)
//     let my_dice = match perspective {
//         Player::P1 => &state.p1,
//         Player::P2 => &state.p2,
//         Player::Chance => &vec![],
//     };
//     
//     for (die_idx, die) in my_dice.iter().enumerate() {
//         if die_idx >= 5 { break; } // Max 5 dice
//         let face_value = match die {
//             Die::One => 0,
//             Die::Two => 1,
//             Die::Three => 2,
//             Die::Four => 3,
//             Die::Five => 4,
//             Die::Six => 5,
//         };
//         priv_features[die_idx * 6 + face_value] = 1.0;
//     }
//     
//     // Player indicator (last 2 bits)
//     match perspective {
//         Player::P1 => priv_features[D_PRI - 2] = 1.0,
//         Player::P2 => priv_features[D_PRI - 1] = 1.0,
//         Player::Chance => {}
//     }
//     
//     // Encode public action history
//     // This is simplified - full encoding would track all bet details
//     for (idx, action) in state.bet_history.iter().take(30).enumerate() {
//         match action {
//             LiarsDieAction::Raise(die, count) => {
//                 // Encode as normalized count
//                 pub_features[idx] = (*count as f32) / 10.0;
//             }
//             LiarsDieAction::BullShit => {
//                 pub_features[idx] = 1.0;
//             }
//             _ => {}
//         }
//     }
//     
//     // Current player indicators
//     let current = state.active_player();
//     match current {
//         Player::P1 => pub_features[D_PUB - 2] = 1.0,
//         Player::P2 => pub_features[D_PUB - 1] = 1.0,
//         Player::Chance => {}
//     }
//     
//     (priv_features, pub_features)
// }
