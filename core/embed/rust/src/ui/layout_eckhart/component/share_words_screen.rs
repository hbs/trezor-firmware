use crate::{
    strutil::TString,
    translations::TR,
    ui::{
        component::{swipe_detect::SwipeConfig, Component, Event, EventCtx, PaginateFull},
        flow::Swipable,
        geometry::{Offset, Rect},
        layout_eckhart::component::{
            button::Button, theme, ActionBar, ActionBarMsg, Header, HeaderMsg, Hint, ShareWords,
        },
        shape::Renderer,
        util::Pager,
    },
};

use heapless::Vec;

/// Full-screen component for rendering ShareWords.
pub struct ShareWordsScreen<'a> {
    header: Header,
    content: ShareWords<'a>,
    hint: Option<Hint<'static>>,
    action_bar: ActionBar,
    /// Common area for the content and hint
    area: Rect,
}

pub enum ShareWordsScreenMsg {
    Cancelled,
    Confirmed,
    Menu,
}

impl<'a> ShareWordsScreen<'a> {
    const WORD_AREA_HEIGHT: i16 = 120;
    const WORD_AREA_WIDTH: i16 = 330;
    const WORD_Y_OFFSET: i16 = 76;

    pub fn new(share_words_vec: Vec<TString<'static>, 33>) -> Self {
        Self {
            header: Header::new(TString::empty()),
            content: ShareWords::new(share_words_vec),
            hint: None,
            action_bar: ActionBar::new_single(Button::empty()),
            area: Rect::zero(),
        }
    }

    pub fn with_header(mut self, header: Header) -> Self {
        self.header = header;
        self
    }

    pub fn with_hint(mut self, hint: Hint<'static>) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn with_action_bar(mut self, action_bar: ActionBar) -> Self {
        self.action_bar = action_bar;
        self
    }

    fn on_page_change(&mut self) {
        // Update the hint based on the current page
        if self.content.pager().is_first() {
            self.hint = Some(Hint::new_instruction(
                TR::share_words__first_word,
                Some(theme::ICON_INFO),
            ));
        } else if self.content.is_repeated() {
            self.hint = Some(Hint::new_instruction_green(
                TR::share_words__word_multiple_times,
                Some(theme::ICON_INFO),
            ));
        } else {
            let mut hint = Hint::new_page_counter();
            hint.update(self.content.pager());
            self.hint = Some(hint);
        }

        // if let Some(hint) = &mut self.hint {
        //     let (content_area, hint_area) =
        // self.area.split_bottom(Hint::HEIGHT_DEFAULT);
        //     hint.place(hint_area);
        //     self.content.place(content_area);
        // }

        self.action_bar.update(self.content.pager());

        self.place(self.area);
    }

    // fn snap_rect_centered_lr(bounds: Rect, width: i16, height: i16, top_offset:
    // i16) -> Rect {     assert!(bounds.width() >= width);
    //     assert!(bounds.height() >= height + top_offset);

    //     let top_left = bounds
    //         .top_left()
    //         .ofs(Offset::new((bounds.width() - width) / 2, top_offset));
    //     Rect::from_top_left_and_size(top_left, Offset::new(width, height))
    // }
}

impl<'a> Swipable for ShareWordsScreen<'a> {
    fn get_pager(&self) -> Pager {
        self.content.pager()
    }
    fn get_swipe_config(&self) -> SwipeConfig {
        SwipeConfig::default()
    }
}

impl<'a> Component for ShareWordsScreen<'a> {
    type Msg = ShareWordsScreenMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        self.area = bounds;
        let (header_area, rest) = bounds.split_top(Header::HEADER_HEIGHT);
        let (rest, action_bar_area) = rest.split_bottom(ActionBar::ACTION_BAR_HEIGHT);
        let content_area = if let Some(hint) = &mut self.hint {
            // TODO: hint area based on text
            let (rest, hint_area) = rest.split_bottom(hint.height());
            hint.place(hint_area);
            rest
        } else {
            rest
        };

        // Use constant y offset for the word area because the height is floating
        let top_left = content_area.top_left().ofs(Offset::new(
            (content_area.width() - Self::WORD_AREA_WIDTH) / 2,
            Self::WORD_Y_OFFSET,
        ));
        let content_area = Rect::from_top_left_and_size(
            top_left,
            Offset::new(Self::WORD_AREA_WIDTH, Self::WORD_AREA_HEIGHT),
        );

        self.header.place(header_area);
        self.content.place(content_area);
        self.action_bar.place(action_bar_area);

        bounds
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        if let Event::Attach(_) = event {
            self.on_page_change();
        }

        if let Some(msg) = self.header.event(ctx, event) {
            match msg {
                HeaderMsg::Cancelled => return Some(ShareWordsScreenMsg::Cancelled),
                HeaderMsg::Menu => return Some(ShareWordsScreenMsg::Menu),
            }
        }

        if let Some(msg) = self.action_bar.event(ctx, event) {
            match msg {
                ActionBarMsg::Cancelled => {
                    return Some(ShareWordsScreenMsg::Cancelled);
                }
                ActionBarMsg::Confirmed => {
                    return Some(ShareWordsScreenMsg::Confirmed);
                }
                ActionBarMsg::Prev => {
                    // self.page_idx = (self.page_idx - 1).max(0);
                    self.content.change_page(self.content.pager().prev());
                    self.on_page_change();
                    return None;
                }
                ActionBarMsg::Next => {
                    self.content.change_page(self.content.pager().next());
                    self.on_page_change();
                    return None;
                }
            }
        }

        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        self.header.render(target);
        self.content.render(target);

        if let Some(hint) = &self.hint {
            hint.render(target);
        }
        self.action_bar.render(target);
    }
}

#[cfg(feature = "ui_debug")]
impl<'a> crate::trace::Trace for ShareWordsScreen<'a> {
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("TextComponent");
        self.header.trace(t);
        self.content.trace(t);
        if let Some(hint) = &self.hint {
            hint.trace(t);
        }
        self.action_bar.trace(t);
    }
}
