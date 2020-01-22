use std::collections::HashMap;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::option::Option;
use yew::{html, ChangeData, Component, ComponentLink, Html, ShouldRender, InputData};
use yew::events::{IKeyboardEvent, ClickEvent, KeyPressEvent};
use yew::services::{ConsoleService};
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};
use yew::virtual_dom::{VList};
use pest::Parser;
use std::fs;
use std::panic;

use crate::grammar::{Grammar, Kind, Interactive};
use crate::style::Style;
use crate::coordinate::{Coordinate, Row, Col};
use crate::session::Session;
use crate::utils::{
    resize_cells, 
    resize, 
    apply_definition_grammar, 
    non_zero_u32_tuple, 
    move_grammar
};
use crate::{
    row_col_vec, 
    coord, 
    coord_row, 
    coord_col
};


#[derive(Parser)]
#[grammar = "coordinate.pest"]
pub struct CoordinateParser;


// Model contains the entire state of the application
#[derive(Debug)]
pub struct Model {
    // model holds a direct reference to the topmost root A1 and meta A2 grammars
    // these two grammars are excluded from the grammar Map
    root: Grammar,
    meta: Grammar,

    // the view that the UI treats as the topmost grammar to start rendering from.
    view_root: Coordinate,

    pub grammars: HashMap</*Key*/ Coordinate, /*Value*/ Grammar>,
    value: String,  // what is this?????
    active_cell: Option<Coordinate>,
    suggestions: Vec<Coordinate>,

    pub col_widths: HashMap<Col, f64>,
    pub row_heights: HashMap<Row, f64>,

    // tabs correspond to sessions
    tabs: Vec<String>,
    current_tab: i32,

    // side menus
    side_menus: Vec<SideMenu>,
    open_side_menu: Option<i32>,

    console: ConsoleService,
    reader: ReaderService,

    link: ComponentLink<Model>,
    tasks: Vec<ReaderTask>,
}

#[derive(Debug)]
struct SideMenu {
    name: String,
    icon_path: String,
}

// ACTIONS
// Triggered in the view, sent to update function
pub enum Action {
    // Do nothing
    Noop,

    // Change string value of Input grammar
    ChangeInput(Coordinate, /* new_value: */ String),

    // Show suggestions dropdown at Coordinate based on query
    ShowSuggestions(Coordinate, /* query: */ String),

    SetActiveCell(Coordinate),

    DoCompletion(/* source: */ Coordinate, /* destination */ Coordinate),

    SetActiveMenu(Option<i32>),

    ReadSession(/* filename: */ File),

    LoadSession(FileData),

    SaveSession(),

    // Grid Operations
    AddNestedGrid(Coordinate, (u32 /*rows*/, u32 /*cols*/)),

    InsertRow,
    InsertCol,

    // Alerts and stuff
    Alert(String),
}

impl Model {
    fn load_session(&mut self, session: Session) {
        self.root = session.root;
        self.meta = session.meta;
        self.grammars = session.grammars;
    }

    fn to_session(&self) -> Session {
        Session {
            root: self.root.clone(),
            meta: self.meta.clone(),
            grammars: self.grammars.clone(),
        }
    }

    fn get_style(&self, coord: &Coordinate) -> String {
        let grammar = self.grammars.get(coord).expect("no grammar with this coordinate");
        if coord.row_cols.len() == 1 {  // root or meta
            return grammar.style(coord);
        }
        if let Kind::Grid(_) = grammar.kind {
            return format!{
                "{}\nwidth: fit-content;\nheight: fit-content;\n",
                grammar.style(coord),
            };
        }
        let col_width = self.col_widths.get(&coord.full_col()).unwrap_or(&90.0);
        let row_height = self.row_heights.get(&coord.full_row()).unwrap_or(&30.0);
        format!{
            "{}\nwidth: {}px;\nheight: {}px;\n",
            grammar.style(coord), col_width, row_height,
        }
    }

    fn query_parent(&self, coord_parent: Coordinate) -> Vec<Coordinate> {
        self.grammars.keys().clone().filter_map(|k| {
            if k.parent() == Some(coord_parent.clone()) {
                Some(k.clone())
            } else { None }
        }).collect()
    }

    fn query_col(&self, coord_col: Col) -> Vec<Coordinate> {
        self.grammars.keys().clone().filter_map(|k| {
            if k.row_cols.len() == 1 /* ignore root & meta */ {
                None
            } else if k.full_col() == coord_col {
                Some(k.clone())
            } else { None }
        }).collect()
    }

