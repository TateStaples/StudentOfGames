from ty

class Game:
    def __init__(self):
        self.root = PublicState()


class PublicState:
    def __init__(self):
        self.children = []
        self.range: List[float] = []
        self.info_states: List[InfoState] = []
        self.histories: List[History] = []
        self.location: str = "Leaf"
        
    def sample_history(self) -> History:
        pass
        
    def clear(self):  # Clears range and counterfactuals from previous CFR
        for info_state in self.info_states:
            info_state.reach_prob = 0.0
            info_state.values = None
        
    def expand(self):  # Expands the game tree
        self.location = "Inner"
    
class InfoState:
    def __init__(self):
        self.strategy: Dict[Action, float] = {}
        self.quality: Dict[Action, float] = []
        self.values: Dict[Action, float] = []
        self.visits: Dict[Action, float] = []
        self.policy: Dict[Action, float] = []
        self.actions: List[Action] = []
        self.children: List[InfoState] = []
        self.public_state: PublicState = None
        self.parent: InfoState = None
        self.reach_prob: float = 0.0
        
    def transition(self, observation: PrivateObservation) -> InfoState:
        pass
    
class History:
    def __init__(self):
        self.children = []
        self.public_state: PublicState = None
        self.info_state: InfoState = None
        self.inactive_state: InfoState = None
        self.actions: List[Action] = []
        self.game_state: Game = None
        self.reach_prob: float = 0.0
    
    def step(self, action: Action) -> History:
        pass
    
    def expand(self):
        pass
    
    
def cfr(root: PublicState):  # Counterfactual Regret Minimization: imperfect information game solver
    cfr_setup(root)
    # Update reach probabilities
    root_prob = sum(root.histories, key=lambda history: history.reach_prob)
    for history in root.histories:
        cfr_reach(history, histories.reach_prob/root_prob)
    # Update counterfactual values
    
    # Update regrets, qualities and strategies
def cfr_reach(history: History, probability: float):
    history.info_state.reach_prob += probability
    history.reach_prob = probability
    for action in history.actions:
        next_history = history.step(action)
        cfr_reach(next_history, probability * history.info_state.strategy[action])
def cfr_values(info_state) -> float:
    if info_state.value == 0.0:
        info_state.value = sum(info_state.children, key=lambda child: cfr_values(child)*child.reach_prob)/info_state.reach_prob
    
    
def cfr_setup(root: PublicState):  # Sets up the game tree
    root.clear()
    for child in root.children:
        setup(child)
    if root.location == "Leaf":
        (p1_values, p2_values) = evaluate(root)
        root.values 

def gt_cfr(root: PublicState, expansions: int, updates_per):  # Growing Tree Counterfactual Regret Minimization: CFR on expanding forms of the game tree
    for i in range(expansions):
        explore_n(root, updates_per)
        cfr(root)
        
        
def explore_n(node: PublicState, count: int):  # Explores a node in the game tree to a certain depth
    for _ in range(count):
        history = node.sample_history()
        grow(node, history)

def grow(node: PublicState, history: History):  # Grows the game tree from a history
    while True:
        match node.location:
            case "Inner":
                action = grow_step(history.info_state)
                history = history.step(action)
                node = history.public_state
            case "Leaf":
                node.expand()
                return
            _:
                return

def grow_step(state: InfoState):  # Grows the game tree from a history by one step
    return max(history.actions, key=lambda action: exploit_value(state, action) + exploration_value(state, action))

def exploit_value(state: InfoState, action: Action):  # Calculates the exploit value of an action
    return state.strategy[action]
    
def exploration_value(state: InfoState, action: Action):  # Calculates the exploration value of an action
    # PUCT
    return state.quality[action]/state.visits[action] + state.exploration_constant * state.policy[action] * (sum(state.visits) ** 0.5) / (1 + state.visits[action])
        

def self_play():
    pass

