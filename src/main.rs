use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    collections::VecDeque,
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use chrono::{self, Utc};

extern crate savefile;
use savefile::prelude::*;

#[macro_use]
extern crate savefile_derive;

#[derive(Clone)]
struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected(&self) -> Option<usize> {
        match self.state.selected() {
            Some(i) => return Some(i),
            None => return None,
        };
    }

    fn select_first(&mut self) {
        self.state.select(Some(0));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

#[derive(PartialEq, Clone)]
enum Windows {
    ACTIONS,
    NOTES,
    WRITER,
}

#[derive(Clone)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Savefile, Debug, Clone)]
struct Note {
    title: String,
    text: String,
    timestamp: String,
}

#[derive(Savefile, Debug)]
struct SavedNotes {
    notes: Vec<Note>,
}

#[derive(Clone)]
struct MenuAction<'a> {
    stateful: StatefulList<(&'a str, usize)>,
}

impl MenuAction<'_> {
    fn new(actions: Vec<&str>) -> MenuAction {
        let mut x: usize = 0;
        let vec = actions
            .iter()
            .map(|i| {
                x += 1;
                return (i.clone(), x);
            })
            .collect();

        MenuAction {
            stateful: StatefulList::with_items(vec),
        }
    }
}

#[derive(Clone)]
struct NoteList {
    stateful: StatefulList<Note>,
}

#[derive(Clone)]
struct ViewerAction<'a> {
    stateful: StatefulList<(&'a str, usize)>,
    input_mode: InputMode,
    input: String,
}

impl ViewerAction<'_> {
    fn new(actions: Vec<&str>) -> ViewerAction {
        let mut x: usize = 0;
        let vec = actions
            .iter()
            .map(|i| {
                x += 1;
                return (i.clone(), x);
            })
            .collect();

        ViewerAction {
            input_mode: InputMode::Normal,
            input: String::new(),
            stateful: StatefulList::with_items(vec),
        }
    }
}

#[derive(Clone)]
struct App<'a> {
    menu_actions: MenuAction<'a>,
    state_notes: NoteList,
    viewer_actions: ViewerAction<'a>,
    active_window: Windows,
}

impl<'a> App<'a> {
    fn new() -> App<'a> {
        let saved_notes: SavedNotes = match load_file("saved-notes.bin", 0) {
            Ok(notes) => notes,
            Err(_) => SavedNotes { notes: vec![] },
        };

        let mut menu_actions = MenuAction::new(vec!["New note", "Quit"]);
        menu_actions.stateful.select_first();

        let state_notes = NoteList {
            stateful: StatefulList::with_items(saved_notes.notes),
        };

        let viewer_actions = ViewerAction::new(vec!["Start writing", "Cancel", "Save"]);