    fn query_row(&self, coord_row: Row) -> Vec<Coordinate> {
        self.grammars.keys().clone().filter_map(|k| {
            if k.row_cols.len() == 1 /* ignore root & meta */ {
                None
            } else if k.full_row() == coord_row {
                Some(k.clone())
            } else { None }
        }).collect()
    }
}

impl Component for Model {
    type Message = Action;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let root_grammar = Grammar {
            name: "root".to_string(),
            style: Style::default(),
            kind: Kind::Grid(row_col_vec![ (1,1), (2,1), (3,1), (1,2), (2,2), (3,2) ]),
        };
        let meta_grammar = Grammar {
            name: "meta".to_string(),
            style: Style::default(),
            kind: Kind::Grid(row_col_vec![ (1,1), (2,1) ]),
        };
        let mut m = Model {
            root: root_grammar.clone(),
            meta: meta_grammar.clone(),
            view_root: coord!("root"),
            grammars: hashmap! {
                coord!("root")    => root_grammar.clone(),
                coord!("root-A1") => Grammar::default(),
                coord!("root-A2") => Grammar::default(),
                coord!("root-A3") => Grammar::default(),
                coord!("root-B1") => Grammar::default(),
                coord!("root-B2") => Grammar::default(),
                coord!("root-B3") => Grammar::default(),
                coord!("meta")    => meta_grammar.clone(),
                coord!("meta-A1") => Grammar::suggestion("js grammar".to_string(), "This is js".to_string()),
                coord!("meta-A2") => Grammar::suggestion("java grammar".to_string(), "This is java".to_string()),
            },
            col_widths: hashmap! {
               coord_col!("root","A") => 90.0,
               coord_col!("root","B") => 90.0,
            },
            row_heights: hashmap! {
               coord_row!("root","1") => 30.0,
               coord_row!("root","2") => 30.0,
               coord_row!("root","3") => 30.0,
            },
            value: String::new(),
            active_cell: Some(coord!("root-A1")),
            suggestions: vec![ coord!("meta-A1"), coord!("meta-A2"), coord!("meta-A3") ],
            // suggestions: vec![],

            console: ConsoleService::new(),
            reader: ReaderService::new(),

            tabs: vec![
               "Session 1".to_string(),
               "My Session".to_string(),
               "Session 100".to_string(),
            ],

            current_tab: 0,

            side_menus: vec![
                SideMenu {
                    name: "Home".to_string(),
                    icon_path: "assets/logo.png".to_string(),
                },
                SideMenu {
                    name: "File Explorer".to_string(),
                    icon_path: "assets/folder_icon.png".to_string(),
                },
                SideMenu {
                    name: "Settings".to_string(),
                    icon_path: "assets/settings_icon.png".to_string(),
                },
                SideMenu {
                    name: "Info".to_string(),
                    icon_path: "assets/info_icon.png".to_string(),
                },
            ],
            open_side_menu: None,

            link,
            tasks: vec![],
        };
        apply_definition_grammar(&mut m, coord!("meta-A3"));
        m
    }

    // The update function is split into sub-update functions that 
    // are specifc to each EventType
    fn update(&mut self, event_type: Self::Message) -> ShouldRender {
        match event_type {
            Action::Noop => false,

            Action::Alert(message) => {
                self.console.log(&message);
                // TODO: make this into a more visual thing
                false
            }

            Action::ChangeInput(coord, new_value) => {
                let old_grammar = self.grammars.get_mut(&coord);
                match old_grammar {
                    Some(g @ Grammar { kind: Kind::Text(_), .. }) => {
                        self.console.log(&new_value);
                        g.kind = Kind::Text(new_value);
                    },
                    _ => ()
                }
                false
            }

            Action::ShowSuggestions(coord, query) => {
                false
            }

            Action::SetActiveCell(coord) => {
                self.active_cell = Some(coord);
                true
            }

            Action::DoCompletion(source_coord, dest_coord) => {
                move_grammar(&mut self.grammars, source_coord, dest_coord.clone());
                resize_cells(&mut self.grammars, dest_coord);
                true
            }

            Action::SetActiveMenu(active_menu) => {
                self.open_side_menu = active_menu;
                true
            }

            Action::ReadSession(file) => {
                let callback = self.link.callback(Action::LoadSession);
                let task = self.reader.read_file(file, callback);
                self.tasks.push(task);
                false
            }

            Action::LoadSession(file_data) => {
                let session : Session = serde_json::from_str(format!{"{:?}", file_data}.deref()).unwrap();
                self.load_session(session);
                true
            }

            Action::SaveSession() => {
                let session = self.to_session();
                let j = serde_json::to_string(&session);
                let filename = "testfile";
                fs::write(filename, j.unwrap()).expect("Unable to write to file!");
                false
            }

            Action::AddNestedGrid(coord, (rows, cols)) => {
                let (r, c) = non_zero_u32_tuple((rows, cols));
                let grammar = Grammar::as_grid(r, c);
                if let Kind::Grid(sub_coords) = grammar.clone().kind {
                    self.active_cell = sub_coords.first().map(|c| Coordinate::child_of(&coord, *c));
                    for sub_coord in sub_coords {
                        let new_coord = Coordinate::child_of(&coord, sub_coord);
                        self.grammars.insert(new_coord.clone(), Grammar::default());
                        // initialize row & col heights as well
                        if !self.row_heights.contains_key(&new_coord.clone().full_row()) {
                            self.row_heights.insert(new_coord.clone().full_row(), 30.0);
                        }
                        if !self.col_widths.contains_key(&new_coord.clone().full_col()) {
                            self.col_widths.insert(new_coord.clone().full_col(), 90.0);
                        }
                    }
                }
                if let Some(parent) = Coordinate::parent(&coord).and_then(|p| self.grammars.get_mut(&p)) {
                    parent.kind = grammar.clone().kind; // make sure the parent gets set to Kind::Grid
                }
                self.grammars.insert(coord.clone(), grammar);
                resize(self, coord,
                    (rows as f64) * (/* default row height */ 30.0),
                    (cols as f64) * (/* default col width */ 90.0));
                true
            }
            Action::InsertCol => {
                if let Some(coord) = self.active_cell.clone() {
                    // find the bottom-most coord
                    let mut right_most_coord = coord.clone();
                    while let Some(right_coord) = right_most_coord.neighbor_right() {
                        if self.grammars.contains_key(&right_coord) {
                            right_most_coord = right_coord;
                        } else { break }
                    }

                    let right_most_col_coords = self.query_col(right_most_coord.full_col());
                    let new_col_coords = right_most_col_coords.iter().map(|c| {
                        (c.row(), NonZeroU32::new(c.col().get() + 1).unwrap())
                    });

                    let parent = coord.parent().unwrap();
                    if let Some(Grammar{ kind: Kind::Grid(sub_coords), name, style }) = self.grammars.get(&parent) {
                        let mut new_sub_coords = sub_coords.clone();
                        let mut grammars = self.grammars.clone();
                        for c in new_col_coords {
                            grammars.insert(Coordinate::child_of(&parent.clone(), c), Grammar::default());
                            new_sub_coords.push(c);
                        }
                        grammars.insert(parent, Grammar {
                            kind: Kind::Grid(new_sub_coords.clone()),
                            name: name.clone(),
                            style: style.clone()
                        });
                        self.grammars = grammars;
                    }
                }
                true
            }
            Action::InsertRow => {
                if let Some(coord) = self.active_cell.clone() {
                    // find the bottom-most coord
                    let mut bottom_most_coord = coord.clone();
                    while let Some(below_coord) = bottom_most_coord.neighbor_below() {
                        if self.grammars.contains_key(&below_coord) {
                            bottom_most_coord = below_coord;
                        } else { break }
                    }

                    let bottom_most_row_coords = self.query_row(bottom_most_coord.full_row());
                    let new_row_coords = bottom_most_row_coords.iter().map(|c| {
                        (NonZeroU32::new(c.row().get() + 1).unwrap(), c.col())
                    });

                    let parent = coord.parent().unwrap();
                    if let Some(Grammar{ kind: Kind::Grid(sub_coords), name, style }) = self.grammars.get(&parent) {
                        let mut new_sub_coords = sub_coords.clone();
                        let mut grammars = self.grammars.clone();
                        for c in new_row_coords {
                            grammars.insert(Coordinate::child_of(&parent.clone(), c), Grammar::default());
                            new_sub_coords.push(c);
                        }
                        grammars.insert(parent, Grammar {
                            kind: Kind::Grid(new_sub_coords.clone()),
                            name: name.clone(),
                            style: style.clone()
                        });
                        self.grammars = grammars;
                    }
                }
                true
            }
        }
    }

    fn view(&self) -> Html {

        let active_cell = self.active_cell.clone();
        html! {
            <div>

                { view_side_nav(&self) }

                { view_menu_bar(&self) }

                { view_tab_bar(&self) }

                <div class="main">
                    <div id="grammars" class="grid-wrapper" onkeypress=self.link.callback(move |e : KeyPressEvent| {
                        if e.key() == "g" && e.ctrl_key() {
                            if let Some(coord) = active_cell.clone() {
                                return Action::AddNestedGrid(coord.clone(), (3, 3));
                            }
                        }
                        Action::Noop
                    })>
                        { view_grammar(&self, coord!{"root"}) }
                    </div>
                </div>
            </div>
        }
    }
}


