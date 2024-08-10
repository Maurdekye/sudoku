#![feature(int_roundings)]
#![feature(generic_arg_infer)]
#![feature(array_chunks)]
use std::{
    fmt::Display,
    io::{stdout, Write},
    ops::{ControlFlow, Index, IndexMut},
    str::FromStr,
};

use space_search::{Scoreable, Searchable, SolutionIdentifiable};

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
type SudokuChoices = [bool; 9];

fn format_sudoku_choices(choices: &SudokuChoices) -> String {
    format!(
        "[{}]",
        choices
            .iter()
            .enumerate()
            .map(|(i, open)| if *open {
                (i + 1).to_string()
            } else {
                " ".to_string()
            })
            .collect::<Vec<_>>()
            .join("")
    )
}

type PossibilitySpaceBoard = Board<SudokuChoices>;
impl PossibilitySpaceBoard {
    fn new(board: &SudokuBoard) -> Self {
        let mut possibility_space = Board([[true; 9]; _]);
        for (possibilities, space) in possibility_space.iter_mut().zip(board.iter()) {
            if let Some(space) = space {
                let space_number = space.idx();
                for (space_idx, value) in possibilities.iter_mut().enumerate() {
                    *value = space_number == space_idx;
                }
            }
        }
        possibility_space
    }
}

impl Display for PossibilitySpaceBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            (0..9)
                .map(|y| {
                    (0..9)
                        .map(|x| format_sudoku_choices(&self[(x, y)]))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

fn set(
    board: &mut SudokuBoard,
    possibilities_board: &mut PossibilitySpaceBoard,
    pos: (usize, usize),
    value: Space,
) -> bool {
    let mut is_invalid = false;
    if board[pos].is_none() {
        let space_idx = value.idx();
        board[pos] = Some(value);
        possibilities_board[pos] = [false; 9];
        possibilities_board[pos][space_idx] = true;

        let (x, y) = pos;
        let (left, top) = ((x / 3) * 3, (y / 3) * 3);
        for i in 0..9 {
            fn attend_to_pos(
                board: &mut SudokuBoard,
                possibilities_board: &mut PossibilitySpaceBoard,
                pos: (usize, usize),
                space_idx: usize,
            ) -> bool {
                possibilities_board[pos][space_idx] = false;
                let remaining_possibilities = possibilities_board[pos]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, o)| o.then_some(Space::try_from(i + 1).unwrap()))
                    .collect::<Vec<_>>();
                match &remaining_possibilities[..] {
                    &[] => true,
                    &[only] if board[pos].is_none() => set(board, possibilities_board, pos, only),
                    _ => false,
                }
            }

            if i != x {
                if attend_to_pos(board, possibilities_board, (i, y), space_idx) {
                    is_invalid = true;
                    break;
                }
            }

            if i != y {
                if attend_to_pos(board, possibilities_board, (x, i), space_idx) {
                    is_invalid = true;
                    break;
                }
            }

            let (sx, sy) = (left + (i % 3), top + (i / 3));
            if (sx, sy) != (x, y) {
                if attend_to_pos(board, possibilities_board, (sx, sy), space_idx) {
                    is_invalid = true;
                    break;
                }
            }
        }
    } else {
        unimplemented!(
            "Not allowed to change the value of an already set space: {:?} to {:?} at {:?}",
            board[pos],
            value,
            pos
        );
    }
    is_invalid
}

