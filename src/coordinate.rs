// use coord_row;
use pest::Parser;
use serde::{Deserialize, Serialize};
use std::char::from_u32;
use std::num::NonZeroU32;
use std::option::Option;
use std::panic;

use crate::coord;
use crate::util::{coord_show, non_zero_u32_tuple};

#[derive(Parser)]
#[grammar = "coordinate.pest"]
pub struct CoordinateParser;

// Coordinate specifies the nested coordinate structure
#[derive(Deserialize, PartialEq, Eq, Debug, Hash, Clone)]
pub struct Coordinate {
    pub row_cols: Vec<(NonZeroU32, NonZeroU32)>, // TEST: should never be empty list
}
js_serializable!(Coordinate);
js_deserializable!(Coordinate);

impl Coordinate {
    // TEST:
    // - parent.row_cols.len() == result.row_cols.len() - 1
    pub fn child_of(parent: &Self, child_coord: (NonZeroU32, NonZeroU32)) -> Coordinate {
        let mut new_row_col = parent.clone().row_cols;
        new_row_col.push(child_coord);
        info! {"parent and child val: (pa: {:?}, child: {:?}, fin {:?});", parent, child_coord, new_row_col};
        Coordinate {
            row_cols: new_row_col,
        }
    }

    // TEST:
    // - test parent(coord!("root")) == None
    // - test parent(coord!("meta")) == None
    // - self.row_cols.len() == parent.row_cols.len() + 1
    pub fn parent(&self) -> Option<Coordinate> {
        if self.row_cols.len() == 1 {
            return None;
        }

        let parent = {
            let mut temp = self.clone();
            temp.row_cols.pop();
            temp
        };

        Some(parent)
    }

    // TEST: to_string(coord!("root-A1-B2-B3")) == "root-A1-B2-B3"
    pub fn to_string(&self) -> String {
        coord_show(
            self.row_cols
                .iter()
                .map(|(r, c)| (r.get(), c.get()))
                .collect(),
        )
        .unwrap()
    }

    // TEST:
    // - row(coord!("root-A1-B2-B3")).get() == 3
    // - row(coord!("root-A1-E13-Z3")).get() == 3
    pub fn row(&self) -> NonZeroU32 {
        if let Some(last) = self.row_cols.last() {
            last.0
        } else {
            panic! {"a coordinate should always have a row, this one doesnt"}
        }
    }

    // TEST: same as above (but mutable)
    fn row_mut(&mut self) -> &mut NonZeroU32 {
        if let Some(last) = self.row_cols.last_mut() {
            &mut last.0
        } else {
            panic! {"a coordinate should always have a row, this one doesnt"}
        }
    }

    // TEST:
    // - full_row(coord!("root-A1-B2-B3")).get() == coord_row!("root-A1-B2", "3")
    // - full_row(coord!("root-A1-E13-Z3")).get() == coord_row!("root-A1-E13", "3")
    pub fn full_row(&self) -> Row {
        Row(
            self.parent()
                .expect("full_row shouldn't be called on root or meta"),
            self.row(),
        )
    }

    // TEST:
    // - row_to_string(coord!("root-A1-B2-B3")) = "root-A1-B2-3"
    pub fn row_to_string(&self) -> String {
        if let Some(parent) = self.parent() {
            info!(
                "row to str {:?}, {:?}",
                format! {"{}-{}", parent.to_string(), self.row().get()},
                self
            );
            format! {"{}-{}", parent.to_string(), self.row().get()}
        } else {
            info!(
                " row to str22 {:?}, {:?}",
                format! {"{}", self.row().get()},
                self
            );
            format! {"{}", self.row().get()}
        }
    }

    // TEST:
    // - col(coord!("root-A1-B2-B3")).get() == 2
    // - col(coord!("root-A1-E13-Z3")).get() == 26
    pub fn col(&self) -> NonZeroU32 {
        if let Some(last) = self.row_cols.last() {
            last.1
        } else {
            panic! {"a coordinate should always have a column, this one doesnt"}
        }
    }

    // TEST: same as above (but mutable)
    fn col_mut(&mut self) -> &mut NonZeroU32 {
        if let Some(last) = self.row_cols.last_mut() {
            &mut last.1
        } else {
            panic! {"a coordinate should always have a column, this one doesnt"}
        }
    }
    // TEST: same as above (but mutable)
    pub fn full_col(&self) -> Col {
        Col(
            self.parent()
                .expect("full_col shouldn't be called on root or meta"),
            self.col(),
        )
    }