fn view_side_nav(m: &Model) -> Html {
    let mut side_menu_nodes = VList::new();
    let mut side_menu_section = html! { <></> };
    for (index, side_menu) in m.side_menus.iter().enumerate() {
        if Some(index as i32) == m.open_side_menu {
            side_menu_nodes.add_child(html! {
                <button class="active-menu" onclick=m.link.callback(|e| Action::SetActiveMenu(None))>
                    <img 
                        src={side_menu.icon_path.clone()} 
                        width="40px" alt={side_menu.name.clone()}>
                    </img>
                </button>
            });

            side_menu_section = view_side_menu(m, side_menu);
        } else {
            side_menu_nodes.add_child(html! {
                <button onclick=m.link.callback(move |e| Action::SetActiveMenu(Some(index as i32)))>
                    <img 
                        src={side_menu.icon_path.clone()} 
                        width="40px" alt={side_menu.name.clone()}>
                    </img>
                </button>
            });
        }
    }

    html! {
        <div class="sidenav">
            { side_menu_nodes }

            { side_menu_section }
        </div>
    }
}

fn view_side_menu(m: &Model, side_menu: &SideMenu) -> Html {
    match side_menu.name.deref() {
        "Home" => {
            html! {
                <div class="side-menu-section">
                    {"THIS IS Home MENU"}
                </div>
            } 
        },
        "File Explorer" => {
            html! {
                <div class="side-menu-section">
                    <h1>
                        {"File Explorer"}
                    </h1>

                    <h3>{"load session"}</h3>
                    <br></br>
                    <input type="file" onchange=m.link.callback(|value| {
                        if let ChangeData::Files(files) = value {
                            if files.len() >= 1 {
                                if let Some(file) = files.iter().nth(0) {
                                    return Action::ReadSession(file);
                                }
                            } else {
                                return Action::Alert("Could not load file".to_string());
                            }
                        }
                        Action::Noop
                    })>
                    </input>

                    <h3>{"save session"}</h3>
                    <br></br>
                    <input type="file" onchange=m.link.callback(|value| {
                        if let ChangeData::Files(files) = value {
                            if files.len() >= 1 {
                                if let Some(file) = files.iter().nth(0) {
                                    return Action::SaveSession();
                                }
                            }
                        }
                        Action::Noop
                    })>
                        
                    </input>
                </div>
            } 
        },
        "Settings" => {
            html! {
                <div class="side-menu-section">
                    {"THIS IS Settings MENU"}
                </div>
            } 
        },
        "Info" => {
            html! {
                <div class="side-menu-section">
                    {"THIS IS info MENU"}
                </div>
            } 
        },

        _ => html! {<> </>}

    }
}


