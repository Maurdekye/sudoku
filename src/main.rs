#![feature(int_roundings)]
#![feature(generic_arg_infer)]
#![feature(array_chunks)]
use std::{
    fmt::Display,
    iter::empty,
    ops::{Index, IndexMut},
    str::FromStr,
};

use space_search::{Scoreable, Searchable, SolutionIdentifiable};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Board<Cell>([Cell; 81]);

impl<Cell> Board<Cell> {
    fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.0.iter()
    }

    fn iter_positions() -> impl Iterator<Item = BoardPosition> {
        (0..81).map(|i| (i % 9, i / 9))
    }
}

type BoardPosition = (usize, usize);

impl<Cell> Index<BoardPosition> for Board<Cell> {
    type Output = Cell;

    fn index(&self, (x, y): BoardPosition) -> &Self::Output {
        &self.0[y * 9 + x]
    }
}

impl<Cell> IndexMut<BoardPosition> for Board<Cell> {
    fn index_mut(&mut self, (x, y): BoardPosition) -> &mut Self::Output {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct SudokuChoices(u16);

impl SudokuChoices {
    fn all() -> Self {
        SudokuChoices(0b111111111)
    }

    fn none() -> Self {
        SudokuChoices(0b000000000)
    }

    fn one(space: Space) -> Self {
        let mut choices = SudokuChoices::none();
        choices.set(space, true);
        choices
    }

    fn new(initial_choice: Option<Space>) -> Self {
        match initial_choice {
            Some(space) => SudokuChoices::one(space),
            None => SudokuChoices::all(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = Space> + '_ {
        (0..9).filter_map(|i| {
            ((1 << i) & self.0 != 0).then(|| Space::try_from(i + 1).unwrap())
        })
    }

    fn set(&mut self, space: Space, value: bool) {
        if value {
            self.0 |= 1 << space.idx();
        } else {
            self.0 &= !(1 << space.idx());
        }
    }
}

impl Index<Space> for SudokuChoices {
    type Output = bool;

    fn index(&self, index: Space) -> &Self::Output {
        // this is stupid, this shouldnt work :/
        if self.0 & (1 << index.idx()) != 0 {
            &true
        } else {
            &false
        }
    }
}

impl Display for SudokuChoices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            (0..9)
                .map(|i| if self.0 & (1 << i) != 0 {
                    (i + 1).to_string()
                } else {
                    " ".to_string()
                })
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

type PossibilitySpaceBoard = Board<SudokuChoices>;
impl PossibilitySpaceBoard {
    fn new(board: &SudokuBoard) -> Self {
        Board(board.0.map(SudokuChoices::new))
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
                        .map(|x| format!("{}", &self[(x, y)]))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[derive(Debug)]
enum SudokuRegion {
    Column(usize),
    Row(usize),
    Square(usize),
}
use SudokuRegion::*;

impl SudokuRegion {
    fn row_of((_, y): BoardPosition) -> SudokuRegion {
        Row(y)
    }

    fn column_of((x, _): BoardPosition) -> SudokuRegion {
        Column(x)
    }

    fn square_of((x, y): BoardPosition) -> SudokuRegion {
        Square((y / 3) * 3 + (x / 3))
    }
}

impl IntoIterator for SudokuRegion {
    type Item = BoardPosition;

    type IntoIter = SudokuRegionIter;

    fn into_iter(self) -> Self::IntoIter {
        SudokuRegionIter {
            region: self,
            index: Some(0),
        }
    }
}

struct SudokuRegionIter {
    region: SudokuRegion,
    index: Option<usize>,
}

impl Iterator for SudokuRegionIter {
    type Item = BoardPosition;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index?;
        let next_pos = match &self.region {
            &Column(col) => (col, index),
            &Row(row) => (index, row),
            &Square(square) => (
                (square % 3) * 3 + (index % 3),
                (square / 3) * 3 + (index / 3),
            ),
        };
        self.index = (index < 8).then_some(index + 1);
        Some(next_pos)
    }
}

impl SudokuBoard {
    fn reduce(&mut self) -> (PossibilitySpaceBoard, bool) {
        fn set(
            board: &mut SudokuBoard,
            possibilities_board: &mut PossibilitySpaceBoard,
            pos: BoardPosition,
            space: Space,
        ) -> bool {
            let mut is_invalid = false;
            if board[pos].is_none() {
                board[pos] = Some(space);
                possibilities_board[pos] = SudokuChoices::one(space);

                for pos in empty()
                    .chain(SudokuRegion::row_of(pos))
                    .chain(SudokuRegion::column_of(pos))
                    .chain(SudokuRegion::square_of(pos))
                    .filter(|p| p != &pos)
                {
                    possibilities_board[pos].set(space, false);
                    let remaining_possibilities =
                        possibilities_board[pos].iter().take(2).collect::<Vec<_>>();
                    is_invalid = match &remaining_possibilities[..] {
                        &[] => true,
                        &[only] if board[pos].is_none() => {
                            set(board, possibilities_board, pos, only)
                        }
                        _ => false,
                    };
                    if is_invalid {
                        break;
                    }
                }
            } else {
                unimplemented!(
                    "Not allowed to change the value of an already set space: {:?} to {:?} at {:?}",
                    board[pos],
                    space,
                    pos
                );
            }
            is_invalid
        }

        let mut possibilities_board = PossibilitySpaceBoard::new(self);

        let mut is_invalid = false;

        'outer: loop {
            let mut adjusted = false;

            for pos in SudokuBoard::iter_positions() {
                let mut new_possibilities = possibilities_board[pos].clone();

                if self[pos].is_none() {
                    for region in [
                        SudokuRegion::row_of(pos),
                        SudokuRegion::column_of(pos),
                        SudokuRegion::square_of(pos),
                    ] {
                        let mut solo_candidates = new_possibilities.clone();
                        for pos in region
                            .into_iter()
                            .filter(|p| p != &pos) 
                        {
                            if let Some(space) = self[pos] {
                                new_possibilities.set(space, false);
                            }
                            for space in possibilities_board[pos].iter() {
                                solo_candidates.set(space, false);
                            }
                        }
                        if let &[value] = &solo_candidates.iter().take(2).collect::<Vec<_>>()[..] {
                            new_possibilities = SudokuChoices::one(value);
                            break;
                        }
                    }

                    // update possibility space
                    adjusted |= new_possibilities != possibilities_board[pos];
                    possibilities_board[pos] = new_possibilities;
                }

                // confirm square if all alternative possibilities are exhausted
                let remaining_possibilities = new_possibilities.iter().take(2).collect::<Vec<_>>();
                match &remaining_possibilities[..] {
                    &[] => {
                        is_invalid = true;
                    }
                    &[value] if self[pos].is_none() => {
                        is_invalid |= set(self, &mut possibilities_board, pos, value);
                    }
                    _ => {}
                }
                if is_invalid {
                    break 'outer;
                }
            }

            if !adjusted || self.is_solution() {
                break;
            }
        }

        (possibilities_board, is_invalid)
    }

    #[allow(unused)]
    fn validate(&self) -> Result<(), String> {
        fn verify_set(it: impl Iterator<Item = Space>) -> Result<(), Space> {
            let mut choices = SudokuChoices::none();
            for space in it {
                if choices[space] {
                    return Err(space);
                }
                choices.set(space, true);
            }
            Ok(())
        }
        for i in 0..9 {
            let space_at = |pos| self[pos];
            if let Err(invalid_space) = verify_set(Row(i).into_iter().filter_map(space_at)) {
                Err(format!("Row {i} is invalid: duplicate {invalid_space:?}"))?;
            }
            if let Err(invalid_space) = verify_set(Column(i).into_iter().filter_map(space_at)) {
                Err(format!(
                    "Column {i} is invalid: duplicate {invalid_space:?}"
                ))?;
            }
            if let Err(invalid_space) = verify_set(Square(i).into_iter().filter_map(space_at)) {
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
                        _ => Err(format!("Character '{chr}' is not valid for a sudoku board")),
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .flatten()
            .collect::<Vec<_>>();
        let space_count = collect.len();
        Ok(Board(collect.try_into().map_err(|_| {
            format!("Incorrect number of spaces on sudoku board: expected 81, found {space_count}")
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

enum NextBoardStates<I> {
    Single(Option<SudokuBoard>),
    States(I),
}

impl<I> Iterator for NextBoardStates<I>
where
    I: Iterator<Item = SudokuBoard>,
{
    type Item = SudokuBoard;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            NextBoardStates::Single(board) => board.take(),
            NextBoardStates::States(iter) => iter.next(),
        }
    }
}

impl Searchable for SudokuBoard {
    fn next_states(&self) -> impl Iterator<Item = Self> {
        let mut reduced_board = self.clone();
        let (possibilities_board, is_invalid) = reduced_board.reduce();
        if is_invalid {
            NextBoardStates::Single(None)
        } else if reduced_board.is_solution() || &reduced_board != self {
            NextBoardStates::Single(Some(reduced_board))
        } else {
            NextBoardStates::States(
                SudokuBoard::iter_positions()
                    .filter({
                        let reduced_board = reduced_board.clone();
                        move |&pos| reduced_board[pos].is_none()
                    })
                    .flat_map(move |pos| {
                        possibilities_board[pos]
                            .clone()
                            .iter()
                            .map({
                                let reduced_board = reduced_board.clone();
                                move |space| {
                                    let mut new_board = reduced_board.clone();
                                    new_board[pos] = Some(space);
                                    new_board
                                }
                            })
                            .collect::<Vec<_>>()
                    }),
            )
        }
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
    assert_eq!(board.validate(), Ok(()));
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
    assert_eq!(solution.validate(), Ok(()));
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
    assert_eq!(solution.validate(), Ok(()));
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
    assert_eq!(solution.validate(), Ok(()));
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
    assert_eq!(solution.validate(), Ok(()));
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
    assert_eq!(board[(0, 0)], Some(Space::Four));
}

#[test]
fn test_reduction_2() {
    let mut board: SudokuBoard = "2  5974 6
6 4231   
   8  23 
    2    
86231    
 45    2 
4 918276 
786953142
 21  6  8"
        .parse()
        .unwrap();
    board.reduce();
    let solution_board: SudokuBoard = "238597416
694231857
517864239
173429685
862315974
945678321
459182763
786953142
321746598"
        .parse()
        .unwrap();
    assert_eq!(board, solution_board);
}

#[test]
fn test_manual_solve() {
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
    let mut board: SudokuBoard = board_str.parse().unwrap();
    println!("\n{board}");
    board.reduce();
    println!("\n{board}");
    println!("next moves: {}", board.next_states().count());
    let before_adjustment = board.clone();
    board[(3, 2)] = Some(Space::Eight);
    println!("\n{board}");
    assert!(before_adjustment
        .next_states()
        .find(|b| b == &board)
        .is_some());
    board.reduce();
    println!("\n{board}");
    assert_eq!(board.validate(), Ok(()));
    assert!(board.is_solution());
}
fn main() {
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
    let mut searcher: Searcher<guided::route::hashable::Manager<_>, _> = Searcher::new(board);
    let solution = searcher.next().expect("Sudoku board has a solution");
    println!("solution:");
    for board in solution {
        println!("---------\n{}", board);
    }
}
