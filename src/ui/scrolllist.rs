use super::style::SharedTheme;
use std::iter::Iterator;
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, Text, Widget},
    Frame,
};

///
struct ScrollableList<'b, L>
where
    L: Iterator<Item = Text<'b>>,
{
    block: Option<Block<'b>>,
    /// Items to be displayed
    items: L,
    /// Index of the scroll position
    scroll: usize,
    /// Base style of the widget
    style: Style,
}

impl<'b, L> ScrollableList<'b, L>
where
    L: Iterator<Item = Text<'b>>,
{
    fn new(items: L) -> Self {
        Self {
            block: None,
            items,
            scroll: 0,
            style: Style::default(),
        }
    }

    fn block(mut self, block: Block<'b>) -> Self {
        self.block = Some(block);
        self
    }

    fn scroll(mut self, index: usize) -> Self {
        self.scroll = index;
        self
    }
}

impl<'b, L> Widget for ScrollableList<'b, L>
where
    L: Iterator<Item = Text<'b>>,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render items
        List::new(self.items)
            .block(self.block.unwrap_or_default())
            .style(self.style)
            .render(area, buf);
    }
}

pub fn draw_list<'b, B: Backend, L>(
    f: &mut Frame<B>,
    r: Rect,
    title: &'b str,
    items: L,
    select: Option<usize>,
    selected: bool,
    theme: &SharedTheme,
) where
    L: Iterator<Item = Text<'b>>,
{
    let list = ScrollableList::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .title_style(theme.title(selected))
                .border_style(theme.block(selected)),
        )
        .scroll(select.unwrap_or_default());
    f.render_widget(list, r)
}
