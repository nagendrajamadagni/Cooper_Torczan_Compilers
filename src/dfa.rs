/* Perform subset construction to convert NFA into DFA
 * Apply Hopcroft's algorithm to generate minimal DFA */

use crate::fa::{FAState, Symbol, FA};
use crate::nfa::NFA;
use bitvec::prelude::*;
use petgraph::dot::Dot;
use petgraph::graph::DiGraph;
use std::collections::hash_map::Values;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::process::Command;

#[derive(Debug)]
pub struct DFA {
    states: Vec<DFAState>,
    start_state: usize,
    accept_states: BitVec<u8>,
    alphabet: HashSet<char>,
    regex: String,
}

#[derive(Debug, Clone)]
struct DFAState {
    id: usize,
    transitions: HashMap<Symbol, usize>, // Store by reference is not a thing in Rust
}

struct LookupTable {
    state_to_set_map: HashMap<usize, usize>,
    set_to_states_map: HashMap<usize, HashSet<usize>>,
}

impl LookupTable {
    fn new() -> Self {
        LookupTable {
            state_to_set_map: HashMap::new(),
            set_to_states_map: HashMap::new(),
        }
    }

    fn insert_state_in_set(&mut self, state: usize, set: usize) {
        let prev_set = self.state_to_set_map.insert(state, set);
        match prev_set {
            None => {
                // If state was not in a previous set, insert it into the provided set
                self.set_to_states_map
                    .entry(set)
                    .or_insert_with(HashSet::new)
                    .insert(state);
            }
            Some(prev_set_key) => {
                // If state was present in a previous set, remove it from previous set and insert
                // it into new set

                if let Some(prev_set) = self.set_to_states_map.get_mut(&prev_set_key) {
                    prev_set.remove(&state);
                    if prev_set.is_empty() {
                        self.set_to_states_map.remove(&prev_set_key);
                    }
                }

                self.set_to_states_map
                    .entry(set)
                    .or_insert_with(HashSet::new)
                    .insert(state);
            }
        }
        self.set_to_states_map
            .entry(set)
            .or_insert_with(HashSet::new)
            .insert(state);
    }

    fn get_set_of_state(&self, state: &usize) -> Option<&usize> {
        self.state_to_set_map.get(state)
    }

    fn get_states_in_set(&self, set: &usize) -> Option<&HashSet<usize>> {
        self.set_to_states_map.get(set)
    }

    fn get_num_sets(&self) -> usize {
        self.set_to_states_map.len()
    }

    fn get_sets(&self) -> Values<usize, HashSet<usize>> {
        self.set_to_states_map.values()
    }
}

impl FA for DFA {
    fn show_fa(&self, filename: &str) {
        let mut graph = DiGraph::new();
        let mut node_map = std::collections::HashMap::new();

        // Add nodes
        for state in &self.states {
            let node = graph.add_node(format!("State {}", state.id));
            node_map.insert(state.id, node);
        }

        // Add edges
        for state in &self.states {
            for (symbol, target) in &state.transitions {
                let symbol_str = match symbol {
                    Symbol::Char(c) => c.to_string(),
                    Symbol::Epsilon => "𝛆".to_string(),
                };
                graph.add_edge(node_map[&state.id], node_map[&target], symbol_str);
            }
        }

        // Mark Start and Accept States

        let start_node = node_map[&self.start_state];
        graph[start_node] = format!("Start\nState {}", self.start_state);

        let accept_states: Vec<usize> = self.accept_states.iter_ones().collect();

        for accept in accept_states {
            let accept_node = node_map[&accept];
            graph[accept_node] =
                graph[accept_node].clone() + &format!("\nAccept\nState {}", accept);
        }

        let dot = Dot::new(&graph);

        // Write dot to file
        let dot_filename = format!("{}.dot", filename);
        let mut dot_file = File::create(&dot_filename).expect("Failed to create dot file");

        dot_file
            .write_all(dot.to_string().as_bytes())
            .expect("Failed to write dot file");

        Command::new("dot")
            .args(&["-Tjpg", &dot_filename, "-o", &format!("{}.jpg", filename)])
            .output()
            .expect("Failed to execute Graphviz");

        println!("DFA vizualization saved as {}.jpg", filename);
    }

    fn add_transition(&mut self, from: usize, symbol: Symbol, to: usize) {
        self.states[from].add_transition(symbol, to);
    }

