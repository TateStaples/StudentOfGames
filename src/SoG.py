from abc import ABC, abstractmethod
from typing import List, Tuple, Optional
import numpy as np
import random

class Game(ABC):
    @abstractmethod
    def public_state(self):
        pass

    @abstractmethod
    def is_over(self):
        pass

    @abstractmethod
    def outcome(self):
        pass

    @abstractmethod
    def step(self, action):
        pass


class Policy(ABC):
    @abstractmethod
    def eval(self, belief):
        pass


class Node:
    def __init__(self, game, ranges, value_prior, solved):
        self.player = game.active_player()
        self.public_state = game.public_state()
        self.actions = game.actions()
        self.state_action_visits = {}
        self.state_action_quality = {}

    def action_probability(self, action_id):
        for state in self.private_states:
            self.state_action_visits[state][action_id] += 1
        return self.cfr_policy()[action_id]

    def visits(self):
        return sum(self.action_counts)

    def update_range(self, state_id, action_id):
        new_range = self.private_ranges.copy()
        for i in range(2):
            for j in range(S):
                new_range[i][j] *= self.action_probability(action_id)
        return new_range

    def belief(self):
        return self.public_state, self.private_ranges


class GtCfr:
    def __init__(self, game, belief, capacity, cfg, prior: Policy):
        self.root = 0  # FIXME
        self.starting_game = game
        self.prior = prior
        self.cfg = cfg
        self.reset(belief, None)

    def reset(self, belief, game):
        self.nodes.clear()
        self.nodes.append(Node(game, belief[1], [1.0 for _ in range(N)], False))
        self.starting_game = game

    def exploit(self, game):
        value, policy = self.search(game)
        action = sample_policy(policy)
        return action

    def continual_resolving(self, new_state):
        pass  # to be implemented

    def match_child(self, node_id, action_id, state):
        pass  # to be implemented

    def gt_cfr(self, node_id, expansions, update_per):
        node = self.node(node_id)
        value = 0.0
        for _ in range(expansions // update_per):
            value = self.cfr(node_id)
            world_state = Game.sample_state(node.public_state)
            for _ in range(update_per):
                self.grow(node_id, world_state)
        return value, node.cfr_policy()

    def cfr(self, node_id):
        node = self.mut_node(node_id)
        active_policy = node.cfr_policy()
        if node.is_visited():
            node_value = 0.0
            for action_id in range(N):
                action_prob = active_policy[action_id]
                action_value = self.action_value(node, action_id)
                for a in range(N):
                    if a == action_id:
                        node.action_quality[a] += action_value
                    else:
                        node.action_quality[a] -= action_value * action_prob
                    node_value += action_value * action_prob
            node.action_quality = [x if x > 0 else 0 for x in node.action_quality]
            return node_value
        elif node.solved:
            return self.terminal_value(node_id)
        else:
            random_number = random.uniform(0, 1)
            belief = node.belief()
            v, p = self.prior.eval(belief)
            return v

    def action_value(self, node, action_id):
        new_range = node.update_range(action_id)
        action_value = 0.0
        for c in node.action_outcomes[action_id]:
            if c is not None:
                child_node = self.mut_node(c)
                child_node.private_ranges = new_range
                value = self.cfr(c)
                child_prob = child_node.visits() / node.action_counts[action_id]
                action_value += value * child_prob
            else:
                break
        return action_value

    def terminal_value(self, node_id):
        pass  # to be implemented

    def grow(self, node_id, world_state):
        action_id = 0
        while True:
            node = self.mut_node(node_id)
            if node.is_visited():
                action_id = self.grow_step(node)
                node.action_counts[action_id] += 1.0
                action = action_id
                world_state.step(action)
                state = world_state.public_state()
                node_id = self.match_child(node_id, action_id, state)
            elif node.solved:
                return
            else:
                self.visit(node_id, action_id, world_state)
                return

    def grow_step(self, parent):
        parent = self.node(parent)
        return max(range(N), key=lambda action: self.exploit_value(parent, action) * 0.5 + self.explore_value(parent, action) * 0.5)

    def exploit_value(self, parent, action_id):
        parent = self.node(parent)
        return parent.action_probability(action_id)

    def explore_value(self, parent, action_id):
        parent = self.node(parent)
        action_visits = parent.action_counts[action_id]
        if self.cfg.exploration == 'Uct':
            visits = (self.cfg.c * np.log(action_visits)) ** 0.5
            return visits / action_visits ** 0.5
        elif self.cfg.exploration == 'PolynomialUct':
            visits = parent.visits() ** 0.5
            return self.cfg.c * parent.action_probability(action_id) * visits / (1.0 + action_visits)

    def visit(self, parent_id, action, new_world_state):
        new_node = Node(new_world_state, True, None, None)  # to be implemented
        new_node_id = len(self.nodes)
        self.nodes.append(new_node)
        for i in range(C):
            if self.node(parent_id).action_outcomes[action][i] is None:
                self.node(parent_id).action_outcomes[action][i] = new_node_id
                break