    pub fn col_to_string(&self) -> String {
        if let Some(parent) = self.parent() {
            format! {"{}-{}", parent.to_string(), from_u32(self.col().get() + 64).unwrap()}
        } else {
            format! {"{}", from_u32(self.col().get() + 64).unwrap()}
        }
    }

    // if a cell is the parent, grandparent,..., (great xN)-grandparent of another
    // Optinoally returns: Some(N) if true (including N=0 if sibling),
    // or None if false
    // Korede Check this
    fn is_n_parent(&self, other: &Self) -> Option<i32> {
        // info!("n parent 11111123334444 {:?}, {:?}", self, other);
        if self.row_cols.len() > other.row_cols.len() {
            return None;
        }

        let mut n = 0;
        for (a, b) in self.row_cols.iter().zip(other.row_cols.iter()) {
            if a != b {
                break;
            }
            n += 1;
        }
        Some(n)
    }
    // (3, 2) (2,2)
    //"root-A1-B2-B3"
    //"root-A1-B2-B2"
    pub fn neighbor_above(&self) -> Option<Coordinate> {
        info!("selllfff {:?}", self);
        let mut new_row_col = self.clone().row_cols;
        if let Some(last) = new_row_col.last_mut() {
            if last.0.get() > 1 {
                *last = (
                    /* row */ NonZeroU32::new(last.0.get() - 1).unwrap(),
                    /* column */ last.1,
                );
                return Some(Coordinate {
                    row_cols: new_row_col,
                });
            }
        }

        None
    }
    //"root-A1-B2-B3"
    //"root-A1-B2-B4"
    pub fn neighbor_below(&self) -> Option<Coordinate> {
        let mut new_row_col = self.clone().row_cols;
        if let Some(last) = new_row_col.last_mut() {
            *last = (
                /* row */ NonZeroU32::new(last.0.get() + 1).unwrap(),
                /* column */ last.1,
            );
            return Some(Coordinate {
                row_cols: new_row_col,
            });
        }

        None
    }

    pub fn neighbor_left(&self) -> Option<Coordinate> {
        let mut new_row_col = self.clone().row_cols;
        if let Some(last) = new_row_col.last_mut() {
            if last.1.get() > 1 {
                *last = (
                    /* row */ last.0,
                    /* column */ NonZeroU32::new(last.1.get() - 1).unwrap(),
                );
                return Some(Coordinate {
                    row_cols: new_row_col,
                });
            }
        }

        None
    }

