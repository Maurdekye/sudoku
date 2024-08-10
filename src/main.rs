#![feature(int_roundings)]
#![feature(generic_arg_infer)]
#![feature(array_chunks)]
#![feature(iterator_try_collect)]
use std::{
    fmt::Display,
    ops::{Index, IndexMut},
    str::FromStr,
};

use space_search::{search::guided, Scoreable, Searchable, Searcher, SolutionIdentifiable};

#[derive(Clone, Hash, PartialEq, Eq)]
struct Board<Cell>([Cell; 81]);

impl<Cell> Board<Cell> {
    fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.0.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        self.0.iter_mut()
    }
}

impl<Cell> Index<(usize, usize)> for Board<Cell> {
    type Output = Cell;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.0[y * 9 + x]
    }
}

impl<Cell> IndexMut<(usize, usize)> for Board<Cell> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &mut self.0[y * 9 + x]
    }
}

#[derive(Clone, Debug, Copy, Hash, PartialEq, Eq)]
enum Space {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

impl Space {
    fn idx(&self) -> usize {
        let space_number: usize = (*self).into();
        space_number - 1
    }
}

impl From<Space> for usize {
    fn from(value: Space) -> Self {
        use Space::*;
        match value {
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
            Six => 6,
            Seven => 7,
            Eight => 8,
            Nine => 9,
        }
    }
}

impl TryFrom<usize> for Space {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        use Space::*;
        let result = match value {
            1 => One,
            2 => Two,
            3 => Three,
            4 => Four,
            5 => Five,
            6 => Six,
            7 => Seven,
            8 => Eight,
            9 => Nine,
            _ => return Err(format!("Cant convert '{}' to a space value", value)),
        };
        Ok(result)
    }
}

type SudokuBoard = Board<Option<Space>>;

