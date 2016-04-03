use std::collections::{HashMap, HashSet};
use std;
use rand;
use rand::Rng;

pub type SquareId = (char, char);
pub type SquareValue = u32;
pub type StartValue = (SquareId, SquareValue);
pub type StartState = Vec<StartValue>;

type SquareValues = HashSet<SquareValue>;
type PeerSet = HashSet<SquareId>;
type Unit = [SquareId; 9];

pub struct Generator {
    config : Config,
    string_handler : StringStartStateHandler
}

impl Generator {
    pub fn new() -> Generator {
        Generator {
            config : Config::new(),
            string_handler : StringStartStateHandler::new()
        }
    }

    pub fn generate(&self, n : usize) -> StartState {
        loop {
            match State::new(&self.config).generate(n) {
                Ok(state) => return state,
                _ => ()
            }
        }
    }

    pub fn generate_str(&self, n : usize) -> String {
        self.string_handler.generate(&self.config, self.generate(n))
    }
}

pub struct Solver {
    config : Config,
    string_handler : StringStartStateHandler
}

impl Solver {
    pub fn new() -> Solver {
        Solver {
            config : Config::new(),
            string_handler : StringStartStateHandler::new()
        }
    }

    pub fn solve(&self, start_state : StartState) -> Result<State, String> {
        let mut state = State::new(&self.config);
        if ! state.solve(start_state) {
            return Err("Failed solving puzzle".to_string());
        }
        Ok(state)
    }

    pub fn solve_str(&self, grid : &str) -> Result<State, String> {
        let grid = match self.string_handler.parse(&self.config, grid.to_string()) {
            Ok(grid) => grid,
            Err(err) => return Err(err)
        };
        self.solve(grid)
    }
}

#[derive(Clone, Debug)]
pub struct State<'a> {
    config : &'a Config,
    values : HashMap<SquareId, SquareValues>
}

impl<'a> State<'a> {

