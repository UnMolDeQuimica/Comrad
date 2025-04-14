use core::str;
use std::{env, fs, io, process::Command};

use crossterm::{event::{self, Event, KeyCode, KeyEvent, KeyEventKind}, terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen, EnterAlternateScreen}};
use ratatui::{
    buffer::Buffer, layout::{Constraint, Flex, Layout, Rect}, style::{
        palette::tailwind::SLATE, Modifier, Style, Stylize,
    }, symbols::border, text::{Line, Text}, widgets::{Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Clear}, DefaultTerminal, Frame
};

use clap::Parser;

#[derive(Parser)]
#[command(name = "comrad", version, about, long_about = r#"
A simple TUI tool made with [ratatui](https://ratatui.rs/) that shows all the terminal commands available in your computer.

Run the `comrad` command to enter the TUI.

- You can move up and down with j and k or using the up and down arrow keys.
- You can go to the first entry pressing 'g' and to the last entry pressing 'G'.
- Press '/' to enter filter mode.
- Press 'h' to show the `--help` page of the current command.
- Press 'm' to show the `man` page of the current command (only available if you have `man` installed).
- Press 'M' to enter the `man` page of the current command (only available if you have `man` installed).
- Press 't' to show the `tldr` page of the current command (only available if you have `tldr` installed).
- Press 'q' to exit comrad.
- Press 'ESC' to go back to the commands list.
"#
)]
struct Cli {
}


const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _cli = Cli::parse();

    run_ui()?;

    Ok(())
}


fn run_ui() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}
#[derive(Debug, Default)]
pub struct App {
    commands: Vec<String>,
    list_state: ratatui::widgets::ListState,
    exit: bool,
    filter_mode: bool,
    filter_query: String,
    show_man_help: bool,
    show_tldr_help: bool,
    add_to_tldr_state: bool,
    tldr_command: String,
    show_help: bool,
    show_comrad_help: bool,
}

impl App {

    fn next(&mut self) {
        let i = self.list_state.selected().unwrap();
        if i < self.commands.len() - 1 {
            self.list_state.select(Some(i + 1));
        }
    }

    fn previous(&mut self) {
        let i = self.list_state.selected().unwrap();
        if i > 0 {
            self.list_state.select(Some(i - 1));
        }
    }

    fn first(&mut self) {
        self.list_state.select_first();
    }