impl SudokuBoard {
    fn solved_regions(&self) -> SolvedRegions {
        SolvedRegions {
            rows: (0..9)
                .map(|y| (0..9).all(|x| self[(x, y)].is_some()))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            columns: (0..9)
                .map(|x| (0..9).all(|y| self[(x, y)].is_some()))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            squares: (0..9)
                .map(|s| {
                    let (left, top) = ((s % 3) * 3, (s / 3) * 3);
                    (0..9).all(|i| {
                        let (x, y) = (left + (i % 3), top + (i / 3));
                        self[(x, y)].is_some()
                    })
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    fn is_solution(&self) -> bool {
        self.iter().all(|space| space.is_some())
    }
}

impl FromStr for SudokuBoard {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let collect = s
            .split('\n')
            .flat_map(|row| {
                row.chars()
                    .map(|chr| match chr {
                        ' ' => Ok(None),
                        '1'..='9' => Ok(Some(
                            Space::try_from(
                                chr.to_digit(10)
                                    .expect("char will always be convertible to a digit")
                                    as usize,
                            )
                            .expect("char will always be convertible to a digit"),
                        )),
                        _ => Err(format!(
                            "Character '{}' is not valid for a sudoku board",
                            chr
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .flatten()
            .collect::<Vec<_>>();
        Ok(Board(collect.try_into().map_err(|_| {
            String::from("Incorrect number of spaces on sudoku boad")
        })?))
    }
}

impl Display for SudokuBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .array_chunks()
                .map(|row: &[_; 9]| {
                    row.iter()
                        .map(|space| match space {
                            None => String::from(" "),
                            Some(space) => format!("{}", usize::from(*space)),
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

type PossibilitySpaceBoard = Board<[bool; 9]>;

fn format_bool_array<const N: usize>(arry: &[bool; N]) -> String {
    format!(
        "[{}]",
        arry.iter()
            .enumerate()
            .map(|(i, &open)| if open {
                (i + 1).to_string()
            } else {
                " ".to_string()
            })
            .collect::<Vec<_>>()
            .join("")
    )
}

impl Display for PossibilitySpaceBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            (0..9)
                .map(|y| {
                    (0..9)
                        .map(|x| format_bool_array(&self[(x, y)]))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct SolvedRegions {
    rows: [bool; 9],
    columns: [bool; 9],
    squares: [bool; 9],
}

impl std::fmt::Debug for SolvedRegions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Solved rows: {}", format_bool_array(&self.rows))?;
        writeln!(f, "Solved columns: {}", format_bool_array(&self.columns))?;
        writeln!(f, "Solved squares: {}", format_bool_array(&self.squares))?;
        Ok(())
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct SudokuGame {
    board: SudokuBoard,
    possibilities_board: PossibilitySpaceBoard,
    scores: SolvedRegions,
    is_invalid: bool,
}

impl SudokuGame {
    fn new(board: SudokuBoard) -> Self {
        let mut possibilities_board = Board([[true; 9]; 81]);

        for (possibilities, space) in possibilities_board.iter_mut().zip(board.iter()) {
            if let Some(space) = space {
                *possibilities = [false; 9];
                possibilities[space.idx()] = true;
            }
        }

        let scores = board.solved_regions();

        Self {
            board,
            possibilities_board,
            scores,
            is_invalid: false,
        }
    }

    fn set(&mut self, pos: (usize, usize), value: Space) {
        if self.board[pos].is_none() {
            let space_idx = value.idx();
            self.board[pos] = Some(value);
            self.possibilities_board[pos] = [false; 9];
            self.possibilities_board[pos][space_idx] = true;

            let (x, y) = pos;
            let (left, top) = ((x / 3) * 3, (y / 3) * 3);
            self.scores.rows[y] = true;
            self.scores.columns[x] = true;
            let square_idx = (top * 3 + left) / 3;
            self.scores.squares[square_idx] = true;
            for i in 0..9 {
                fn attend_to_pos(
                    game: &mut SudokuGame,
                    pos: (usize, usize),
                    space_idx: usize,
                    mut evaluated_score_setter: impl FnMut(&mut SudokuGame, bool),
                ) {
                    game.possibilities_board[pos][space_idx] = false;
                    let remaining_possibilities = game.possibilities_board[pos]
                        .iter()
                        .enumerate()
                        .filter_map(|(i, o)| o.then_some(Space::try_from(i + 1).unwrap()))
                        .collect::<Vec<_>>();
                    match &remaining_possibilities[..] {
                        &[] => {
                            game.is_invalid = true;
                        }
                        &[only] if game.board[pos].is_none() => {
                            game.set(pos, only);
                        }
                        &[_, _, ..] => {
                            evaluated_score_setter(game, false);
                        }
                        _ => {}
                    }
                }

                if i != x {
                    attend_to_pos(self, (i, y), space_idx, |game, value| {
                        game.scores.rows[y] = value
                    });
                }

                if i != y {
                    attend_to_pos(self, (x, i), space_idx, |game, value| {
                        game.scores.columns[x] = value
                    });
                }

                let (sx, sy) = (left + (i % 3), top + (i / 3));
                if (sx, sy) != (x, y) {
                    attend_to_pos(self, (sx, sy), space_idx, |game, value| {
                        game.scores.squares[square_idx] = value
                    });
                }
            }
        } else {
            unimplemented!(
                "Not allowed to change the value of an already set space: {:?} to {:?} at {:?}",
                self.board[pos],
                value,
                pos
            );
        }
    }

    fn reduce(&mut self) {
        if self.is_invalid {
            return;
        }

        loop {
            let mut adjusted = false;

            for i in 0..81 {
                let (x, y) = (i % 9, i / 9);
                println!(
                    "\npass on square ({x}, {y}): {:?}, {}",
                    self.board[(x, y)],
                    format_bool_array(&self.possibilities_board[(x, y)])
                );
                println!("before:\n{:?}", self);

                if self.board[(x, y)].is_some() {
                    continue;
                }

                let mut new_possibilities = self.possibilities_board[(x, y)].clone();

                if new_possibilities.iter().filter(|x| **x).count() > 1 {
                    // check current row
                    for dx in 0..9 {
                        if x != dx {
                            if let Some(space) = &self.board[(dx, y)] {
                                new_possibilities[space.idx()] = false;
                            }
                        }
                    }

                    // check current column
                    for dy in 0..9 {
                        if y != dy {
                            if let Some(space) = &self.board[(x, dy)] {
                                new_possibilities[space.idx()] = false;
                            }
                        }
                    }

                    // check current box
                    let left = x - x % 3;
                    let top = y - y % 3;
                    for i in 0..9 {
                        let (dx, dy) = (left + (i % 3), top + (i / 3));
                        if (x, y) != (dx, dy) {
                            if let Some(space) = &self.board[(dx, dy)] {
                                new_possibilities[space.idx()] = false;
                            }
                        }
                    }

                    // update possibility space
                    if new_possibilities != self.possibilities_board[(x, y)] {
                        adjusted = true;
                    }
                    self.possibilities_board[(x, y)] = new_possibilities;
                }

                let remaining_possibilities = new_possibilities
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| p.then_some(i))
                    .collect::<Vec<_>>();
                match &remaining_possibilities[..] {
                    // mark board as invalid if a square has no possible moves remaining
                    &[] => {
                        self.is_invalid = true;
                        break;
                    }

                    // confirm square if only one possible move remains
                    &[i] => {
                        self.set(
                            (x, y),
                            (i + 1)
                                .try_into()
                                .expect("index will always correspond to a valid space value"),
                        );
                        adjusted = true;
                    }
                    _ => {}
                }
            }

            if !adjusted {
                break;
            }
        }
    }
}

impl Display for SudokuGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.board)
    }
}

impl std::fmt::Debug for SudokuGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "board:")?;
        writeln!(f, "{}", self.board)?;
        writeln!(f, "possibilities:")?;
        writeln!(f, "{}", self.possibilities_board)?;
        writeln!(f, "{:?}", self.scores)?;
        writeln!(f, "invalid? {}", self.is_invalid)?;
        Ok(())
    }
}

impl From<SudokuBoard> for SudokuGame {
    fn from(value: SudokuBoard) -> Self {
        SudokuGame::new(value)
    }
}

impl SolutionIdentifiable for SudokuGame {
    fn is_solution(&self) -> bool {
        [self.scores.rows, self.scores.columns, self.scores.squares]
            .iter()
            .all(|arry| arry.iter().all(|b| *b))
    }
}

struct NextSudokuBoardsIterator {
    sudoku: SudokuGame,
    index: usize,
    sub_index: usize,
}
// static mut COUNTER: usize = 0;

impl Iterator for NextSudokuBoardsIterator {
    type Item = SudokuGame;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let NextSudokuBoardsIterator {
                sudoku:
                    SudokuGame {
                        board,
                        possibilities_board,
                        is_invalid,
                        ..
                    },
                index,
                sub_index,
            } = self;

            // unsafe {
            //     if COUNTER % 1_000_000 == 0 {
            //         println!("---------\n{reduced_board}");
            //     }
            //     COUNTER += 1;
            // }

            if *index >= 81 || *is_invalid {
                return None;
            }

            // hacky but im too lazy to implement this properly with a de facto abstraction
            if board.is_solution() {
                *index = 81;
                return Some(self.sudoku.clone());
            }

            if *sub_index >= 9 {
                *index += 1;
                *sub_index = 0;
                continue;
            }

            let (x, y) = (*index % 9, *index / 9);

            if board[(x, y)].is_some() {
                *index += 1;
                continue;
            }

            let possibilities = &possibilities_board[(x, y)];

            if possibilities[*sub_index] {
                let mut new_game = self.sudoku.clone();
                new_game.set((x, y), Space::try_from(*sub_index + 1).unwrap());
                *sub_index += 1;
                return Some(new_game);
            }

            *sub_index += 1;
        }
    }
}

impl Searchable for SudokuGame {
    fn next_states(&self) -> impl Iterator<Item = Self> {
        let mut sudoku = self.clone();
        sudoku.reduce();
        return NextSudokuBoardsIterator {
            sudoku,
            index: 0,
            sub_index: 0,
        };
    }
}

#[derive(PartialEq, Eq)]
struct SudokuBoardScore {
    unsolved_regions: usize,
    empty_squares: usize,
}

impl PartialOrd for SudokuBoardScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.unsolved_regions.partial_cmp(&other.unsolved_regions) {
            Some(core::cmp::Ordering::Equal) => {
                self.empty_squares.partial_cmp(&other.empty_squares)
            }
            ord => ord,
        }
    }
}

impl Ord for SudokuBoardScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(&other).unwrap()
    }
}

impl Scoreable for SudokuGame {
    type Score = SudokuBoardScore;

    fn score(&self) -> Self::Score {
        fn sum_scores(scores: &[bool; 9]) -> usize {
            scores.iter().filter(|&&x| x).count()
        }
        let SolvedRegions {
            rows,
            columns,
            squares,
        } = self.scores;
        SudokuBoardScore {
            unsolved_regions: sum_scores(&rows) + sum_scores(&columns) + sum_scores(&squares),
            empty_squares: self.board.iter().filter(|space| space.is_none()).count(),
        }
    }
}

#[test]
fn test_reduction() {
    #[rustfmt::skip]
    let board_str = 
"53  7    
6  195   
 98    6 
8   6   3
4  8 3  1
7   2   6
 6    28 
   419  5
    8  79";
    let mut game = SudokuGame::new(board_str.parse().unwrap());
    println!("initial board:");
    println!("{}", game);
    game.reduce();
    println!("after reduction:");
    println!("{}", game);
}

#[test]
fn test_solve_hard() {
    #[rustfmt::skip]
    let board_str = 
"2  5 74 6
    31   
      23 
    2    
86 31    
 45      
  9   7  
  695   2
  1  6  8";
    let game = SudokuGame::new(board_str.parse().unwrap());
    println!("initial board:");
    println!("{}", game);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(game);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

#[test]
fn test_solve_hard_2() {
    #[rustfmt::skip]
    let board_str = 
"  65     
7 5  23  
 3     8 
 5  96 7 
1 4     8
   82    
 2     9 
  72  4  
     75  ";
    let game = SudokuGame::new(board_str.parse().unwrap());
    println!("initial board:");
    println!("{}", game);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(game);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

#[test]
fn test_solve_hard_3() {
    #[rustfmt::skip]
    let board_str = 
" 293 8456
5782 61 9
   1 5 7 
3 5 2 6  
     9 4 
 91 67   
 3  5    
     29 3
9 7    24";
    let game = SudokuGame::new(board_str.parse().unwrap());
    println!("initial board:");
    println!("{}", game);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(game);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

#[test]
fn test_solve_hard_4() {
    #[rustfmt::skip]
    let board_str = 
"5 8427   
 4  1 7  
19   3  2
    6   5
7     2  
6 513 9  
9    15  
    4  2 
 7      8";
    let game = SudokuGame::new(board_str.parse().unwrap());
    println!("initial board:");
    println!("{}", game);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(game);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

fn main() {
    #[rustfmt::skip]
    let board_str = 
"53  7    
6  195   
 98    6 
8   6   3
4  8 3  1
7   2   6
 6    28 
   419  5
    8  79";
    let mut game = SudokuGame::new(board_str.parse().unwrap());
    println!("initial board:");
    println!("{}", game);
    game.reduce();
    println!("after reduction:");
    println!("{}", game);
}