    pub fn new(config : &'a Config) -> State<'a> {
        State {
            config : config,
            values : config.values.clone()
        }
    }

    pub fn solve(&mut self, state : StartState) -> bool {
        if ! self.apply_start_state(state) {
            return false;
        }
        self.search()
    }

    pub fn generate(&mut self, n : usize) -> Result<StartState, ()> {
        match self.randomize(n) {
            Ok(_) => Ok(self.encode()),
            Err(_) => Err(())
        }
    }

    fn encode(&self) -> StartState {
        self.values.iter()
                   .filter(|&(_, vs)| vs.len() == 1)
                   .map(|(s, vs)| (s.clone(), vs.iter().nth(0).unwrap().clone()))
                   .collect()
    }

    fn randomize(&mut self, n : usize) -> Result<(), ()> {
        let mut squares = self.config.squares.clone();
        let mut rng = rand::thread_rng();
        rng.shuffle(&mut squares);
        for s in &squares {
            let vals : Vec<u32> = self.values.get(s).unwrap().iter().cloned().collect();
            if ! self.assign(&s, &rng.choose(&vals).unwrap().clone()) {
                return Err(());
            }
            let d_values = self.values.iter()
                                      .filter(|&(_, vs)| vs.len() == 1)
                                      .flat_map(|(_, vs)| vs.iter())
                                      .cloned()
                                      .collect::<Vec<u32>>();
            let d_uniq_values = d_values.iter().cloned().collect::<HashSet<u32>>();
            if d_values.len() >= n && d_uniq_values.len() >= 8 {
                return Ok(());
            }
        }
        Err(())
    }

    fn apply_start_state(&mut self, state : StartState) -> bool {
        for (s, v) in state {
            if v != 0 {
                if ! self.assign(&s, &v) {
                    return false;
                }
            }
        }
        true
    }

    fn assign(&mut self, square : &SquareId, value : &SquareValue) -> bool {
        let mut remove_values = self.values.get(square).unwrap().clone();
        remove_values.remove(value);
        remove_values.iter().all(|d2| self.eliminate(square, d2))
    }

    fn eliminate(&mut self, square : &SquareId, value: &SquareValue) -> bool {
        let vs_len = {
            let mut vs = self.values.get_mut(square).unwrap();
            if ! vs.contains(value) {
                return true; // already eliminated
            }
            vs.remove(value);
            vs.len()
        };
        // (1) If a square s is reduced to one value d2, then eliminate d2 from the peers.
        if vs_len == 0 {
            return false; // contradiction: last value removed
        } else if vs_len == 1 {
            let d2 = self.values.get(square).unwrap().iter().nth(0).unwrap().clone();
            if !self.config.peers.get(square).unwrap().iter().all(|s2| self.eliminate(s2, &d2)) {
                return false;
            }
        }
        // (2) If a unit u is reduced to only one place for a value d, then put it there.
        for u in self.config.units.get(square).unwrap() {
            let places : Vec<SquareId> = u.iter().filter(|s| self.values.get(s).unwrap().contains(value)).cloned().collect();
            if places.len() == 0 {
                return false;
            } else if places.len() == 1 {
                if ! self.assign(&places[0], value) {
                    return false;
                }
            }
        }
        true
    }

    fn sort_values(&self, square : &SquareId) -> Vec<SquareValue> {
        let vs = self.values.get(square).unwrap();
        let mut v_n = Vec::with_capacity(vs.len());
        for v in vs {
            v_n.push((v.clone(), self.values.iter().filter(|&(_, sv)| sv.contains(v)).count()));
        }
        v_n.sort_by(|a, b| a.1.cmp(&b.1));
        v_n.iter().map(|&(v, _)| v).collect()
    }

    fn search(&mut self) -> bool {
        if self.is_solved() {
            return true
        }
        let square = self.config.squares.iter().filter(|s| self.values.get(s).unwrap().len() > 1)
                                                .min_by_key(|s| self.values.get(s).unwrap().len())
                                                .unwrap();
        for d in self.sort_values(&square).clone() {
            let mut child_state = self.clone();
            if child_state.internal_solve(&square, &d) {
                self.values = child_state.values;
                return true;
            }
        }
        false
    }

    fn internal_solve(&mut self, square : &SquareId, value : &SquareValue) -> bool {
        if ! self.assign(square, value) {
            return false;
        }
        self.search()
    }

    pub fn is_solved(&self) -> bool {
        self.config.squares.iter().all(|s| self.values.get(s).unwrap().len() == 1)
    }
}

impl<'a> std::fmt::Display for State<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        fn as_string(values : &SquareValues) -> String {
            let mut s = String::new();
            for v in values {
                s.push(std::char::from_digit(*v, 10).unwrap());
            }
            s
        }

        for (i, s) in self.config.squares.iter().enumerate() {
            if i == 0 {
                write!(f, "").unwrap();
            } else if (i%27) == 0 {
                write!(f, "\n---------+---------+---------\n").unwrap();
            } else if (i%9) == 0 {
                write!(f, "\n").unwrap();
            } else if (i%3) == 0 {
                write!(f, "|").unwrap();
            }
            write!(f, "{: ^3}", as_string(self.values.get(s).unwrap())).unwrap();
        }
        write!(f, "\n")
    }
}

#[derive(Debug)]
pub struct Config {
    squares : Vec<SquareId>,
    units : HashMap<SquareId, Vec<Unit>>,
    peers : HashMap<SquareId, PeerSet>,
    digits : SquareValues,
    values : HashMap<SquareId, SquareValues>
}

impl Config {