impl SudokuBoard {
    fn reduce(&mut self) -> (PossibilitySpaceBoard, bool) {
        // prepare possibility space
        let mut possibilities_board = PossibilitySpaceBoard::new(self);

        let mut is_invalid = false;

        loop {
            let mut adjusted = false;

            for i in 0..81 {
                let (x, y) = (i % 9, i / 9);
                let mut new_possibilities = possibilities_board[(x, y)].clone();

                #[cfg(debug_assertions)]
                {
                    println!();
                    println!(
                        "{:?}, {}",
                        (x, y),
                        format_sudoku_choices(&new_possibilities)
                    );
                    println!("{}", self);
                    println!("{}", possibilities_board);
                    println!();
                }

                if self[(x, y)].is_none() {
                    'checks: {
                        fn check_region(
                            board: &mut SudokuBoard,
                            possibilities_board: &mut PossibilitySpaceBoard,
                            new_possibilities: &mut SudokuChoices,
                            region_positions: impl Iterator<Item = (usize, usize)>,
                        ) -> ControlFlow<(), ()> {
                            let mut solo_candidates = new_possibilities.clone();
                            for pos in region_positions {
                                if let Some(space) = &board[pos] {
                                    new_possibilities[space.idx()] = false;
                                }
                                for i in possibilities_board[pos]
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, o)| o.then_some(i))
                                {
                                    solo_candidates[i] = false;
                                }
                            }
                            if let &[value] = &solo_candidates
                                .iter()
                                .enumerate()
                                .filter_map(|(i, o)| o.then_some(i))
                                .collect::<Vec<_>>()[..]
                            {
                                *new_possibilities = [false; 9];
                                new_possibilities[value] = true;
                                return ControlFlow::Break(());
                            }
                            return ControlFlow::Continue(());
                        }

                        // check current row
                        if check_region(
                            self,
                            &mut possibilities_board,
                            &mut new_possibilities,
                            (0..9).filter(|i| *i != x).map(|i| (i, y)),
                        )
                        .is_break()
                        {
                            break 'checks;
                        }

                        // check current column
                        if check_region(
                            self,
                            &mut possibilities_board,
                            &mut new_possibilities,
                            (0..9).filter(|i| *i != y).map(|i| (x, i)),
                        )
                        .is_break()
                        {
                            break 'checks;
                        }

                        // check current square
                        let left = x - x % 3;
                        let top = y - y % 3;
                        if check_region(
                            self,
                            &mut possibilities_board,
                            &mut new_possibilities,
                            (0..9)
                                .map(|i| (left + (i % 3), top + (i / 3)))
                                .filter(|pos| *pos != (x, y)),
                        )
                        .is_break()
                        {
                            break 'checks;
                        }
                    }

                    // update possibility space
                    if new_possibilities != possibilities_board[(x, y)] {
                        adjusted = true;
                    }
                    possibilities_board[(x, y)] = new_possibilities;
                }

                // confirm square if all alternative possibilities are exhausted
                let remaining_possibilities = new_possibilities
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| p.then_some(Space::try_from(i + 1).unwrap()))
                    .collect::<Vec<_>>();
                match &remaining_possibilities[..] {
                    &[] => {
                        is_invalid = true;
                        break;
                    }
                    &[value] if self[(x, y)].is_none() => {
                        is_invalid |= set(self, &mut possibilities_board, (x, y), value);
                        if is_invalid {
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if !adjusted || self.is_solution() {
                break;
            }
        }

        (possibilities_board, is_invalid)
    }

    fn validate(&self) -> Result<(), String> {
        fn verify_set(it: impl Iterator<Item = Space>) -> Result<(), Space> {
            let mut choices = [false; 9];
            for space in it {
                let space_idx = space.idx();
                if choices[space_idx] {
                    return Err(space);
                }
                choices[space_idx] = true;
            }
            Ok(())
        }
        for i in 0..9 {
            if let Err(invalid_space) = verify_set((0..9).filter_map(|x| self[(x, i)])) {
                Err(format!("Row {i} is invalid: duplicate {invalid_space:?}"))?;
            }
            if let Err(invalid_space) = verify_set((0..9).filter_map(|y| self[(i, y)])) {
                Err(format!(
                    "Column {i} is invalid: duplicate {invalid_space:?}"
                ))?;
            }
            let (left, top) = ((i % 3) * 3, (i / 3) * 3);
            if let Err(invalid_space) = verify_set((0..9).filter_map(|i| {
                let (sx, sy) = (left + (i % 3), top + (i / 3));
                self[(sx, sy)]
            })) {
                Err(format!(
                    "Square {i} is invalid: duplicate {invalid_space:?}"
                ))?;
            }
        }
        Ok(())
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

struct NextSudokuBoardsIterator {
    reduced_board: SudokuBoard,
    possibilities_board: PossibilitySpaceBoard,
    index: usize,
    sub_index: usize,
}

static mut COUNTER: usize = 0;

impl Iterator for NextSudokuBoardsIterator {
    type Item = SudokuBoard;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let NextSudokuBoardsIterator {
                reduced_board,
                possibilities_board,
                index,
                sub_index,
            } = self;

            if *index >= 81 {
                return None;
            }

            if reduced_board.is_solution() {
                // hacky but im too lazy to implement this properly with a de facto abstraction
                *index = 81;
                return Some(self.reduced_board.clone());
            }

            if *sub_index >= 9 {
                *index += 1;
                *sub_index = 0;
                continue;
            }

            let (x, y) = (*index % 9, *index / 9);

            if reduced_board[(x, y)].is_some() {
                *index += 1;
                continue;
            }

            let possibilities = &possibilities_board[(x, y)];

            if possibilities[*sub_index] {
                let mut new_board = reduced_board.clone();
                new_board[(x, y)] = Some(Space::try_from(*sub_index + 1).unwrap());
                *sub_index += 1;
                return Some(new_board);
            }

            *sub_index += 1;
        }
    }
}

