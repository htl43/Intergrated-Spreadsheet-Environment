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
use node_sys::fs as node_fs;
use node_sys::Buffer;
use js_sys::{
    JsString,
    Function
};
use electron_sys::{ipc_renderer};
use wasm_bindgen::JsValue;
use stdweb::web::{document, HtmlElement, IElement, IHtmlElement, INode, IParentNode, INonElementParentNode};
use stdweb::unstable::{TryFrom, TryInto};

use crate::grammar::{Grammar, Kind, Interactive};
use crate::style::Style;
use crate::coordinate::{Coordinate, Row, Col};
use crate::session::Session;
use crate::util::{
    resize_cells, 
    resize, 
    apply_definition_grammar, 
    non_zero_u32_tuple, 
    move_grammar
};
use crate::view::{
    view_grammar,
    view_menu_bar,
    view_side_nav,
    view_tab_bar
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
    view_root: Coordinate,

    pub active_cell: Option<Coordinate>,
    pub suggestions: Vec<Coordinate>,

    pub col_widths: HashMap<Col, f64>,
    pub row_heights: HashMap<Row, f64>,

    // tabs correspond to sessions
    pub tabs: Vec<Session>,
    pub current_tab: usize,

    // side menus
    pub side_menus: Vec<SideMenu>,
    pub open_side_menu: Option<i32>,

    console: ConsoleService,
    reader: ReaderService,

    pub link: ComponentLink<Model>,
    tasks: Vec<ReaderTask>,
}

#[derive(Debug)]
pub struct SideMenu {
    pub name: String,
    pub icon_path: String,
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

    SetSessionTitle(String),
    ReadDriverFiles(Vec<File>),
    LoadDriverMainFile(FileData),
    UploadDriverMiscFile(FileData),

    // Grid Operations
    AddNestedGrid(Coordinate, (u32 /*rows*/, u32 /*cols*/)),

    InsertRow,
    InsertCol,

    // Alerts and stuff
    Alert(String),
}

impl Model {

    // only use this if you need a COPY of the current session
    // i.e. not changing its values
    pub fn to_session(&self) -> Session {
        return self.tabs[self.current_tab].clone();
    }

    fn load_session(&mut self, session: Session) {
        self.tabs[self.current_tab].root = session.root;
        self.tabs[self.current_tab].meta = session.meta;
        self.tabs[self.current_tab].grammars = session.grammars;
    }

    fn query_parent(&self, coord_parent: Coordinate) -> Vec<Coordinate> {
        self.to_session().grammars.keys().clone().filter_map(|k| {
            if k.parent() == Some(coord_parent.clone()) {
                Some(k.clone())
            } else { None }
        }).collect()
    }

    fn query_col(&self, coord_col: Col) -> Vec<Coordinate> {
        self.to_session().grammars.keys().clone().filter_map(|k| {
            if k.row_cols.len() == 1 /* ignore root & meta */ {
                None
            } else if k.full_col() == coord_col {
                Some(k.clone())
            } else { None }
        }).collect()
    }

