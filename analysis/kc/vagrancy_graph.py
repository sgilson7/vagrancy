"""
vagrancy_graph.py -- the knowledge-component graph of Vagrancy, as a definition
module for the kit in Gilson, "Building and Analysing a Knowledge-Component
Graph for a Course Unit" (2026): build_graph.py and run_algorithms.py run on it
unchanged.

The graph itself is data/kc_graph.json (structure) and data/copy.en.json under
kc and kc_edge (the words), so that the game's tutorial and this analysis read
one definition. This module only reshapes them into the template's tuples.

Instances are what the game puts in front of a player: the fifteen drills of
the practice yard (drill/<step>) and the 33 fights of the road (fight/<id>),
each tagged with the components it requires, from the prompt (the drill's text,
the opponent's "does") and the solution (the opponent's "try") together.

Run from the kit's directory:
    PYTHONPATH=<this dir> python3 build_graph.py vagrancy_graph <out>
    PYTHONPATH=<this dir> python3 run_algorithms.py vagrancy_graph <out>
"""
import json, os

_ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
_G = json.load(open(os.path.join(_ROOT, "data", "kc_graph.json")))
_C = json.load(open(os.path.join(_ROOT, "data", "copy.en.json")))


def _lower(s):
    s = s.rstrip(".")
    return s[0].lower() + s[1:]


NODES = [
    (n["id"], n["group"], _C["kc"][n["id"]]["name"], _lower(_C["kc"][n["id"]]["when"]),
     _lower(_C["kc"][n["id"]]["then"]), n["app"], n["resp"], n["verbal"], n["rationale"], n["strategy"])
    for n in _G["nodes"]
]

INSTANCES = dict(_G["instances"])

# With VAGRANCY_WITH_TUTORIAL=1, the tutorial's missions are items too: each
# requires what it teaches and the start of each edge it builds on. Running
# the kit both ways shows what the tutorial adds to the game's practice.
if os.environ.get("VAGRANCY_WITH_TUTORIAL") == "1":
    for m in json.load(open(os.path.join(_ROOT, "data", "tutorial.json")))["missions"]:
        kcs = list(m["teaches"])
        for e in m.get("builds_on", []):
            src = e.split("_")[0]
            if src not in kcs:
                kcs.append(src)
        INSTANCES[f"tutorial/{m['id']}"] = kcs

EDGES = [
    (e["src"], e["dst"], e["type"], _lower(_C["kc_edge"][f"{e['src']}_{e['dst']}"]), e["process"], e["stated_in"])
    for e in _G["edges"]
]

COMPARISON_PAIRS = [(c["a"], c["b"], c["kc"], _lower(_C["kc_compare"][c["id"]]), c["status"]) for c in _G["comparisons"]]

# The composites a player is working toward, and what a player who has played
# the yard and the first fights reliably keeps.
TARGETS = ["I2"]
RETAINED_SEEDS = ["K01", "K04", "K13", "K12"]