fn view_menu_bar(m: &Model) -> Html {
    html! {
        <div class="menu-bar horizontal-bar">
            <input 
                class="active-cell-indicator"
                disabled=true 
                // TODO: clicking on this should highlight
                // the active cell
                value={
                    if let Some(cell) = m.active_cell.clone() {
                        cell.to_string()
                    } else {
                        "".to_string()
                    }
                }>
            </input>
            <button class="menu-bar-button" onclick=m.link.callback(|_| Action::Noop) >
                { "Save" }
            </button>
            <button class="menu-bar-button">
                { "Git" }
            </button>
            <button class="menu-bar-button">
                { "Zoom In (+)" }
            </button>
            <button class="menu-bar-button">
                { "Zoom Out (-)" }
            </button>
            <button class="menu-bar-button" onclick=m.link.callback(|_| Action::InsertRow)>
                { "Insert Row" }
            </button>
            <button class="menu-bar-button" onclick=m.link.callback(|_| Action::InsertCol)>
                { "Insert Column" }
            </button>
            <button class="menu-bar-button">
                { "Delete Row" }
            </button>
            <button class="menu-bar-button">
                { "Delete Column" }
            </button>
        </div>
    }
}

fn view_tab_bar(m: &Model) -> Html {
    let mut tabs = VList::new();
    for (index, tab) in m.tabs.clone().iter().enumerate() {
        if (index as i32) == m.current_tab {
            tabs.add_child(html! {
                <button class="tab active-tab">{ tab }</button>
            });
        } else {
            tabs.add_child(html! {
                <button class="tab">{ tab }</button>
            });
        }
    }
    html! {
        <div class="tab-bar horizontal-bar">
            { tabs }
            <button class="newtab-btn">
                <span>{ "+" }</span>
            </button>
        </div>
    }
}