    fn query_row(&self, coord_row: Row) -> Vec<Coordinate> {
        self.to_session().grammars.keys().clone().filter_map(|k| {
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
            view_root: coord!("root"),
            col_widths: hashmap! {
               coord_col!("root","A") => 90.0,
               coord_col!("root","B") => 90.0,
            },
            row_heights: hashmap! {
               coord_row!("root","1") => 30.0,
               coord_row!("root","2") => 30.0,
               coord_row!("root","3") => 30.0,
            },
            active_cell: Some(coord!("root-A1")),
            suggestions: vec![ coord!("meta-A1"), coord!("meta-A2"), coord!("meta-A3") ],
            // suggestions: vec![],

            console: ConsoleService::new(),
            reader: ReaderService::new(),

            tabs: vec![
                Session{
                    title: "my session".to_string(),
                    root: root_grammar.clone(),
                    meta: meta_grammar.clone(),
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
                    }
                }
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
                let old_grammar = self.tabs[self.current_tab].grammars.get_mut(&coord);
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
                move_grammar(&mut self.tabs[self.current_tab].grammars, source_coord, dest_coord.clone());
                resize_cells(&mut self.tabs[self.current_tab].grammars, dest_coord);
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
                let j = serde_json::to_string(&session.clone());
                let filename = session.title.to_string();
                let jsfilename = JsString::from(filename);
                let jsbuffer = Buffer::from_string(&JsString::from(j.unwrap()), None);
                let jscallback = Function::new_no_args("{}");
                node_fs::append_file(&jsfilename, &jsbuffer, None, &jscallback);
                false
            }

            Action::SetSessionTitle(name) => {
                // cant use to_session() here since we're actually changing it
                self.tabs[self.current_tab].title = name;
                true
            }

            Action::ReadDriverFiles(files_list) => {
                // Get the main file and miscellaneous/additional files from the drivers list 
                let (main_file, misc_files) = {
                    let (main_file_as_vec, misc_files) : (Vec<File>, Vec<File>) = files_list.iter().fold((Vec::new(), Vec::new()), |accum, file| { 
                        // Iter::partition is used to divide a list into two given a certain condition.
                        // Here the condition here is to separate the main file of the driver from the
                        // addditional ones, where the main file's path looks like
                        // '{directory_name}/{file_name}.js' and the directory_name == file_name
                        let mut new_accum = accum.clone();
                        // use `webkitRelativePath` as the `name`, if it's available
                        // we'll call out to regular JS to do this using the `js!` macro.
                        // Note that yew::services::reader::File::name() calls "file.name" under the
                        // hood (https://docs.rs/stdweb/0.4.20/src/stdweb/webapi/file.rs.html#23)
                        let full_file_name : String = js!(
                            if (!!@{&file}.webkitRelativePath) {
                                return @{&file}.webkitRelativePath;
                            } else {
                                console.log("couldn't get relative path of file: ", @{&file}.name);
                                return @{&file}.name; // equivalent to yew::services::reader::File::name()
                            }
                        ).try_into().unwrap();
                        let path_parts : Vec<&str> = full_file_name.split("/").collect();
                        match (path_parts.first(), path_parts.last(), path_parts.len()) {
                            (Some(directory), Some(file_name), 2) if format!{"{}.js", directory} == file_name.to_string() => {
                                if new_accum.0.len() == 0 {
                                    new_accum.0.push(file.clone());
                                } else {
                                    panic!("[Action::ReadDriverFiles]: there shouldn't be more than 1 main file in the driver directory")
                                }
                            },
                            _ => {
                                new_accum.1.push(file.clone());
                            },
                        };
                        new_accum
                    });
                    // the `partition` call above gives us a tuple of two Vecs (Vec, Vec) where the first Vec 
                    // should have only one element, so we'll convert it to a (Vec::Item, Vec).
                    // If this has an error, then there's something wrong with how the driver
                    // directory is organized.
                    (main_file_as_vec.first().unwrap().clone(), misc_files.clone())
                };

                // upload misc files so they can be served by electron to be used by main driver file
                let upload_callback = self.link.callback(|file_data| Action::UploadDriverMiscFile(file_data));
                for file in misc_files {
                    let task = self.reader.read_file(file, upload_callback.clone());
                    self.tasks.push(task);
                }

                // Load main driver file. After this task has been scheduled and executed, the
                // driver is ready for use.
                self.tasks.push(
                    self.reader.read_file(main_file, 
                        self.link.callback(Action::LoadDriverMainFile)));

                false
            }

            Action::UploadDriverMiscFile(file_data) => {
                // Here, we use some electron APIs to call out to the main process in JS.
                // For this, we use the `electron_sys` library which is pretty experimental but
                // feature complete.
                // See here for documentation how to communicate between the main and renderer proess in Electron:
                //   https://www.tutorialspoint.com/electron/electron_inter_process_communication.htm
                // And here, for the documentation for the electon_sys Rust bindings for electron.ipcRenderer:
                //   https://docs.rs/electron-sys/0.4.0/electron_sys/struct.IpcRenderer.html 
                
                let args : [JsValue; 2] = [
                    JsValue::from_str(file_data.name.deref()),
                    JsValue::from_str(std::str::from_utf8(&file_data.content).unwrap()),
                ];
                ipc_renderer.send_sync("upload-driver-misc-file", Box::new(args));
                false
            }

            Action::LoadDriverMainFile(main_file_data) => {
                info!{"Loading Driver: {}", &main_file_data.name};
                let file_contents = std::str::from_utf8(&main_file_data.content).unwrap();
                // dump file contents into script tag and attach to the DOM
                let script = document().create_element("script").unwrap();
                script.set_text_content(file_contents);
                script.set_attribute("type", "text/javascript");
                script.set_attribute("class", "ise-driver");
                script.set_attribute("defer", "true");
                let head = document().query_selector("head").unwrap().unwrap();
                head.append_child(&script);
                true
            }


            Action::AddNestedGrid(coord, (rows, cols)) => {
                let (r, c) = non_zero_u32_tuple((rows, cols));
                let grammar = Grammar::as_grid(r, c);
                if let Kind::Grid(sub_coords) = grammar.clone().kind {
                    self.active_cell = sub_coords.first().map(|c| Coordinate::child_of(&coord, *c));
                    for sub_coord in sub_coords {
                        let new_coord = Coordinate::child_of(&coord, sub_coord);
                        self.tabs[self.current_tab].grammars.insert(new_coord.clone(), Grammar::default());
                        // initialize row & col heights as well
                        if !self.row_heights.contains_key(&new_coord.clone().full_row()) {
                            self.row_heights.insert(new_coord.clone().full_row(), 30.0);
                        }
                        if !self.col_widths.contains_key(&new_coord.clone().full_col()) {
                            self.col_widths.insert(new_coord.clone().full_col(), 90.0);
                        }
                    }
                }
                if let Some(parent) = Coordinate::parent(&coord).and_then(|p| self.tabs[self.current_tab].grammars.get_mut(&p)) {
                    parent.kind = grammar.clone().kind; // make sure the parent gets set to Kind::Grid
                }
                self.tabs[self.current_tab].grammars.insert(coord.clone(), grammar);
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
                        if self.to_session().grammars.contains_key(&right_coord) {
                            right_most_coord = right_coord;
                        } else { break }
                    }

                    let right_most_col_coords = self.query_col(right_most_coord.full_col());
                    let new_col_coords = right_most_col_coords.iter().map(|c| {
                        (c.row(), NonZeroU32::new(c.col().get() + 1).unwrap())
                    });

                    let parent = coord.parent().unwrap();
                    if let Some(Grammar{ kind: Kind::Grid(sub_coords), name, style }) = self.to_session().grammars.get(&parent) {
                        let mut new_sub_coords = sub_coords.clone();
                        let mut grammars = self.to_session().grammars.clone();
                        for c in new_col_coords {
                            grammars.insert(Coordinate::child_of(&parent.clone(), c), Grammar::default());
                            new_sub_coords.push(c);
                        }
                        grammars.insert(parent, Grammar {
                            kind: Kind::Grid(new_sub_coords.clone()),
                            name: name.clone(),
                            style: style.clone()
                        });
                        self.tabs[self.current_tab].grammars = grammars;
                    }
                }
                true
            }
            Action::InsertRow => {
                if let Some(coord) = self.active_cell.clone() {
                    // find the bottom-most coord
                    let mut bottom_most_coord = coord.clone();
                    while let Some(below_coord) = bottom_most_coord.neighbor_below() {
                        if self.tabs[self.current_tab].grammars.contains_key(&below_coord) {
                            bottom_most_coord = below_coord;
                        } else { break }
                    }

                    let bottom_most_row_coords = self.query_row(bottom_most_coord.full_row());
                    let new_row_coords = bottom_most_row_coords.iter().map(|c| {
                        (NonZeroU32::new(c.row().get() + 1).unwrap(), c.col())
                    });

                    let parent = coord.parent().unwrap();
                    if let Some(Grammar{ kind: Kind::Grid(sub_coords), name, style }) = self.tabs[self.current_tab].grammars.get(&parent) {
                        let mut new_sub_coords = sub_coords.clone();
                        let mut grammars = self.tabs[self.current_tab].grammars.clone();
                        for c in new_row_coords {
                            grammars.insert(Coordinate::child_of(&parent.clone(), c), Grammar::default());
                            new_sub_coords.push(c);
                        }
                        grammars.insert(parent, Grammar {
                            kind: Kind::Grid(new_sub_coords.clone()),
                            name: name.clone(),
                            style: style.clone()
                        });
                        self.tabs[self.current_tab].grammars = grammars;
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
