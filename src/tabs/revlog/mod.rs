mod utils;

use crate::{
    components::{
        CommandBlocking, CommandInfo, Component, DrawableComponent,
        ScrollType,
    },
    keys,
    strings::commands,
    ui::calc_scroll_top,
};
use asyncgit::{sync, AsyncLog, AsyncNotification, CWD};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use std::{borrow::Cow, cmp, convert::TryFrom, time::Instant};
use sync::Tags;
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Text},
    Frame,
};
use utils::{ItemBatch, LogEntry};

const COLOR_SELECTION_BG: Color = Color::Blue;

const STYLE_TAG: Style = Style::new().fg(Color::Yellow);
const STYLE_HASH: Style = Style::new().fg(Color::Magenta);
const STYLE_TIME: Style = Style::new().fg(Color::Blue);
const STYLE_AUTHOR: Style = Style::new().fg(Color::Green);
const STYLE_MSG: Style = Style::new().fg(Color::Reset);

const STYLE_TAG_SELECTED: Style =
    Style::new().fg(Color::Yellow).bg(COLOR_SELECTION_BG);
const STYLE_HASH_SELECTED: Style =
    Style::new().fg(Color::Magenta).bg(COLOR_SELECTION_BG);
const STYLE_TIME_SELECTED: Style =
    Style::new().fg(Color::White).bg(COLOR_SELECTION_BG);
const STYLE_AUTHOR_SELECTED: Style =
    Style::new().fg(Color::Green).bg(COLOR_SELECTION_BG);
const STYLE_MSG_SELECTED: Style =
    Style::new().fg(Color::Reset).bg(COLOR_SELECTION_BG);

static ELEMENTS_PER_LINE: usize = 10;
static SLICE_SIZE: usize = 1200;

///
pub struct Revlog {
    selection: usize,
    selection_max: usize,
    items: ItemBatch,
    git_log: AsyncLog,
    visible: bool,
    first_open_done: bool,
    scroll_state: (Instant, f32),
    tags: Tags,
    current_size: (u16, u16),
    scroll_top: usize,
}