fn view_grammar(m: &Model, coord: Coordinate) -> Html {
    if let Some(grammar) = m.grammars.get(&coord) {
        match grammar.kind.clone() {
            Kind::Text(value) => {
                view_text_grammar(m, &coord, value)
            }
            Kind::Input(value) => {
                let is_active = m.active_cell.clone() == Some(coord.clone());
                let suggestions = m.suggestions.iter().filter_map(|suggestion_coord| {
                    if let Some(suggestion_grammar) = m.grammars.get(&suggestion_coord) {
                        Some((suggestion_coord.clone(), suggestion_grammar.clone()))
                    } else {
                        None
                    }
                }).collect();
                view_input_grammar(
                    m,
                    coord.clone(),
                    suggestions,
                    value,
                    is_active,
                )
            }
            Kind::Interactive(name, Interactive::Button()) => {
                html! {
                    <div
                        class=format!{"cell row-{} col-{}", coord.row_to_string(), coord.col_to_string()}
                        id=format!{"cell-{}", coord.to_string()}
                        style={ m.get_style(&coord) }>
                        <button>
                            { name }
                        </button>
                    </div>
                }
            }
            Kind::Interactive(name, Interactive::Slider(value, min, max)) => {
                html! {
                    <div 
                        class=format!{"cell row-{} col-{}", coord.row_to_string(), coord.col_to_string()}
                        id=format!{"cell-{}", coord.to_string()}
                        style={ m.get_style(&coord) }>
                        <input type="range" min={min} max={max} value={value}>
                            { name }
                        </input>
                    </div>
                }
            }
            Kind::Interactive(name, Interactive::Toggle(checked)) => {
                html! {
                    <div
                        class=format!{"cell row-{} col-{}", coord.row_to_string(), coord.col_to_string()}
                        id=format!{"cell-{}", coord.to_string()}
                        style={ m.get_style(&coord) }>
                        <input type="checkbox" checked={checked}>
                            { name }
                        </input>
                    </div>
                }
            }
            Kind::Grid(sub_coords) => {
                view_grid_grammar(
                    m,
                    &coord,
                    sub_coords.iter().map(|c| Coordinate::child_of(&coord, *c)).collect(),
                )
            }
        }
    } else {
        // return empty fragment
        html! { <></> }
    }
}

fn view_input_grammar(
    m: &Model,
    coord: Coordinate,
    suggestions: Vec<(Coordinate, Grammar)>,
    value: String,
    is_active: bool,
) -> Html {
    let mut suggestion_nodes = VList::new();
    let mut active_cell_class = "cell-inactive";
    if is_active {
        active_cell_class = "cell-active";
        for (s_coord, s_grammar) in suggestions {
            let c = coord.clone();
            suggestion_nodes.add_child(html! {
                <a 
                    tabindex=-1
                    onclick=m.link.callback(move |_ : ClickEvent| Action::DoCompletion(s_coord.clone(), c.clone()))>
                    { &s_grammar.name }
                </a>
            })
            
        }
    }
    let suggestions = html!{
        <div class="suggestion-content">
            { suggestion_nodes }
        </div>
    };

    let new_active_cell = coord.clone();

    html! {
        <div
            class=format!{"cell suggestion row-{} col-{}", coord.row_to_string(), coord.col_to_string()}
            id=format!{"cell-{}", coord.to_string()}
            style={ m.get_style(&coord) }>
            <input 
                class={ format!{ "cell-data {}", active_cell_class } }
                value=value
                oninput=m.link.callback(move |e : InputData| Action::ChangeInput(coord.clone(), e.value))
                onclick=m.link.callback(move |_ : ClickEvent| Action::SetActiveCell(new_active_cell.clone()))>
            </input>
            
            { suggestions }
        </div>
    }
}

fn view_text_grammar(m: &Model, coord: &Coordinate, value : String) -> Html {
    html! {
        <div
            class=format!{"cell text row-{} col-{}", coord.row_to_string(), coord.col_to_string()}
            id=format!{"cell-{}", coord.to_string()}
            style={ m.get_style(&coord) }>
            { value }
        </div>
    }
}

fn view_grid_grammar(m: &Model, coord: &Coordinate, sub_coords: Vec<Coordinate>) -> Html {
    let mut nodes = VList::new();

    for c in sub_coords {
        nodes.add_child(view_grammar(m, c.clone()));
    }

    html! {
        <div
            class=format!{"cell grid row-{} col-{}", coord.row_to_string(), coord.col_to_string()}
            id=format!{"cell-{}", coord.to_string()}
            style={ m.get_style(&coord) }>
            { nodes }
        </div>
    }
}
