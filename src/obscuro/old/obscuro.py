# Imports
import enum
import numpy as np
import random
import threading
import time
from typing import *
from copy import deepcopy

# Constants
SOLVE_TIME = 5
MIN_INFO_SIZE = 200
MAX_SUPPORT = 3
EXPANDERS = 1

# Data types
REWARD = float
PROBABILITY = float
class ResolveActions(enum.Enum):
    ENTER = 0
    SKIP = 1
class Game:  # RPS
    class State:
        def __init__(self, *args, **kwargs) -> None:
            self.args = args
            self.kwargs = kwargs
        def available_actions(self): return Game.decode(self).available_actions()
        def current_player(self): return Game.decode(self).current_player()
    class Player(enum.Enum):  # type within Game
        P1 = 0      # ▲
        P2 = 1      # ▼
        CHANCE = 2  # C
        TERMINAL = 3# T
        def other(self) -> Self: return ~self
        def __repr__(self): return self.name

        def mult(self):
            match self:
                case self.P1: return 1
                case self.P2: return -1
                case self.CHANCE: return 0
                case self.TERMINAL: return 0
            raise ValueError("Unnamed player", self)

        def __invert__(self):
            match self:
                case self.P1: return self.P2
                case self.P2: return self.P1
            raise ValueError("Chance has no other player")
    class Action(enum.Enum):
        ROCK = 0
        PAPER = 1
        SCISSORS = 2

        def __repr__(self):
            return self.name
    class Observation:
        def __init__(self, *args): self.args = args
        def __repr__(self): return f"O{self.args}"
        def __eq__(self, other): return str(self) == str(other)
        def __ne__(self, other): return not (self == other)
        def __hash__(self): return hash(str(self))
    def __init__(self, history: list[Action]):
        self.history = history
    def current_player(self) -> Player:
        return [Game.Player.P1, Game.Player.P2, Game.Player.TERMINAL][len(self.history)]
    def encode(self) -> State:
        return Game.State(self.history)
    @staticmethod
    def decode(state: State) -> 'Game':
        return Game(*state.args, **state.kwargs)
    def available_actions(self) -> list[Action]:
        return [Game.Action.ROCK, Game.Action.PAPER, Game.Action.SCISSORS]
    def observation(self, player: Player) -> Observation:
        return Game.Observation(self.history[-1] if len(self.history) > 0 and player == Game.Player.P1 else None)
    def play(self, action: Action) -> Self:
        return Game(self.history + [action])
    def is_over(self) -> bool:
        assert len(self.history) < 3
        return len(self.history) == 2
    def evaluate(self) -> float:
        if not self.is_over(): return 0.0
        payoffs = [
            #r p s - p1/p2
            [0,1,-5],  # r
            [-1,0,1],  # p
            [5,-1,0],  # s
        ]
        return payoffs[self.history[-1].value][self.history[0].value]

    def filter(self, observation: Observation) -> Self:
        return Game(self.history + [observation])
    @staticmethod
    def possible_positions(observations: list[Observation]) -> Generator['Game', None, None]:
        """Enumerate all possible positions from a list of observations."""
        if len(observations) == 1:
            for action in [Game.Action.ROCK, Game.Action.PAPER, Game.Action.SCISSORS]:
                yield Game([action])
        return
        # raise NotImplementedError("This method should be implemented")