    fn set_accept_state(&mut self, state_id: usize) {
        self.accept_states.set(state_id, true);
    }

    fn add_state(&mut self) -> usize {
        let state_id = self.states.len();
        let new_state: DFAState = DFAState::new(state_id);
        self.states.push(new_state);
        self.accept_states.push(false);
        return state_id;
    }

    fn get_num_states(&self) -> usize {
        self.states.len()
    }

    fn get_start_state(&self) -> usize {
        self.start_state
    }

    fn get_alphabet(&self) -> &HashSet<char> {
        &self.alphabet
    }

    fn get_acceptor_states(&self) -> &BitVec<u8> {
        &self.accept_states
    }

    fn get_regex(&self) -> &String {
        &self.regex
    }
}

impl FAState for DFAState {
    fn add_transition(&mut self, symbol: Symbol, to: usize) {
        self.transitions.insert(symbol, to);
    }
}

impl DFAState {
    fn new(id: usize) -> Self {
        DFAState {
            id,
            transitions: HashMap::new(),
        }
    }

    fn get_transitions(&self) -> &HashMap<Symbol, usize> {
        &self.transitions
    }
}

impl DFA {
    fn new() -> Self {
        DFA {
            states: Vec::new(),
            start_state: 0,
            accept_states: BitVec::new(),
            alphabet: HashSet::new(),
            regex: String::new(),
        }
    }

    fn set_regex(&mut self, regex: String) {
        self.regex = regex;
    }

    fn get_state(&self, id: usize) -> &DFAState {
        let state = self.states.get(id);
        match state {
            Some(state) => state,
            None => panic!("Invalid state index provided"),
        }
    }
}

fn get_epsilon_closure(nfa: &NFA, nfa_states: BitVec<u8>) -> BitVec<u8> {
    let num_states: usize = nfa.get_num_states();

    let mut epsilon_closure: BitVec<u8, Lsb0> = BitVec::repeat(false, num_states);

    let mut visited: BitVec<u8, Lsb0> = BitVec::repeat(false, num_states);

    let mut nfa_states: VecDeque<_> = nfa_states.iter_ones().collect();

    while !nfa_states.is_empty() {
        let state = nfa_states.pop_front();
        let state = match state {
            Some(state) => state,
            None => panic!("Trying to remove element from empty queue"),
        };
        let state = nfa.get_state(state);
        let transitions = state.get_transitions();

        let eps_transitions = transitions.get(&Symbol::Epsilon);
        match eps_transitions {
            Some(targets) => {
                for target in targets {
                    let target = *target; // Unboxing the value
                    if !visited[target] {
                        visited.set(target, true);
                        nfa_states.push_back(target);
                    }
                }
            }
            None => {}
        }
        epsilon_closure.set(state.get_id(), true); // Adding the state itself to the epsilon closure
    }
    return epsilon_closure;
}

// This function returns the set of states accessible via char c within the set q

fn delta(nfa: &NFA, q: &BitVec<u8>, c: char) -> BitVec<u8> {
    let mut result = BitVec::repeat(false, q.len());
    let nodes: Vec<usize> = q.iter_ones().collect();
    for node in nodes {
        let nfa_state = nfa.get_state(node);
        let transitions = nfa_state.get_transitions();
        let target_state_ids = transitions.get(&Symbol::Char(c));
        let target_state_ids = match target_state_ids {
            None => continue,
            Some(state_ids) => state_ids,
        };
        for state_id in target_state_ids {
            let state_id = *state_id; // Unwrapping the box
            result.set(state_id, true);
        }
    }
    return result;
}

