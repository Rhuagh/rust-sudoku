extern crate time;
extern crate rand;

mod sudoku;

fn time_solve(solver : &sudoku::Solver, puzzle: &str) -> u64 {
    let start = time::precise_time_ns();
    solver.solve_str(puzzle).unwrap();
    time::precise_time_ns() - start
}

fn main() {
    let easy = "..3.2.6..9..3.5..1..18.64....81.29..7.......8..67.82....26.95..8..2.3..9..5.1.3..";
    let hard = "4.....8.5.3..........7......2.....6.....8.4......1.......6.3.7.5..2.....1.4......";
    let hardest = ".....6....59.....82....8....45........3........6..3.54...325..6..................";
    let solver = sudoku::Solver::new();
    let generator = sudoku::Generator::new();
    println!("{:.6}", time_solve(&solver, easy) as f64 / 1000000000.0);
    println!("{:.6}", time_solve(&solver, hard) as f64 / 1000000000.0);
    println!("{:.6}", time_solve(&solver, hardest) as f64 / 1000000000.0);
    println!("{:.6}", time_solve(&solver, &generator.generate_str(17)) as f64 / 1000000000.0);
}