class Policy[A]:  # Mix of Policy and RegretMinimizer
    C = 1
    def __init__(self, initial_reward: dict[A, REWARD], multiplier: int):
        self.multiplier = multiplier  # +1 = maximizing, -1 = minimizing
        actions = initial_reward.keys()
        self.visits = {a: 2 for a in actions}  # start with prior samples of -1 & +1
        self.net_variances = {a: 2 for a in actions}
        self.expansions: dict[A, int] = {a: 0 for a in actions}
        self.expectations = initial_reward.copy()
        # best = max(initial_reward, key=lambda a: initial_reward[a]*multiplier)
        self.acc_regret = initial_reward.copy()#{a: (a == best) for a in actions}  # INITIALIZE strategy towards the best payoff
        self.net_regret = sum(self.acc_regret.values())
        self.num_updates = 1

    def __repr__(self):
        return f"P({self.multiplier=}, {self.expectations=}, {self.acc_regret=})"

    def _avg_variance(self, action: A) -> REWARD:
        return self.net_variances[action] / self.visits[action]
    
    def _q(self, a: A) -> REWARD: # Q(I, A) := u(x, y|I, a) + C σ(I,a) sqrt(N(I))/(1+N(I))
        return self.expectations[a] * self.multiplier + self.C * self._avg_variance(a) * np.sqrt(sum(self.expansions.values()))/(1+self.expansions[a])

    def _puct(self) -> dict[A, PROBABILITY]:# ~x_PUCT = one with the highest_quality
        quality = [self._q(a) for a in self.visits]
        x = {a: 0 for a in self.visits}
        x[np.argmax(quality)] = 1
        return x

    def _max(self) -> dict[A, PROBABILITY]: # ~x_Max = uniform where play > 0
        exploit = self.exploit_policy()
        support = sum(exploit[a]>0 for a in exploit)
        return {a: 1/support for a in exploit}  # FIXME

    def _sample_dist(self) -> dict[Game.Action, PROBABILITY]: # ~x_sample = 1/2 (Max + PUCT)
        puct = self._puct()
        _max = self._max()
        return {a: (puct[a]+_max[a])/2 for a in self.visits}

    def _sample(self, x: dict[Game.Action, PROBABILITY]):
        net_prob = sum(x.values())
        if net_prob == 0:
            return random.choice(list(x.keys()))
        r = np.random.random()
        for a in x:
            r -= x[a]/net_prob
            if r <= 0:
                return a
        raise ValueError("Action Probabilities sum below 1")

    def update(self):
        # TODO: chance nodes might have numerical instability -> same with steady state
        momentum_coeff = self.num_updates/(self.num_updates+1)
        self.num_updates += 1
        ev = self.expectation()
        inst_regret = {a: self.expectations[a]*self.multiplier-ev for a in self.expectations}
        self.acc_regret = {a: max(0.0, momentum_coeff * self.acc_regret[a] + inst_regret[a]) for a in self.acc_regret}
        self.net_regret = sum(self.acc_regret.values())
        # if self.multiplier == 1:
        #     print(f"\t{ev=}, {inst_regret=}, {self.acc_regret=}")
        #     print(f"\t{self.exploit_policy()=}")
        #     print(f"\t{self.expectation()=}, {self.expectations=}")
        # raise NotImplementedError("This method should be implemented ")

    def expectation(self):
        return sum(self.p_exploit(a)*self.expectations[a]*self.multiplier for a in self.expectations)

    def explore(self) -> Game.Action:
        return self._sample(self._sample_dist())

    def exploit(self) -> Game.Action:
        return self._sample(self.acc_regret)

    def exploit_policy(self) -> dict[A, REWARD]:
        actions = self.acc_regret.keys()
        if self.net_regret == 0:
            return {a: 1/len(actions) for a in actions}
        return {a: self.acc_regret[a] / self.net_regret for a in self.acc_regret}

    def p_exploit(self, a: A) -> PROBABILITY:
        if self.net_regret == 0:
            return 1/len(self.acc_regret)
        return self.acc_regret[a]/self.net_regret

    def stable_actions(self) -> set[A]:
        return set(self.acc_regret.keys())  # FIXME: this isn't exactly right, stable actions are those that have always had probability > 0 since 1/2 t

    def purified(self) -> A:
        best_action = max(self.acc_regret.keys(), key=lambda a: self.acc_regret[a]) # a*
        stable = self.stable_actions()
        stable.add(best_action)
        support = list(sorted(stable, key=lambda a: self.p_exploit(a)))[:MAX_SUPPORT]
        play_policy = {a: (self.acc_regret[a] if a in support else 0) for a in self.acc_regret}
        return self._sample(play_policy)