    fn last(&mut self) {
        self.list_state.select_last();
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.commands = self.get_commands();

        self.list_state = ListState::default();
        self.list_state.select(Some(0));

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.filter_mode {
            match key_event.code {
                KeyCode::Enter if self.filter_mode => {
                    self.filter_mode = false;
                    self.list_state.select(Some(0));
                    // self.update_filter();
                },

                KeyCode::Char(c) => {
                    self.filter_query.push(c);
                    // self.update_filter();
                },

                KeyCode::Backspace => {
                    self.filter_query.pop();
                    // self.update_filter();
                },
                KeyCode::Esc => {
                    self.filter_mode = false;
                    self.list_state.select(Some(0));
                },

                _ => {}
            }
        }
        else if self.show_man_help {
            match key_event.code {
                KeyCode::Esc => {
                    self.show_man_help = false;
                    // self.list_state.select(Some(0));
                },

                _ => {}
            }
        }
        else if self.show_tldr_help {
            match key_event.code {
                KeyCode::Esc => {
                    self.show_tldr_help = false;
                    // self.list_state.select(Some(0));
                },

                _ => {}
            }
        }
        else if self.add_to_tldr_state {
            match key_event.code {
                KeyCode::Char('y') => {
                    self.add_to_tldr_cache();
                    self.add_to_tldr_state = false;
                },
                KeyCode::Char('Y') => {
                    self.add_to_tldr_cache();
                    self.add_to_tldr_state = false;
                },
                _ => {
                    self.add_to_tldr_state = false;
                    self.show_tldr_help = false;
                }
            }
        }
        else if self.show_help {
            match key_event.code {
                KeyCode::Esc => {
                    self.show_help = false;
                },
                _ => {}
            }
        }
        else {
            match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char('g') => self.first(),
                KeyCode::Char('G') => self.last(),
                KeyCode::Char('j') => self.previous(),
                KeyCode::Char('k') => self.next(),
                KeyCode::Char('h') => {
                    self.show_help = true;
                },
                KeyCode::Char('m') => {
                    self.show_man_help = true
                },
                KeyCode::Char('M') => self.enter_man_help(),
                KeyCode::Char('t') => {
                    self.show_tldr_help = true
                },
                KeyCode::Char('/') => {
                    self.filter_mode = true;
                    // self.update_filter();
                },
                KeyCode::Up => self.previous(),
                KeyCode::Down => self.next(),
                _ => {}
            }

        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn get_commands(&mut self) -> Vec<String> {
        let mut commands = Vec::new();

        if let Ok(path) = env::var("PATH") {
            for dir in path.split(':') {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(name) = path.file_name() {
                                commands.push(name.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }
        commands.sort_by(|command_a, command_b| command_a.to_lowercase().cmp(&command_b.to_lowercase()));
        commands.dedup();
        commands
    }

    fn center(&mut self, area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
        let [area] = Layout::horizontal([horizontal])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([vertical]).margin(3).areas(area);
        area
    }

    fn render_filter_popup(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.filter_mode {
            return
        };

        let popup_area = self.center(
            area,
            Constraint::Percentage(30),
            Constraint::Length(3)
        );
        let popup = Paragraph::new(Text::raw(&self.filter_query)).block(Block::bordered().title(Line::from(" Filter ").centered()));
        Clear.render(popup_area, buf);
        popup.render(popup_area, buf)
    }

    fn render_man_help(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.show_man_help {
            return
        };

        let index = self.list_state.selected().unwrap();
        let commands = Vec::from_iter(self.commands
            .iter()
            .filter(|cmd|
            cmd.to_lowercase().contains(&self.filter_query.to_lowercase())));

        let command = commands[index];

        let man_output = Command::new("man").arg(command).output().unwrap();

        let output = match str::from_utf8(&man_output.stdout) {
            Ok(val) => val,
            Err(_) => "Unexpected error when reading the output."
        };

        let text = if output.len() > 0 { output } else { "No entries in the manual for this command" };

        Clear.render(area, buf);
        Paragraph::new(Text::raw(text)).block(Block::bordered().title(Line::from(String::from(command)).centered())).render(area, buf);
    }

    fn enter_man_help(&mut self) {
        let index = self.list_state.selected().unwrap();
        let commands = Vec::from_iter(self.commands
            .iter()
            .filter(|cmd|
            cmd.to_lowercase().contains(&self.filter_query.to_lowercase())));

        let command = commands[index];

        Command::new("man").arg(command).status().expect("No entries in the manual for this command.");

    }

    fn check_in_tldr_cache(&self, command: &String) -> bool {
        let command_output = Command::new("tldr").arg("-l").output();
        let tldr_cache = match command_output {
            Ok(tldr_cache) => {
                if tldr_cache.status.success() {
                    let result = String::from_utf8_lossy(&tldr_cache.stdout).to_string();
                    result
                }
                else {
                    let err_msg = String::from_utf8_lossy(&tldr_cache.stderr).to_string();
                    err_msg
                }
            }
            Err(_) => {
                let tldr_not_installed_message = String::from("Error: Check if TLDR is installed in your system.").to_string();
                tldr_not_installed_message
            }
        };
        tldr_cache.contains(command)
    }

    fn render_tldr_help(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.show_tldr_help {
            return
        };

        let index = self.list_state.selected().unwrap();
        let commands = Vec::from_iter(self.commands
            .iter()
            .filter(|cmd|
            cmd.to_lowercase().contains(&self.filter_query.to_lowercase())));

        let command = commands[index];

        if !self.check_in_tldr_cache(command) {
            self.add_to_tldr_state = true;
            self.show_tldr_help = false;
            return
        }

        let tldr_output = Command::new("tldr").arg(command).output().unwrap();

        let output = match str::from_utf8(&tldr_output.stdout){
            Ok(val) => val,
            Err(_) => "Unexpected error when reading the output."
        };

        let text = if output.len() > 0 { output } else { "No entries in tldr for this command" };

        Clear.render(area, buf);
        Paragraph::new(Text::raw(text)).block(Block::bordered().title(Line::from(String::from(command)).centered())).render(area, buf);
    }

    fn render_add_to_tldr_cache(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.add_to_tldr_state {
            return
        }
        let popup_area = self.center(
            area,
            Constraint::Length(80),
            Constraint::Length(5)
        );
        let popup = Paragraph::new(Text::raw(" The command is not in the TLDR cache. \n Press Y to add it to the cache (it might take a while)").centered()).block(Block::bordered().title(Line::from(" Warning ").centered()));
        Clear.render(popup_area, buf);
        popup.render(popup_area, buf);
    }

    fn add_to_tldr_cache(&mut self) {
        disable_raw_mode().unwrap();
        crossterm::execute!(io::stdout(), LeaveAlternateScreen).unwrap();
        Command::new("tldr").arg(&self.tldr_command).status().unwrap();
        std::io::stdin().read_line(&mut String::new()).unwrap();

        enable_raw_mode().unwrap();
        crossterm::execute!(io::stdout(), EnterAlternateScreen).unwrap();
        println!("Exiting COMRAD")
    }

    fn render_help(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.show_help {
            return
        }

        let index = self.list_state.selected().unwrap();
        let commands = Vec::from_iter(self.commands
            .iter()
            .filter(|cmd|
            cmd.to_lowercase().contains(&self.filter_query.to_lowercase())));

        let command = commands[index];
        let help_output = Command::new(command).arg("--help").output().unwrap();

        let output = match str::from_utf8(&help_output.stdout){
            Ok(val) => val,
            Err(_) => "Unexpected error when reading the output."
        };

        let text = if output.len() > 0 { output } else { "No help implemented for this command" };
        Clear.render(area, buf);
        Paragraph::new(Text::raw(text)).block(Block::bordered().title(Line::from(String::from(command)).centered())).render(area, buf);
    }

    fn render_comrad_help(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.show_comrad_help {
            return
        };
        let text = String::from("
You can move up and down with j and k or using the up and down arrow keys.

You can go to the first entry pressing 'g' and to the last entry pressing 'G'.

Press '/' to enter filter mode.

Press 'h' to show the `--help` page of the current command.

Press 'm' to show the `man` page of the current command (only available if you have `man` installed).

Press 'M' to enter the `man` page of the current command (only available if you have `man` installed).

Press 't' to show the `tldr` page of the current command (only available if you have `tldr` installed).

Press 'q' to exit comrad.

Press 'ESC' to go back to the commands list.

        ");
        Clear.render(area, buf);
        Paragraph::new(Text::raw(text)).block(Block::bordered().title(Line::from(String::from("Comrad Help")).centered())).centered().render(area, buf);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" COMRAD ".bold());
        let filter_instructions = Line::from(vec![
            " Exit filter ".into(),
            "<Esc/Enter>".blue().bold(),
            ]);

        let general_instructions = Line::from(vec![
            " Comrad help ".into(),
            "<H> ".blue().bold(),
            " Show help ".into(),
            "<h> ".blue().bold(),
            " Show man ".into(),
            "<m> ".blue().bold(),
            " Enter man ".into(),
            "<M> ".blue().bold(),
            " Show tldr ".into(),
            "<t> ".blue().bold(),
            " Filter ".into(),
            "</>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),

            ]);

        let instructions = if self.filter_mode { filter_instructions.clone() } else { general_instructions.clone() };
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let commands = self.commands
            .iter()
            .filter(|cmd|
            cmd.to_lowercase().contains(&self.filter_query.to_lowercase()))
            .map(|c| ListItem::new(c.as_str()));

        let list =List::new(commands)
            .block(block)
            .highlight_symbol(">> ")
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_style(SELECTED_STYLE);

        StatefulWidget::render(list, area, buf, &mut self.list_state);

        self.render_filter_popup(area, buf);
        self.render_man_help(area, buf);
        self.render_tldr_help(area, buf);
        self.render_add_to_tldr_cache(area, buf);
        self.render_help(area, buf);
        self.render_comrad_help(area, buf)
    }
}