    pub fn new() -> Config {
        let squares : [SquareId; 81] =
            [('A', '1'), ('A', '2'), ('A', '3'), ('A', '4'), ('A', '5'), ('A', '6'), ('A', '7'), ('A', '8'), ('A', '9'),
            ('B', '1'), ('B', '2'), ('B', '3'), ('B', '4'), ('B', '5'), ('B', '6'), ('B', '7'), ('B', '8'), ('B', '9'),
            ('C', '1'), ('C', '2'), ('C', '3'), ('C', '4'), ('C', '5'), ('C', '6'), ('C', '7'), ('C', '8'), ('C', '9'),
            ('D', '1'), ('D', '2'), ('D', '3'), ('D', '4'), ('D', '5'), ('D', '6'), ('D', '7'), ('D', '8'), ('D', '9'),
            ('E', '1'), ('E', '2'), ('E', '3'), ('E', '4'), ('E', '5'), ('E', '6'), ('E', '7'), ('E', '8'), ('E', '9'),
            ('F', '1'), ('F', '2'), ('F', '3'), ('F', '4'), ('F', '5'), ('F', '6'), ('F', '7'), ('F', '8'), ('F', '9'),
            ('G', '1'), ('G', '2'), ('G', '3'), ('G', '4'), ('G', '5'), ('G', '6'), ('G', '7'), ('G', '8'), ('G', '9'),
            ('H', '1'), ('H', '2'), ('H', '3'), ('H', '4'), ('H', '5'), ('H', '6'), ('H', '7'), ('H', '8'), ('H', '9'),
            ('I', '1'), ('I', '2'), ('I', '3'), ('I', '4'), ('I', '5'), ('I', '6'), ('I', '7'), ('I', '8'), ('I', '9')];

        let unitlist : [Unit; 27] =
            [[('A', '1'), ('A', '2'), ('A', '3'), ('A', '4'), ('A', '5'), ('A', '6'), ('A', '7'), ('A', '8'), ('A', '9')],
            [('B', '1'), ('B', '2'), ('B', '3'), ('B', '4'), ('B', '5'), ('B', '6'), ('B', '7'), ('B', '8'), ('B', '9')],
            [('C', '1'), ('C', '2'), ('C', '3'), ('C', '4'), ('C', '5'), ('C', '6'), ('C', '7'), ('C', '8'), ('C', '9')],
            [('D', '1'), ('D', '2'), ('D', '3'), ('D', '4'), ('D', '5'), ('D', '6'), ('D', '7'), ('D', '8'), ('D', '9')],
            [('E', '1'), ('E', '2'), ('E', '3'), ('E', '4'), ('E', '5'), ('E', '6'), ('E', '7'), ('E', '8'), ('E', '9')],
            [('F', '1'), ('F', '2'), ('F', '3'), ('F', '4'), ('F', '5'), ('F', '6'), ('F', '7'), ('F', '8'), ('F', '9')],
            [('G', '1'), ('G', '2'), ('G', '3'), ('G', '4'), ('G', '5'), ('G', '6'), ('G', '7'), ('G', '8'), ('G', '9')],
            [('H', '1'), ('H', '2'), ('H', '3'), ('H', '4'), ('H', '5'), ('H', '6'), ('H', '7'), ('H', '8'), ('H', '9')],
            [('I', '1'), ('I', '2'), ('I', '3'), ('I', '4'), ('I', '5'), ('I', '6'), ('I', '7'), ('I', '8'), ('I', '9')],
            [('A', '1'), ('B', '1'), ('C', '1'), ('D', '1'), ('E', '1'), ('F', '1'), ('G', '1'), ('H', '1'), ('I', '1')],
            [('A', '2'), ('B', '2'), ('C', '2'), ('D', '2'), ('E', '2'), ('F', '2'), ('G', '2'), ('H', '2'), ('I', '2')],
            [('A', '3'), ('B', '3'), ('C', '3'), ('D', '3'), ('E', '3'), ('F', '3'), ('G', '3'), ('H', '3'), ('I', '3')],
            [('A', '4'), ('B', '4'), ('C', '4'), ('D', '4'), ('E', '4'), ('F', '4'), ('G', '4'), ('H', '4'), ('I', '4')],
            [('A', '5'), ('B', '5'), ('C', '5'), ('D', '5'), ('E', '5'), ('F', '5'), ('G', '5'), ('H', '5'), ('I', '5')],
            [('A', '6'), ('B', '6'), ('C', '6'), ('D', '6'), ('E', '6'), ('F', '6'), ('G', '6'), ('H', '6'), ('I', '6')],
            [('A', '7'), ('B', '7'), ('C', '7'), ('D', '7'), ('E', '7'), ('F', '7'), ('G', '7'), ('H', '7'), ('I', '7')],
            [('A', '8'), ('B', '8'), ('C', '8'), ('D', '8'), ('E', '8'), ('F', '8'), ('G', '8'), ('H', '8'), ('I', '8')],
            [('A', '9'), ('B', '9'), ('C', '9'), ('D', '9'), ('E', '9'), ('F', '9'), ('G', '9'), ('H', '9'), ('I', '9')],
            [('A', '1'), ('A', '2'), ('A', '3'), ('B', '1'), ('B', '2'), ('B', '3'), ('C', '1'), ('C', '2'), ('C', '3')],
            [('A', '4'), ('A', '5'), ('A', '6'), ('B', '4'), ('B', '5'), ('B', '6'), ('C', '4'), ('C', '5'), ('C', '6')],
            [('A', '7'), ('A', '8'), ('A', '9'), ('B', '7'), ('B', '8'), ('B', '9'), ('C', '7'), ('C', '8'), ('C', '9')],
            [('D', '1'), ('D', '2'), ('D', '3'), ('E', '1'), ('E', '2'), ('E', '3'), ('F', '1'), ('F', '2'), ('F', '3')],
            [('D', '4'), ('D', '5'), ('D', '6'), ('E', '4'), ('E', '5'), ('E', '6'), ('F', '4'), ('F', '5'), ('F', '6')],
            [('D', '7'), ('D', '8'), ('D', '9'), ('E', '7'), ('E', '8'), ('E', '9'), ('F', '7'), ('F', '8'), ('F', '9')],
            [('G', '1'), ('G', '2'), ('G', '3'), ('H', '1'), ('H', '2'), ('H', '3'), ('I', '1'), ('I', '2'), ('I', '3')],
            [('G', '4'), ('G', '5'), ('G', '6'), ('H', '4'), ('H', '5'), ('H', '6'), ('I', '4'), ('I', '5'), ('I', '6')],
            [('G', '7'), ('G', '8'), ('G', '9'), ('H', '7'), ('H', '8'), ('H', '9'), ('I', '7'), ('I', '8'), ('I', '9')]];

        let units = squares.iter()
                            .map(|s| (s.clone(), unitlist.iter()
                                                         .filter(|u| u.contains(s))
                                                         .cloned()
                                                         .collect::<Vec<Unit>>()))
                            .collect::<HashMap<SquareId, Vec<Unit>>>();

        let digits : SquareValues = [1, 2, 3, 4, 5, 6, 7, 8, 9].iter().cloned().collect::<SquareValues>();

        Config {
            squares : squares.iter().cloned().collect(),
            units : units.clone(),
            peers : squares.iter()
                            .map(|s| (s.clone(), units.get(s).unwrap().iter()
                                                                      .flat_map(|u| u.iter()
                                                                                     .filter(|s2| s2 != &s)
                                                                                     .cloned())
                                                                      .collect::<PeerSet>()))
                            .collect::<HashMap<SquareId, PeerSet>>(),
            digits : digits.clone(),
            values : squares.iter()
                            .map(|s| (s.clone(), digits.clone()))
                            .collect::<HashMap<SquareId, SquareValues>>()
        }
    }
}