class History:  # Histories h
    class Status(enum.Enum):
        NEW = 0
        EXPANDED = 1
        TERMINAL = 2
        VISITED = 3
    
    def __init__(self, game: Game, parent):  # self = h
        self.parent = parent
        self.status = History.Status.TERMINAL if game.is_over() else History.Status.NEW if parent is not None else History.Status.VISITED
        self.info = None
        # TODO: idk if I've done enough to handle chance nodes
        self.state: Game.State = game.encode()  # The state of the game at this node
        self.observations: dict[Game.Player, Game.Observation] = {p: game.observation(p) for p in [Game.Player.P1, Game.Player.P2]} # o_i(h)
        self.children: dict[Game.Action, History] = {}              # a, ha
        self.reach_probs: dict[Game.Player, float] = {p: 1.0 for p in [Game.Player.P1, Game.Player.P2]}  # π
    def __repr__(self):
        return f"H{round(hash(self)%9999)}({self.history(self.player())}"
    @staticmethod
    def first_node() -> 'History':
        h = History(Game([]), None)
        return h

    def new(self) -> bool: return self.status == History.Status.NEW
    def expanded(self) -> bool: return self.status == History.Status.EXPANDED
    def terminal(self) -> bool: return self.status == History.Status.TERMINAL
    def player(self) -> Game.Player: return self.state.current_player()

    def payoff(self):  # u: Z -> [-1, +1]
        # might merge with expectation
        assert self.status != History.Status.EXPANDED
        return Game.decode(self.state).evaluate()

    def expand(self):
        assert self.status == History.Status.VISITED
        self.status = History.Status.EXPANDED
        game = Game.decode(self.state)
        for a in game.available_actions():
            new_game = game.play(a)
            child = History(new_game, self)
            # print(f"{self=}, {a=}, {child=}")
            self.children[a] = child
        infostate = self.history(game.current_player())
        if infostate in info_sets:
            info_sets[infostate].add_history(self)
        else:
            # print("New Infostate:", infostate)
            info_sets[infostate] = Info(self)
        # print(f"new policy ({self}):{info_sets[infostate].policy}")

    def history(self, player: Game.Player) -> tuple[Game.Observation, ...]:  # s_i(h)
        agg: list[Game.Observation] = []
        current = self
        while current is not None:
            obs: Game.Observation = current.observations.get(player)
            if obs is not None:
                agg.append(obs)
            current = current.parent
        agg.reverse()
        return tuple(agg)

    def get_info(self, player: Game.Player) -> 'Info':
        if player == self.player():
            return self.info
        if self.parent is None:
            i = Info(self)
            i.player = player
            return i
        return self.parent.get_info(player)
    
    def player_reach(self, player: Game.Player) -> float:
        """Return the reach probability for the given player."""
        return self.reach_probs.get(player, 0.0)
    def other_player_reach(self, player: Game.Player) -> float:
        """Return the reach probability for the other player."""
        return self.reach_probs.get(player.other(), 0.0) * self.player_reach(Game.Player.CHANCE)
    def reach(self) -> float:
        """Return the reach probability for the current node."""
        return self.player_reach(Game.Player.P1) * self.player_reach(Game.Player.P2) * self.player_reach(Game.Player.CHANCE)
    # u* (best response value) is just what is there new best strategy after making a move
    def __le__(self, other):
        current = other
        while current is not None:
            if current is self:
                return True
            current = current.parent
        return False
    def __lt__(self, other): return other is not self and self.__le__(other)
    def __ge__(self, other): return not self < other
    def __gt__(self, other): return not self <= other
    def __len__(self): return 1 + sum(len(child) for child in self.children.values())