impl Searchable for SudokuBoard {
    fn next_states(&self) -> impl Iterator<Item = Self> {
        unsafe {
            COUNTER += 1;
            if COUNTER % 10_000 == 0 {
                print!("\r{}", COUNTER);
                stdout().flush().unwrap();
            }
        }

        let mut reduced_board = self.clone();
        let (possibilities_board, is_invalid) = reduced_board.reduce();
        return NextSudokuBoardsIterator {
            reduced_board,
            possibilities_board,
            index: if is_invalid { 81 } else { 0 },
            sub_index: 0,
        };
    }
}

impl SolutionIdentifiable for SudokuBoard {
    fn is_solution(&self) -> bool {
        self.iter().all(|space| space.is_some())
    }
}

impl Scoreable for SudokuBoard {
    type Score = usize;

    fn score(&self) -> Self::Score {
        self.iter().filter(|space| space.is_none()).count()
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
    let mut board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    board.reduce();
    println!("after reduction:");
    println!("{}", board);
}

#[test]
fn test_solve_hard() {
    use space_search::{search::*, *};
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
    let board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(board);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

#[test]
fn test_solve_hard_2() {
    use space_search::{search::*, *};
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
    let board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(board);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

#[test]
fn test_solve_hard_3() {
    use space_search::{search::*, *};
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
    let board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(board);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

#[test]
fn test_solve_hard_4() {
    use space_search::{search::*, *};
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
    let board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    let mut searcher: Searcher<guided::no_route::hashable::Manager<_>, _> = Searcher::new(board);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    println!("{}", solution);
}

// #[test]
// fn test_verify() {
//     let board_str = "213587496
// 594631887
// 758469231
// 937625814
// 862314579
// 145798623
// 329143765
// 476958312
// 351276948";
//     let board: SudokuBoard = board_str.parse().unwrap();
//     assert_eq!(board.validate(), Ok(()))
// }

#[test]
fn test_solver_duplicates() {
    // use space_search::{search::*, *};
    #[rustfmt::skip]
    let board_str = 
"213587496
    31   
      23 
937 25  4
86231  7 
1457   23
 29 4 76 
 7695   2
  1276  8";
    let mut board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    board.reduce();
    println!("solution:");
    println!("{}", board);
    assert_eq!(board.validate(), Ok(()));
}

#[test]
fn test_solo_candidate_deduction() {
    #[rustfmt::skip]
    let board_str = 
"        
3        
6        
2        
1        
     4   
8        
       4 ";
    println!("{}", board_str.len());
    let mut board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    board.reduce();
    println!("solution:");
    println!("{}", board);
    assert_eq!(board.validate(), Ok(()));
}

fn main() {
    #[rustfmt::skip]
    let board_str = 
"         
3        
6        
2        
1        
     4   
8        
5        
       4 ";
    println!("{}", board_str.len());
    let mut board: SudokuBoard = board_str.parse().unwrap();
    println!("initial board:");
    println!("{}", board);
    board.reduce();
    println!("solution:");
    println!("{}", board);
    assert_eq!(board.validate(), Ok(()));
}