pub fn construct_minimal_dfa(dfa: &DFA) {
    let alphabet = dfa.get_alphabet();
    let mut lookup_table = LookupTable::new();
    let states = dfa.get_acceptor_states();
    // 0 is acceptors states, 1 is non acceptor states

    for accept_state in states.iter_ones() {
        lookup_table.insert_state_in_set(accept_state, 0);
    }

    for non_accept_state in states.iter_zeros() {
        lookup_table.insert_state_in_set(non_accept_state, 1);
    }

    loop {
        let number_of_sets = lookup_table.get_num_sets(); // Get number of sets at start of
                                                          // iteration
        let sets: Vec<_> = lookup_table.get_sets().cloned().collect(); // Get list of sets

        // Try to split the sets further

        for set in sets {
            if set.len() == 1 {
                // Cannot split a set with only 1 element
                continue;
            }
            let next_set = lookup_table.get_num_sets() + 1; // The next set which will be inserted
            let member_state_id = set.iter().next();
            let member_state_id = match member_state_id {
                Some(id) => id,
                None => panic!("Trying to remove element from empty set!"),
            };

            let member_state = dfa.get_state(*member_state_id);

            let member_state_transitions = member_state.get_transitions();

            for state_id in set {
                let state = dfa.get_state(state_id);
                let state_transitions = state.get_transitions();

                for c in alphabet {
                    let state_dest = state_transitions.get(&Symbol::Char(*c)); // Get destination
                                                                               // for the state for
                                                                               // this symbol and
                                                                               // member
                    let member_dest = member_state_transitions.get(&Symbol::Char(*c));

                    match (state_dest, member_dest) {
                        (None, None) => continue, // If both don't have a transition, no splitting
                        (Some(_), None) | (None, Some(_)) => {
                            // If only one has a transition,
                            // split
                            lookup_table.insert_state_in_set(state_id, next_set);
                            continue;
                        }
                        (Some(state_dest), Some(member_dest)) => {
                            // If both have transitions,
                            // make sure both transition to
                            // same set
                            let state_dest_set = lookup_table.get_set_of_state(state_dest).unwrap();
                            let member_dest_set =
                                lookup_table.get_set_of_state(member_dest).unwrap();

                            if state_dest_set == member_dest_set {
                                continue;
                            } else {
                                // If not, split
                                lookup_table.insert_state_in_set(state_id, next_set);
                                break;
                            }
                        }
                    }
                }
            }
        }
        let new_number_of_sets = lookup_table.get_num_sets();

        if number_of_sets == new_number_of_sets {
            break;
        }
    }

    let sets = lookup_table.get_sets();

    let minimal_dfa = DFA::new();

    let start_state = dfa.get_start_state();

    for set in sets {
        println!("The set is {:?}", set);
    }
}
pub fn construct_dfa(nfa: NFA) -> DFA {
    let mut result = DFA::new(); // Create new DFA
    result.alphabet = nfa.get_alphabet().clone(); // DFA has same alphabet as NFA

    let nfa_accepts = nfa.get_acceptor_states();

    let di = result.add_state(); // Add an iniital state
    result.start_state = di;
    let n0: usize = nfa.get_start_state(); // Get n0
    let mut q_list = HashMap::new(); // Mapping from nfa state set to DFA state
    let mut work_list = VecDeque::new();

    let mut nfa_states = BitVec::repeat(false, nfa.get_num_states()); // Get the initial nfa states
    nfa_states.set(n0, true); // Add the start state to nfa states set

    let q0 = get_epsilon_closure(&nfa, nfa_states); // Get its epsilon closure
    q_list.insert(q0.clone(), di); // Add it to the mapping
    work_list.push_back(q0.clone()); // Add the first nfa states set to the work list

    let has_common = (q0 & nfa_accepts).any();

    if has_common {
        result.set_accept_state(di);
    }

    let dfa_alphabet = result.alphabet.clone();

    while !work_list.is_empty() {
        let q = work_list.pop_front();
        let q = match q {
            Some(q) => q,
            None => panic!("trying to pop empty list!"),
        };
        for c in dfa_alphabet.iter() {
            let end_states = delta(&nfa, &q, *c);
            if end_states.not_any() {
                continue;
            }
            let t = get_epsilon_closure(&nfa, end_states);

            if !q_list.contains_key(&t) {
                // check if di is as an acceptor state
                let di = result.add_state();
                q_list.insert(t.clone(), di);
                work_list.push_back(t.clone());
                let has_common = (t.clone() & nfa_accepts).any();
                if has_common {
                    result.set_accept_state(di);
                }
            }
            // add a transition from diq to dit
            let dq = q_list.get(&q);
            let dq = match dq {
                Some(dq) => dq,
                None => panic!("value not found in hash table"),
            };
            let di = q_list.get(&t);
            let di = match di {
                Some(di) => di,
                None => panic!("value not found in hash table"),
            };
            let di = *di;
            let dq = *dq; // Unwrapping the box
            result.add_transition(dq, Symbol::Char(*c), di);
        }
    }
    let regex = nfa.get_regex();
    result.set_regex(regex.to_string());
    let filename = format!("{regex}_dfa");
    result.show_fa(&filename);
    construct_minimal_dfa(&result);
    return result;
}