class Info[A]:
    def __init__(self, first_h: History):
        assert first_h.status != History.Status.TERMINAL and first_h.status != History.Status.NEW
        first_h.info = self
        self.parent = None if first_h.parent is None else first_h.parent
        self.known_histories = [first_h]
        self.player: Game.Player = first_h.player()
        self.observations: list[Game.Observation] = []  # TODO: I think this should contain the observations for each player
        self._populate_observations(first_h)
        self.reach_probs = first_h.reach_probs.copy()
        # FIXME: idk if this payoff is preforming correctly (payoff for whom)
        self.visited = False
        self.policy = Policy[A]({a: first_h.children[a].payoff() for a in first_h.children}, (1 if self.player == Game.Player.P1 else -1))

    def __repr__(self):
        return f"{self.sequence()=}, {self.player=}"

    def _populate_observations(self, h: History):
        self.observations.append(h.observations[self.player])
        current = h.parent
        while current is not None and current.player() != self.player:
            self.observations.append(current.observations[self.player])
            current = current.parent

    def filter(self, observation_history, player: Game.Player) -> Self:
        # Ensure observation_history is a tuple for consistent comparison and immutability.
        if not isinstance(observation_history, tuple):
            observation_history = tuple(observation_history)
        my_obs_hist = tuple(self.known_histories[0].history(player))
        comp_len = min(len(my_obs_hist), len(observation_history))
        # print(f"{my_obs_hist=}, {observation_history=}, {comp_len=}")
        if my_obs_hist == observation_history:
            return self
        elif my_obs_hist[:comp_len] != observation_history[:comp_len]:
            return None

        for child in self.children():
            x = child.filter(observation_history, player)
            if x is not None:
                return x
        return None

    def add_history(self, h: History) -> Self:
        assert h.status != History.Status.TERMINAL and h.status != History.Status.NEW
        for p in self.reach_probs:
            self.reach_probs[p] += h.reach_probs[p]
        self.known_histories.append(h)
        h.info = self
        return self

    def sample_history(self) -> History:
        threshold = random.random()
        for h in self.known_histories:
            threshold -= h.reach_probs[self.player]/self.reach_probs[self.player]
            if threshold <= 0:
                return h
        raise ValueError("No history was sampled.")
        return random.choice(self.known_histories)

    def gift(self) -> float:  # g(J) and ˆg(J)
        agg = 0
        for child in self.children():
            agg += child.gift()
            agg += max(0.0, child.expectation()-self.expectation())  # this is rediculously inefficient
        return agg

    def expectation(self) -> float:
        return self.policy.expectation()

    def play(self, action: A) -> set[History]: # (I, a)
        agg = set()
        for h in self.known_histories:
            if action in h.children:
                agg.add(h.children[action])
        return agg

    def sequence(self) -> tuple[Game.Observation, ...]:
        return self.known_histories[0].history(self.player)

    def children(self) -> set[Self]:
        agg = set()
        for h in self.known_histories:
            for child in h.children.values():
                if child.info is not None:
                    agg.add(child.info)
        return agg

    # π(s)
    def player_reach(self, player: Game.Player) -> float:
        """Return the reach probability for the given player."""
        return self.reach_probs[player]
    def other_player_reach(self, player: Game.Player) -> float:
        """Return the reach probability for the other player."""
        return self.reach_probs[player.other()] * self.player_reach(Game.Player.CHANCE)
    def reach(self) -> float:
        """Return the reach probability for the current node."""
        return self.player_reach(Game.Player.P1) * self.player_reach(Game.Player.P2) * self.player_reach(Game.Player.CHANCE)

    @staticmethod
    def create_subgame(j0: set['Info'], player) -> 'Info':
        other_player = ~player
        net_reach = sum(J.player_reach(other_player) for J in J0)
        m = len(J0)

        h = History.first_node()
        h.status = History.Status.EXPANDED
        h.observations[player] = Game.Observation("SUBGAME ROOT")
        h.observations[other_player] =  Game.Observation("SUBGAME ROOT")
        i = Info(h)
        i.player = other_player
        player: Game.Player = next(iter(j0)).player
        for j in j0:
            hJ = History(Game([]), h)
            hJ.info = j
            hJ.parent = j.known_histories[0].parent  # To replicate sequence stuff
            hJ.status = History.Status.EXPANDED
            hJ.children = {c: c for c in j.known_histories}
            j.policy = Policy[History]({hJ: hJ.player_reach(other_player) for hJ in j.known_histories}, multiplier=0)
            j.known_histories = [hJ]
            h.children[j] = hJ
        for J in J0:
            prior_probs[J] = 1 / 2 * (J.player_reach(other_player) / net_reach + 1 / m)
        i.policy = Policy[Self]({j: prior_probs[j] for j in j0}, (1 if player == Game.Player.P1 else -1))
        return i

    # maybe define closure here (as all infosets below this point)
    def __le__(self, other):
        current = other
        while current is not None:
            if current is self:
                return True
            current = current.parent
        return False
    def __lt__(self, other): return other is not self and self.__le__(other)
    def __ge__(self, other): return not self < other
    def __gt__(self, other): return not self <= other
    def __len__(self):return 1 + sum(len(child) for child in self.children())
    def __hash__(self): return hash((tuple(self.sequence()), self.player))
    def __eq__(self, other): return self.sequence() == other.sequence() and self.player == other.player
SEQUENCE = list[Game.Observation]

# Margin: M(x', J) := u*(x' | J) - u*(x | J)  -> how much this strategy improves at a given opp. infoset
# MaxMargin: max_x' [min_J M(x', J)] -> improve the worst margin
    # Gadget: chose J, immediately lose expected util, sample h in J, play game
