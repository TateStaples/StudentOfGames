(**********************************************************************
 *  obscuro_agent.ml  –  one-file skeleton of the FoW-Chess algorithm  *
 *  ─────────────────────────────────────────────────────────────────  *
 *********************************************************************)

module Policy = struct (* π *)
  type t = float array

  let mix (p1 : t) (p2 : t) : t =
    if Array.length p1 <> Array.length p2 then
      failwith "Policies must have the same length";
    Array.init (Array.length p1) (fun i -> 0.5 *. (p1.(i) +. p2.(i)))
end

module Utility = struct
  let conditional _strat _j = 0.0
end

module Gift = struct
  let estimate _strat _j = 0.0 (* TODO: *)
end


module Time = struct
  let within_budget () = false  (* stop immediately for now *)
end

module Resolve = struct
  let reweight_roots () = ()
end

(* module Expansion = struct
  let policy ~exploring:_ (_h : Node.t) = [||]
  let expand_leaf _h = ()
end *)

module Purify = struct
  let stable_support ~π ~top ~k:_ = [| top |]
  let project π ~support ~fallback:_ =
    let proj = Array.make (Array.length π) 0.0 in
    Array.iter (fun i -> proj.(i) <- 1.0 /. float (Array.length support)) support;
    proj
end

module Game = struct
  type state = unit
  type game = unit * unit
  type action = unit * unit * unit
  type observation = unit * unit * unit * unit
  type player = P1 | P2 | Chance | Terminal  (* ▲ vs ▼ vs C vs T *)
  let current_player(g: game) : Player.t = failwith "TODO"
  let encode(g: game) : state = failwith "TODO"
  let decode(s: state) : game = failwith "TODO"
  let is_over(g: Game) = false
  let available_actions(g: game) : Game.action list = []
  let observation (g: game) (i: Player.t) : Observation.t = failwith "TODO"
  let play(g: game) (A = ()
  let other_player = function P1 -> P2 
  | P2 -> P1
  | Chance -> raise (Invalid_argument "Chance has no other player")
  | Terminal -> raise (Invalid_argument "Terminal has no other player")

  (* TODO: These should potentially be implemented elsewhere *)
  let enumerate_positions (_obs : Observation.t list) = []
  let nodes_from_positions _p = []
  let order2_min _i_nodes = []
end

(* ─────────────────────────── Node  ───────────────────────────── *)
module Node = struct
  type status = New | Expanded | Terminal

  type t = {
    parent             : t option;
    info               : Infoset.t;           (* infoset id *) 
    mutable kids       : t list;
    mutable stat       : status;
    player             : Player.t;            (* active player*)
    action_from_parent : Game.action option;  (* action taken to reach this node *)
    mutable value      : float;               (* evaluation / CFR bookkeeping *)
  }
  let child(h: t) (a: Game.action) : t option = List.find_opt (fun n -> n.action_from_parent = Some a) h.kids (*TOOD: repalce with Map*)
  let exploration_policy(n: t) : Policy.t = failwith "TODO"
  let play_policy(n: t) : Policy.t = failwith "TODO"
  let sequence(h: t, i: Player.t) : Observation.t list =
    (* the sequence of player i at h as of the last time they played an action *)
    let rec aux n acc =
      match n.parent with
      | None -> List.rev acc
      | Some p -> aux p (Observation.t::acc)
    in
    aux h []
  let player_reach(h: t) (i: Game.player): float = failwith "TODO" (* TODO: reach probabilities for player *)
  let other_reach(h: t) (i: Game.player): float = failwith "TODO" (* TODO: reach probabilities for other player *)
  let reach(h: t) : float = let f = player_reach h in f Game.P1 * f Game.P2 f Game.Chance
end

(* ────────────────────────── Infoset ──────────────────────────── *)
module Infoset = struct
  (* available actions, cumulative regret, policy, player, observation seq *)
  type t = {
    owner         : Player.t;
    mutable seq_p : float array;   (* behaviour π(a|I) *)
    mutable cfv   : float array;   (* counter-factual values *)
  }

  let children (a: Game.action) : Set.Make(t) = failwith "TODO"
  let parent_seq (I: t) : list Game.observation = failwith "TODO" (* σ(I) *)
  let 
end

(* ─────────────────── Minimal regret minimiser ────────────────── *)
module RMPlus = struct
  type t = { probs : float array }   (* (cumulative regrets omitted) *)

  let create k = { probs = Array.make k (1.0 /. float k) }
  let strategy r = r.probs
  let update _ (_cfv : float array) = ()   (* TODO real regret update *)
end

(* ───────────────────── Stubs for missing bits ────────────────── *)
(* TODO: Everything below is just enough to make the file compile.
   Replace each section with proper implementations. *)
(* ───────────────────── Sub-game construction ─────────────────── *)
module Subgame = struct
  let min_infoset_size = 256

  let build ~(obs_seq : Observation.t list) ~(blueprint : 'b) : unit =
    let positions  = Game.enumerate_positions obs_seq in
    let i_nodes    = Game.nodes_from_positions positions in

    List.iter
      (fun j ->
         (* alternate value initialisation: *)
         let _alt =
           Utility.conditional blueprint j -. Gift.estimate blueprint j
         in
         (* TODO: store alt in table *) ())
      (Enumerator.order2_min i_nodes);

    let rec grow () =
      if List.length i_nodes < min_infoset_size then begin
        let _s = Sampler.random_state positions in
        (* TODO: sample, add to i_nodes … *)
        grow ()
      end
    in
    grow ();

    (* TODO: build resolve-/max-margin gadget roots *)
    ()
end

(* ────────────────── Solver (one-sided GT-CFR) ────────────────── *)
module Solver = struct
  let rec cfr_iter ~exploring =
    (* TODO run a single CFR sweep for [exploring] player *)
    ignore exploring

  let rec loop () =
    while Time.within_budget () do
      cfr_iter ~exploring:Player.Max;
      cfr_iter ~exploring:Player.Min;
      Resolve.reweight_roots ()
    done
end

(* ──────────────────────── Expander ───────────────────────────── *)
module Expander = struct
  let rec step ~exploring =
    let h = ref GameTree.root in
    while !h.Node.stat = Node.Expanded do
      let π̃ = Expansion.policy ~exploring !h in
      h := GameTree.get_node (Sampler.weighted_child !h π̃)
    done;
    if !h.Node.stat = Node.New then () else Expansion.expand_leaf !h

  let rec loop () =
    while Time.within_budget () do
      step ~exploring:Player.Max;
      step ~exploring:Player.Min;
    done
end

(* ──────────────────────── Agent.turn ────────────────────────── *)
module Agent = struct
  let max_support = 3

  let move (obs_seq : Observation.t list) : Action.t =
    Subgame.build ~obs_seq ~blueprint:Strategy.current;

    (* launch threads – disabled in stub build *)
    
    let solver_dom   = Domain.spawn Solver.loop
    and expander_dom = Domain.spawn Expander.loop in
    Domain.join expander_dom;
    Domain.join solver_dom;
   

    let i     = GameTree.root  (* TODO: current infoset id *) in
    let π     = Strategy.behaviour i.Node.id in
    let a_max = ref 0 in
    Array.iteri (fun idx p -> if p > π.(!a_max) then a_max := idx) π;

    let support = Purify.stable_support ~π ~top:!a_max ~k:max_support in
    let π_play  = Purify.project π ~support ~fallback:!a_max in
    Sampler.weighted_action π_play
end
(* 
module Sampler = struct
  let random_state _positions = ()
  let weighted_child (_n : Node.t) (weights : float array) =
    if Array.length weights = 0 then failwith "empty weights";
    (* pick first child for stub: *)
    match _n.Node.kids with
    | x::_ -> x
    | []   -> failwith "no child"
  let weighted_action (π : float array) =
    (* pick argmax for now *)
    let max_i = ref 0 in
    Array.iteri (fun i p -> if p > π.(!max_i) then max_i := i) π;
    !max_i
end *)
(* ────────────────────── Playing a match ─────────────────────── *)
let rec play_game (state : Game.state) =
  match Game.is_over state with
  | Some winner -> winner
  | None ->
      let obs = Game.observation state in
      let a   = Agent.move obs in
      play_game (Game.apply_action state a)

(* Compile-test entry to silence “unused function” warnings. *)
let () =
  ignore (play_game ())