        App {
            menu_actions,
            state_notes,
            viewer_actions,
            active_window: Windows::ACTIONS,
        }
    }

    fn quit(&mut self) -> io::Result<()> {
        let notes: Vec<Note> = self.state_notes.stateful.items.drain(..).collect();

        save_file("saved-notes.bin", 0, &SavedNotes { notes }).unwrap();

        return Ok(());
    }

    fn add_note(&mut self, note: Note) {
        self.state_notes.stateful.items.push(note);
    }

    fn remove_note(&mut self, index: usize) {
        self.state_notes.stateful.items.remove(index);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // todo: move each input_handler to it's widget struct
                match app.active_window {
                    Windows::ACTIONS => match key.code {
                        KeyCode::Char('q') => return app.quit(),
                        KeyCode::Esc => return app.quit(),
                        KeyCode::Down => app.menu_actions.stateful.next(),
                        KeyCode::Up => app.menu_actions.stateful.previous(),
                        KeyCode::Right => {
                            if app.state_notes.stateful.items.len() > 0 {
                                app.state_notes.stateful.select_first();
                                app.active_window = Windows::NOTES;
                                app.menu_actions.stateful.unselect();
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(current) = app.menu_actions.stateful.selected() {
                                match current {
                                    0 => {
                                        app.active_window = Windows::WRITER;
                                        app.viewer_actions.stateful.select_first();
                                    }
                                    1 => return app.quit(),
                                    _ => todo!(),
                                }
                            }
                        }
                        _ => {}
                    },
                    Windows::NOTES => match key.code {
                        KeyCode::Char('q') => return app.quit(),
                        KeyCode::Up => app.state_notes.stateful.previous(),
                        KeyCode::Down => app.state_notes.stateful.next(),
                        KeyCode::Left => {
                            app.menu_actions.stateful.select_first();
                            app.active_window = Windows::ACTIONS;
                            app.state_notes.stateful.unselect();
                        }
                        _ => {}
                    },
                    Windows::WRITER => match app.viewer_actions.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Enter => {
                                if let Some(current) = app.viewer_actions.stateful.selected() {
                                    match current {
                                        0 => {
                                            app.active_window = Windows::WRITER;
                                            app.viewer_actions.input_mode = InputMode::Editing;
                                        }
                                        1 => {
                                            app.viewer_actions.input.clear();
                                            app.viewer_actions.stateful.unselect();
                                            app.viewer_actions.input.clear();
                                            app.active_window = Windows::ACTIONS;
                                        }
                                        2 => {
                                            let dt = Utc::now();
                                            let timestamp = dt.format("%F %T").to_string();

                                            let text: String =
                                                app.viewer_actions.input.drain(..).collect();
                                            let mut lines: VecDeque<&str> =
                                                text.split('\n').collect();

                                            let title = match lines.pop_front() {
                                                Some(t) => String::from(t),
                                                None => "New note".to_string(),
                                            };

                                            let text: String = lines.drain(..).collect();

                                            app.add_note(Note {
                                                title,
                                                text,
                                                timestamp,
                                            });

                                            app.viewer_actions.stateful.unselect();
                                            app.active_window = Windows::ACTIONS;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            KeyCode::Up => app.viewer_actions.stateful.previous(),
                            KeyCode::Down => app.viewer_actions.stateful.next(),
                            KeyCode::Esc => {
                                app.viewer_actions.stateful.unselect();
                                app.active_window = Windows::ACTIONS;
                            }
                            KeyCode::Char('q') => return app.quit(),
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => {
                                app.viewer_actions.input.push('\n');
                            }
                            KeyCode::Char(c) => {
                                app.viewer_actions.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.viewer_actions.input.pop();
                            }
                            KeyCode::Esc => {
                                app.viewer_actions.input_mode = InputMode::Normal;
                            }
                            _ => {}
                        },
                    },
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
        .split(Rect::new(0, 0, size.width / 2, size.height));

    let right_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(Rect::new(
            chunks[0].x + chunks[0].width,
            chunks[0].y,
            size.width - chunks[0].width,
            size.height,
        ));

    let items: Vec<ListItem> = match app.active_window {
        Windows::WRITER => app
            .viewer_actions
            .stateful
            .items
            .iter()
            .map(|i| {
                let lines = vec![Spans::from(i.0)];
                ListItem::new(lines).style(Style::default().fg(Color::Blue))
            })
            .collect(),
        _ => app
            .menu_actions
            .stateful
            .items
            .iter()
            .map(|i| {
                let lines = vec![Spans::from(i.0)];
                ListItem::new(lines).style(Style::default().fg(Color::Blue))
            })
            .collect(),
    };

    let state = match app.active_window {
        Windows::WRITER => &mut app.viewer_actions.stateful.state,
        _ => &mut app.menu_actions.stateful.state,
    };

    // Create a List from all list actions and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .highlight_style(Style::default().bg(Color::White))
        .highlight_symbol("> ");

    f.render_stateful_widget(items, chunks[0], state);

    let notes: Vec<ListItem> = app
        .state_notes
        .stateful
        .items
        .iter()
        .map(|note| {
            let header = Spans::from(vec![
                Span::styled(
                    format!("{:<9}", note.title),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" "),
                Span::styled(
                    String::from(&note.timestamp),
                    Style::default().add_modifier(Modifier::ITALIC),
                ),
            ]);

            let log = Spans::from(vec![Span::raw(String::from(&note.text))]);

            ListItem::new(vec![
                Spans::from("-".repeat(chunks[1].width as usize)),
                header,
                Spans::from(""),
                log,
            ])
        })
        .collect();

    let notes_list = List::new(notes)
        .block(Block::default().borders(Borders::ALL).title("Notes"))
        .highlight_style(Style::default().bg(Color::Rgb(133, 95, 80)));

    f.render_stateful_widget(
        notes_list,
        right_chunk[0],
        &mut app.state_notes.stateful.state,
    );

    let create_block = |title: String| Block::default().title(title).borders(Borders::ALL);

    let paragraph = match app.active_window {
        Windows::ACTIONS => {
            let text = vec![
                Spans::from(Span::styled(
                    "How to navigate the app:",
                    Style::default().fg(Color::Red),
                )),
                Spans::from(""),
                Spans::from("Use the up and down arrow keys to scroll the lists"),
                Spans::from(
                    "Use the left and right arrow keys to switch from the Action and Notes screen",
                ),
                Spans::from("Press enter to press a button"),
                Spans::from(""),
                Spans::from(Span::styled(
                    "Press 'q' to quit the app or use the quit button",
                    Style::default().fg(Color::Red),
                )),
            ];

            Paragraph::new(text)
                .style(Style::default())
                .wrap(Wrap { trim: true })
                .block(create_block("Info".to_string()))
                .alignment(tui::layout::Alignment::Left)
        }
        Windows::NOTES => {
            let note =
                &app.state_notes.stateful.items[app.state_notes.stateful.selected().unwrap()];

            Paragraph::new(String::from(&note.text))
                .style(Style::default())
                .wrap(Wrap { trim: true })
                .block(create_block(String::from(&note.title)))
                .alignment(tui::layout::Alignment::Left)
        }
        Windows::WRITER => Paragraph::new(app.viewer_actions.input.as_ref())
            .style(match app.viewer_actions.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .wrap(Wrap { trim: true })
            .block(create_block("New note".to_string()))
            .alignment(tui::layout::Alignment::Left),
    };

    f.render_widget(paragraph, chunks[1]);

    match app.viewer_actions.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            let splits: Vec<&str> = app.viewer_actions.input.split('\n').collect();

            // ! This does not account for line wrapping. Only linebreaks.
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + splits[splits.len() - 1].len() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + splits.len() as u16,
            )
        }
    }
}