# Resolve: max_x' 1/|J0| * sum(max(0, M(x', J))
    # Gadget: sample h, opp choose to play or exit with alt value
def move(hist: SEQUENCE) -> Game.Action:
    global start_time
    construct_subgame(hist)
    # assert game_tree is root_h.info
    start_time = time.time()

    # Gt-CFR
    for _ in range(EXPANDERS):
        threading.Thread(target=run_expander_thread, args=()).start()
    run_solver_thread()  #  this can block this thread

    p = game_tree.policy
    p_max = max(r.p_exploit(ResolveActions.ENTER) for r in resolvers.values())
    if p_max > 0:  # If you never doing resolve
        return p.purified()  # MAXMARGIN
    return p.exploit()  # RESOLVE

def construct_subgame(hist: SEQUENCE):
    global positions, J0, game_tree, resolvers, prior_probs, subgame_root, game_tree
    resolvers.clear()
    prior_probs.clear()
    positions = list(filter(lambda p: p.consistent(hist), positions))  # TODO: feel like this should be a generator
    # print(f"{positions=}")
    game_tree = game_tree.filter(hist, Game.Player.P1)
    assert game_tree is not None
    player: Game.Player = game_tree.player
    other_player: Game.Player = ~player
    # print(f"{player=}, {other_player=}")
    J0 = info_closure(game_tree)
    # print(f"{J0=}")
    for J in J0:
        alt = J.expectation() - J.gift()  # sounds like there is a good why to implement this
        resolvers[J] = Policy({ResolveActions.SKIP: alt, ResolveActions.ENTER: 0}, (1 if other_player == Game.Player.P1 else -1))
    # print(f"{resolvers=}")
    # something about possible opponent infosets
    while len(game_tree) < min(MIN_INFO_SIZE, len(positions)):  # FIXME:
        s = random.choice(positions)
        # add s to my infoset
        J = Info(s) # assume opponent has perfect knowledge and create single object info class for this
        J.player = Game.Player.CHANCE
        J0.add(J)
        alt = min(expectation, s.evaluate())
        resolvers[J] = Policy[ResolveActions]({ResolveActions.SKIP: alt, ResolveActions.ENTER: 0}, s.player())
    m = len(J0)
    # print(f"{m=}")
    net_reach = sum(J.player_reach(other_player) for J in J0)
    for J in J0:
        prior_probs[J] = 1/2 * (J.player_reach(other_player)/net_reach + 1/m)
        # Resolve now max_x' prior_prob * Margin(x, x')
    # create new root node ø where p2 selects J, reaching h_J. h_J is a chance node that samples h from the internals of J
    subgame_root = Info.create_subgame(J0, player)

def run_solver_thread():
    global J0
    while time.time() - start_time < SOLVE_TIME:
        cfr_iteration(Game.Player.P1)
        cfr_iteration(Game.Player.P2)

        for J in J0:  # For each infoset off the augmented game, update whether it would choose to play (p_max)
            r = resolvers[J]
            r.expectations[ResolveActions.ENTER] = J.expectation()
            r.update()

        p_max = max(r.p_exploit(ResolveActions.ENTER) for r in resolvers.values())
        for J in subgame_root.children():  # Maybe subgame.children keys is infos
            assert isinstance(J, Info)
            p_maxmargin = subgame_root.policy.p_exploit(J)
            p_resolve = resolvers[J].p_exploit(ResolveActions.ENTER)
            prior_p = prior_probs[J]
            reach_prob = p_max * prior_p * p_resolve + (1-p_max) * p_maxmargin
            subgame_root.policy.acc_regret[J] = reach_prob
        subgame_root.policy.net_regret = sum(subgame_root.policy.acc_regret.values())

def cfr_iteration(player: Game.Player):
    for h in subgame_root.known_histories:  # Should only be the one
        make_utilities(player, h)

    if player == Game.Player.P1:  # if not the J0 player
        for J in subgame_root.children():
            # print(f"{J=}, {J.policy=}, {subgame_root.policy.expectations=}")
            subgame_root.policy.expectations[J] += resolvers[J].expectations[ResolveActions.ENTER] * J.policy.multiplier # maybe this should be -= (depends
    def aux(info: Info):
        if not info.visited: return
        for child in info.children():
            aux(child)
        policy = info.policy
        if info.player == player:
            policy.update()
            info.visited = False
        policy.expectations = {a: 0 for a in policy.expectations}  # What is the difference (practically) between counterfactuals and expectations
    aux(subgame_root)