    pub fn neighbor_right(&self) -> Option<Coordinate> {
        let mut new_row_col = self.clone().row_cols;
        if let Some(last) = new_row_col.last_mut() {
            *last = (
                /* row */ last.0,
                /* column */ NonZeroU32::new(last.1.get() + 1).unwrap(),
            );
            return Some(Coordinate {
                row_cols: new_row_col,
            });
        }

        None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct Row(
    /* parent */ pub Coordinate,
    /* row_index */ pub NonZeroU32,
);

impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl Eq for Row {}

#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct Col(
    /* parent */ pub Coordinate,
    /* col_index */ pub NonZeroU32,
);

impl PartialEq for Col {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl Eq for Col {}

// macro for easily defining a coordinate
// either absolutely or relative to it's parent coordinate
// TODO: this code is messy, can be optimized more later
#[macro_export]
macro_rules! coord {
    ( $coord_str:tt ) => {{
        let mut fragments: Vec<(NonZeroU32, NonZeroU32)> = Vec::new();

        let pairs = CoordinateParser::parse(Rule::coordinate, $coord_str)
            .unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            match pair.as_rule() {
                Rule::special if pair.as_str() == "root" => {
                    fragments.push(non_zero_u32_tuple((1, 1)));
                }
                Rule::special if pair.as_str() == "meta" => {
                    fragments.push(non_zero_u32_tuple((1, 2)));
                }
                Rule::fragment => {
                    let mut fragment: (u32, u32) = (0, 0);
                    for inner_pair in pair.into_inner() {
                        match inner_pair.as_rule() {
                            // COLUMN
                            Rule::alpha => {
                                let mut val: u32 = 0;
                                for ch in inner_pair.as_str().to_string().chars() {
                                    val += (ch as u32) - 64;
                                }
                                fragment.1 = val;
                            }
                            // ROW
                            Rule::digit => {
                                fragment.0 = inner_pair.as_str().parse::<u32>().unwrap();
                            }
                            _ => unreachable!(),
                        };
                    }
                    fragments.push(non_zero_u32_tuple(fragment));
                }
                _ => unreachable!(),
            }
        }

        Coordinate {
            row_cols: fragments,
        }
    }};
}

#[macro_export]
macro_rules! coord_col {
    ( $parent_str:tt, $col_str:tt ) => {{
        let mut col: u32 = 0;
        for ch in $col_str.to_string().chars() {
            col += (ch as u32) - 64;
        }

        Col(coord!($parent_str), NonZeroU32::new(col).unwrap())
    }};
}

#[macro_export]
macro_rules! coord_row {
    ( $parent_str:tt, $row_str:tt ) => {{
        let row: u32 = $row_str.parse::<u32>().unwrap();

        Row(coord!($parent_str), NonZeroU32::new(row).unwrap())
    }};
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_row() {
        assert_eq!(coord!("root-A1-B2-B3").row().get(), 3);
    }

    #[test]
    // - parent.row_cols.len() == result.row_cols.len() - 1
    fn test_childOf() {
        // let param = coord!().row_cols.len();
        // assert_eq!(
        //     coord!("root-A1-B2-B3").child_of().len(),
        //     coord!("root-A1-B2-B3").len() + 1
        // )
        unimplemented!();
    }

    #[test]
    fn test_parent() {
        assert_eq!(coord!("root").parent(), None);
        assert_eq!(coord!("meta").parent(), None);
        // assert_eq!(coord!().parent().len(), coord!().parent().len() + 1); //Check with korede for parent

        // unimplemented!();
    }

    #[test]
    fn test_to_string() {
        assert_eq!(coord!("root-A1-B2-B3").to_string(), "root-A1-B2-B3");
    }

    #[test]
    fn test_row_mut() {
        // assert_eq!(coord!("root-A1-B2-B3").row().get(), 3); // Test for mutability required
        unimplemented!()
    }

    #[test]
    fn test_full_row() {
        assert_ne!(
            coord!("root-A1-B2-B3").full_row(),
            coord_row!("root-A1-B1", "3")
        );
        assert_eq!(
            coord!("root-A1-B2-B3").full_row(),
            coord_row!("root-A1-B2", "3")
        );
    }

    #[test]
    fn test_row_to_string() {
        // - row_to_string(coord!("root-A1-B2-B3")) = "root-A1-B2-3"
        assert_eq!(coord!("root-A1-B2-B3").row_to_string(), "root-A1-B2-3");
        // info!("{:?}", coord!("root").row_to_string())
        assert_eq!(coord!("root").row_to_string(), "1");
        assert_eq!(coord!("meta").row_to_string(), "1");
        // unimplemented!();
    }

    #[test]
    fn test_is_n_parent() {
        unimplemented!();
    }

    #[test]
    fn test_neighbor_above() {
        //"root-A1-B2-B3"
        //"root-A1-B2-B2"
        assert_eq!(
            coord!("root-A1-B2-B3").neighbor_above().unwrap(),
            coord!("root-A1-B2-B2")
        );
        assert_ne!(
            coord!("root-A1-B2-B3").neighbor_above().unwrap(),
            coord!("root-A1-B2-B1")
        );
        // unimplemented!();
    }

    #[test]
    fn test_neighbor_below() {
        assert_eq!(
            coord!("root-A1-B2-B3").neighbor_below().unwrap(),
            coord!("root-A1-B2-B4")
        );
        assert_ne!(
            coord!("root-A1-B2-B3").neighbor_below().unwrap(),
            coord!("root-A1-B2-B6")
        );
        // unimplemented!();
    }

    #[test]
    fn test_neighbor_left() {
        assert_eq!(
            coord!("root-A1-B2-B3").neighbor_left().unwrap(),
            coord!("root-A1-B2-A3")
        );
        assert_ne!(
            coord!("root-A1-B2-B3").neighbor_left().unwrap(),
            coord!("root-A1-B2-B6")
        );
        // unimplemented!();
    }

    #[test]
    fn test_neighbor_right() {
        assert_eq!(
            coord!("root-A1-B2-B3").neighbor_right().unwrap(),
            coord!("root-A1-B2-C3")
        );
        assert_ne!(
            coord!("root-A1-B2-B3").neighbor_right().unwrap(),
            coord!("root-A1-B2-C6")
        );
        // unimplemented!();
    }
}
