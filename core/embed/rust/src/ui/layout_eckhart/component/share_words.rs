use crate::{
    strutil::TString,
    ui::{
        component::{Component, Event, EventCtx, Never, PaginateFull},
        geometry::{Alignment, Offset, Rect},
        layout_eckhart::{fonts, theme},
        shape::{Bar, Renderer, Text},
        util::Pager,
    },
};

use heapless::Vec;

const MAX_WORDS: usize = 33; // super-shamir has 33 words, all other have less

type IndexVec = Vec<u8, MAX_WORDS>;

/// Component showing mnemonic/share words during backup procedure. Model T3W1
/// contains one word per screen. A user is instructed to swipe up/down to see
/// next/previous word.
pub struct ShareWords<'a> {
    share_words: Vec<TString<'a>, MAX_WORDS>,
    area: Rect,
    repeated_indices: IndexVec,
    pager: Pager,
}

impl<'a> ShareWords<'a> {
    const AREA_WORD_HEIGHT: i16 = 120;
    const ORDINAL_PADDING: i16 = 16;

    pub fn new(share_words: Vec<TString<'a>, MAX_WORDS>) -> Self {
        let repeated_indices = Self::find_repeated(share_words.as_slice());
        let pager = Pager::new(share_words.len() as u16);
        Self {
            share_words,
            area: Rect::zero(),
            repeated_indices,
            pager,
        }
    }

    pub fn is_repeated(&self) -> bool {
        self.repeated_indices
            .contains(&(self.pager().current() as u8))
    }

    fn find_repeated(share_words: &[TString]) -> IndexVec {
        let mut repeated_indices = IndexVec::new();
        for i in (0..share_words.len()).rev() {
            let word = share_words[i];
            if share_words[..i].contains(&word) {
                unwrap!(repeated_indices.push(i as u8));
            }
        }
        repeated_indices.reverse();
        repeated_indices
    }
}

// Pagination
impl<'a> PaginateFull for ShareWords<'a> {
    fn pager(&self) -> Pager {
        self.pager
    }

    fn change_page(&mut self, to_page: u16) {
        let to_page = to_page.min(self.pager.total() - 1);

        // Update the pager
        self.pager.set_current(to_page);
    }
}

impl<'a> Component for ShareWords<'a> {
    type Msg = Never;

    fn place(&mut self, bounds: Rect) -> Rect {
        self.area = bounds;
        bounds
    }

    fn event(&mut self, _ctx: &mut EventCtx, _event: Event) -> Option<Self::Msg> {
        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        // the ordinal number of the current word
        let ordinal_val = self.pager().current() as u8 + 1;
        let ordinal_pos = self.area.top_left();
        let ordinal = uformat!("{}", ordinal_val);
        Text::new(ordinal_pos, &ordinal, fonts::FONT_SATOSHI_REGULAR_38)
            .with_fg(theme::GREY)
            .render(target);

        // Render lines as bars with the with 1px
        let top_line = Rect::from_bottom_right_and_size(
            self.area.top_right(),
            Offset::new(
                self.area.width()
                    - theme::TEXT_NORMAL.text_font.text_width(&ordinal)
                    - Self::ORDINAL_PADDING,
                1,
            ),
        );
        let bottom_line = Rect::from_bottom_right_and_size(
            self.area.bottom_right(),
            Offset::new(self.area.width(), 1),
        );

        Bar::new(top_line)
            .with_fg(theme::GREY_EXTRA_DARK)
            .render(target);

        Bar::new(bottom_line)
            .with_fg(theme::GREY_EXTRA_DARK)
            .render(target);

        let word = self.share_words[self.pager().current() as usize];
        let font = fonts::FONT_SATOSHI_EXTRALIGHT_72;

        let word_baseline = self.area.center() + Offset::y(font.visible_text_height("A") / 2);
        word.map(|w| {
            Text::new(word_baseline, w, font)
                .with_align(Alignment::Center)
                .render(target);
        });
    }
}

#[cfg(feature = "ui_debug")]
impl<'a> crate::trace::Trace for ShareWords<'a> {
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("ShareWordsInner");
        let word = &self.share_words[self.pager().current() as usize];
        let content = word.map(|w| uformat!("{}. {}\n", self.pager().current() + 1, w));
        t.string("screen_content", content.as_str().into());
        t.int("page_count", self.share_words.len() as i64)
    }
}