def make_utilities(i: Game.Player, h: History, reach_prob=1) -> REWARD:
    # print(f"{h=}, {reach_prob=}, {h.status=}")
    if h.status == History.Status.NEW:
        h.status = History.Status.VISITED
    if not h.expanded() or h.terminal():
        # print(f"{h.payoff()=}")
        return h.payoff()
    else:
        h.info.visited = True
        info = h.info
        ev = 0
        active = h.player()
        # print(f"{h.children=}, {h.info.policy=}")
        for action in h.children:
            action_prob = info.policy.p_exploit(action)
            # print(f"{action=}, child={h.children[action]},{action_prob=}")
            if active == i or action_prob > 0:
                child = h.children[action]
                child.reach_probs[active] = h.reach_probs[active] * action_prob
                child_expectation = make_utilities(i, child, reach_prob=reach_prob*action_prob)
                # print(f'{h=}, {action=}, {action_prob=}, {h.children=}, {info.policy=}')
                ev += action_prob * child_expectation
                info.policy.expectations[action] += child_expectation * reach_prob * action_prob#child.other_player_reach(i)
        return ev

def run_expander_thread():
    while time.time() - start_time < SOLVE_TIME:
        expansion_step(Game.Player.P1, game_tree.sample_history())
        expansion_step(Game.Player.P2, game_tree.sample_history())

def expansion_step(player: Game.Player, here: History):
    while here.expanded():
        policy = here.info.policy
        action = policy.explore() if here.player == player else policy.exploit()
        policy.expansions[action] += 1
        here = here.children[action]
    if here.new() or here.terminal():
        # print(f"Expansion deadends: {here}, {here.status}")
        return
    # Expand new node -> reserve it from threads
    here.expand()

# might just be the info of all parent nodes, if parent is None do weird
def info_closure(node: Info[Game.Action], k=2) -> set[Info]:  # TODO: make this support > k=2 knowledge
    agg = dict()
    for h in node.known_histories:
        prev_info = h.info
        seq = h.history(~node.player)
        if seq in agg:
            agg[seq].add_history(h)
        else:
            i = Info(h)
            i.player = Game.Player.CHANCE
            agg[seq] = i
        h.info = prev_info
    assert game_tree not in agg.values()
    assert root_h.info is game_tree
    return set(agg.values())

# Maintained Variables
expectation = 0  # v*
positions = set()  # P
info_sets: dict[tuple[Game.Observation, ...], Info] = dict()  # I of h
resolvers: dict[Info, Policy[ResolveActions]] = dict()  # R_J
prior_probs: dict[Info, float] = dict() # \alpha
start_time = 0  # T
root_game: Game = Game([])
root_h = History(root_game, None)
root_h.expand()
game_tree: Info = root_h.info  # gamma
subgame_root: Info = None  # ø
J0: set[Info] = None
print(f"{root_h=}, {game_tree=}, {game_tree.policy=}")
move([Game.Observation(None)])

print(game_tree.policy.exploit_policy())
print(game_tree.policy)
for child in game_tree.children():
    print(child, child.policy, child.policy.exploit_policy())
    for h in child.known_histories:
        print(h.observations)
        for a in h.children:
            c2 = h.children[a]
            print(a, c2.payoff())

# move([])

def unit_tests():
    # winners
    assert Game([Game.Action.ROCK, Game.Action.SCISSORS]).evaluate() > 0  # ROCK beats paper
    assert Game([Game.Action.PAPER, Game.Action.ROCK]).evaluate() > 0  # Paper beats rock
    assert Game([Game.Action.SCISSORS, Game.Action.PAPER]).evaluate() > 0  # Paper beats rock
    # losers
    assert Game([Game.Action.ROCK, Game.Action.PAPER]).evaluate() < 0  # ROCK beats paper
    assert Game([Game.Action.PAPER, Game.Action.SCISSORS]).evaluate() < 0  # Paper beats rock
    assert Game([Game.Action.SCISSORS, Game.Action.ROCK]).evaluate() < 0  # Paper beats rock
unit_tests()


# TODO: The next challenge problem is to start in the second state and try to recover the first state