pub struct StringStartStateHandler;

pub trait StartStateHandler<T> {
    fn parse(&self, config : &Config, input : T) -> Result<StartState, String>;
    fn generate(&self, config : &Config, state : StartState) -> T;
}

impl StringStartStateHandler {

    pub fn new() -> StringStartStateHandler {
        StringStartStateHandler
    }

}

impl StartStateHandler<String> for StringStartStateHandler {

    fn parse(&self, config: &Config, grid : String) -> Result<StartState, String> {
        if grid.len() != 81 {
            return Err("Incorrect length".to_string());
        }
        let mut grid_chars : [u32; 81] = [0 ; 81];
        for (i,v) in grid.as_bytes().iter().enumerate() {
            match (*v as char).to_digit(10) {
                Some(v32) => {
                    if config.digits.contains(&v32) {
                        grid_chars[i] = v32;
                    }
                },
                None => ()
            };
        }
        Ok(config.squares.iter().cloned().zip(grid_chars.iter().cloned()).collect())
    }

    fn generate(&self, config : &Config, state : StartState) -> String {
        let mut chars = ['.'; 81];
        for (square, value) in state {
            match config.squares.iter().position(|&s| s == square) {
                Some(index) => chars[index] = std::char::from_digit(value, 10).unwrap(),
                _ => ()
            };
        }
        chars.iter().cloned().collect()
    }

}