impl Revlog {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        Self {
            items: ItemBatch::default(),
            git_log: AsyncLog::new(sender.clone()),
            selection: 0,
            selection_max: 0,
            visible: false,
            first_open_done: false,
            scroll_state: (Instant::now(), 0_f32),
            tags: Tags::new(),
            current_size: (0, 0),
            scroll_top: 0,
        }
    }

    ///
    pub fn any_work_pending(&self) -> bool {
        self.git_log.is_pending()
    }

    ///
    pub fn update(&mut self) {
        self.selection_max =
            self.git_log.count().unwrap().saturating_sub(1);

        if self.items.needs_data(self.selection, self.selection_max) {
            self.fetch_commits();
        }

        if self.tags.is_empty() {
            self.tags = sync::get_tags(CWD).unwrap();
        }
    }

    fn fetch_commits(&mut self) {
        let want_min = self.selection.saturating_sub(SLICE_SIZE / 2);

        let commits = sync::get_commits_info(
            CWD,
            &self.git_log.get_slice(want_min, SLICE_SIZE).unwrap(),
            self.current_size.0.into(),
        );

        if let Ok(commits) = commits {
            self.items.set_items(want_min, commits);
        }
    }

    fn move_selection(&mut self, scroll: ScrollType) {
        self.update_scroll_speed();

        #[allow(clippy::cast_possible_truncation)]
        let speed_int = usize::try_from(self.scroll_state.1 as i64)
            .unwrap()
            .max(1);

        let page_offset =
            usize::from(self.current_size.1).saturating_sub(1);

        self.selection = match scroll {
            ScrollType::Up => {
                self.selection.saturating_sub(speed_int)
            }
            ScrollType::Down => {
                self.selection.saturating_add(speed_int)
            }
            ScrollType::PageUp => {
                self.selection.saturating_sub(page_offset)
            }
            ScrollType::PageDown => {
                self.selection.saturating_add(page_offset)
            }
            ScrollType::Home => 0,
            ScrollType::End => self.selection_max,
        };

        self.selection = cmp::min(self.selection, self.selection_max);

        self.update();
    }

    fn update_scroll_speed(&mut self) {
        const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
        const SCROLL_SPEED_START: f32 = 0.1_f32;
        const SCROLL_SPEED_MAX: f32 = 10_f32;
        const SCROLL_SPEED_MULTIPLIER: f32 = 1.05_f32;

        let now = Instant::now();

        let since_last_scroll =
            now.duration_since(self.scroll_state.0);

        self.scroll_state.0 = now;

        let speed = if since_last_scroll.as_millis()
            < REPEATED_SCROLL_THRESHOLD_MILLIS
        {
            self.scroll_state.1 * SCROLL_SPEED_MULTIPLIER
        } else {
            SCROLL_SPEED_START
        };

        self.scroll_state.1 = speed.min(SCROLL_SPEED_MAX);
    }

    fn add_entry<'a>(
        e: &'a LogEntry,
        selected: bool,
        txt: &mut Vec<Text<'a>>,
        tags: Option<String>,
    ) {
        let count_before = txt.len();

        let splitter_txt = Cow::from(" ");
        let splitter = if selected {
            Text::Styled(
                splitter_txt,
                Style::new().bg(COLOR_SELECTION_BG),
            )
        } else {
            Text::Raw(splitter_txt)
        };

        txt.push(Text::Styled(
            Cow::from(&e.hash[0..7]),
            if selected {
                STYLE_HASH_SELECTED
            } else {
                STYLE_HASH
            },
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(e.time.as_str()),
            if selected {
                STYLE_TIME_SELECTED
            } else {
                STYLE_TIME
            },
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(e.author.as_str()),
            if selected {
                STYLE_AUTHOR_SELECTED
            } else {
                STYLE_AUTHOR
            },
        ));
        txt.push(splitter.clone());
        txt.push(Text::Styled(
            Cow::from(if let Some(tags) = tags {
                format!(" {}", tags)
            } else {
                String::from("")
            }),
            if selected {
                STYLE_TAG_SELECTED
            } else {
                STYLE_TAG
            },
        ));
        txt.push(splitter);
        txt.push(Text::Styled(
            Cow::from(e.msg.as_str()),
            if selected {
                STYLE_MSG_SELECTED
            } else {
                STYLE_MSG
            },
        ));
        txt.push(Text::Raw(Cow::from("\n")));

        assert_eq!(txt.len() - count_before, ELEMENTS_PER_LINE);
    }

    fn get_text(&self) -> Vec<Text> {
        let selection = self.relative_selection();

        let mut txt = Vec::new();

        for (idx, e) in self.items.items.iter().enumerate() {
            let tag = if let Some(tags) = self.tags.get(&e.hash) {
                Some(tags.join(" "))
            } else {
                None
            };
            Self::add_entry(e, idx == selection, &mut txt, tag);
        }

        txt
    }

    fn relative_selection(&self) -> usize {
        self.selection.saturating_sub(self.items.index_offset)
    }
}

impl DrawableComponent for Revlog {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        self.current_size = (
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );

        let height_in_lines = self.current_size.1 as usize;
        let selection = self.relative_selection();

        self.scroll_top = calc_scroll_top(
            self.scroll_top,
            height_in_lines,
            selection,
        );

        let title = format!(
            "commit {}/{}",
            self.selection, self.selection_max,
        );

        f.render_widget(
            Paragraph::new(
                self.get_text()
                    .iter()
                    .skip(self.scroll_top * ELEMENTS_PER_LINE)
                    .take(height_in_lines * ELEMENTS_PER_LINE),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title.as_str()),
            )
            .alignment(Alignment::Left),
            area,
        );
    }
}

impl Component for Revlog {
    fn event(&mut self, ev: Event) -> bool {
        if self.visible {
            if let Event::Key(k) = ev {
                return match k {
                    keys::MOVE_UP => {
                        self.move_selection(ScrollType::Up);
                        true
                    }
                    keys::MOVE_DOWN => {
                        self.move_selection(ScrollType::Down);
                        true
                    }
                    keys::SHIFT_UP | keys::HOME => {
                        self.move_selection(ScrollType::Home);
                        true
                    }
                    keys::SHIFT_DOWN | keys::END => {
                        self.move_selection(ScrollType::End);
                        true
                    }
                    keys::PAGE_UP => {
                        self.move_selection(ScrollType::PageUp);
                        true
                    }
                    keys::PAGE_DOWN => {
                        self.move_selection(ScrollType::PageDown);
                        true
                    }
                    _ => false,
                };
            }
        }

        false
    }

    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        out.push(CommandInfo::new(
            commands::SCROLL,
            self.visible,
            self.visible || force_all,
        ));

        if self.visible {
            CommandBlocking::Blocking
        } else {
            CommandBlocking::PassingOn
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn show(&mut self) {
        self.visible = true;

        if !self.first_open_done {
            self.first_open_done = true;
            self.git_log.fetch().unwrap();
        }
    }
}
