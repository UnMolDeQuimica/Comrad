use std::{env, fs, path::Path, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer, layout::{Constraint, Flex, Layout, Rect}, style::{
        palette::tailwind::{BLUE, GREEN, SLATE},
        Color, Modifier, Style, Stylize,
    }, symbols::border, text::{Line, Text}, widgets::{Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap, Clear}, DefaultTerminal, Frame
};

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
fn main() -> io::Result<()> {
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
}

impl App {

    fn next(&mut self) {
        let i = self.list_state.selected().unwrap();
        if i < self.commands.len() - 1 {
            self.list_state.select((Some(i + 1)));
        }
    }

    fn previous(&mut self) {
        let i = self.list_state.selected().unwrap();
        if i > 0 {
            self.list_state.select((Some(i - 1)));
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
        if !self.filter_mode {

            match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char('g') => self.first(),
                KeyCode::Char('G') => self.last(),
                KeyCode::Char('j') => self.previous(),
                KeyCode::Char('k') => self.next(),
                KeyCode::Char('/') => {
                    self.filter_mode = true;
                    // self.update_filter();
                },
                KeyCode::Up => self.previous(),
                KeyCode::Down => self.next(),
                _ => {}
            }
        }

        else {
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
                }

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
        commands.sort();
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
        let popup = Paragraph::new(Text::raw(&self.filter_query)).block(Block::bordered().title("Filter"));
        Clear.render(popup_area, buf);
        popup.render(popup_area, buf)
    }

}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" COMRAD ".bold());
        let instructions = Line::from(vec![
            " Down ".into(),
            "<j>".blue().bold(),
            " Up ".into(),
            "<k>".blue().bold(),
            " First ".into(),
            "<g> ".blue().bold(),
            " Last ".into(),
            "<G> ".blue().bold(),
            " Filter ".into(),
            "</>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),

        ]);
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
            .highlight_spacing(HighlightSpacing::Always);
            // .render(area, buf);
        
        StatefulWidget::render(list, area, buf, &mut self.list_state);

        self.render_filter_popup(area, buf);
        // Paragraph::new(example_text)
            // .centered()
            // .block(block)
            // .render(area, buf);
    }
}
