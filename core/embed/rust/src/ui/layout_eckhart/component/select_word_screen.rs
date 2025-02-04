use crate::{
    strutil::TString,
    ui::{
        component::{Component, Event, EventCtx, Label},
        geometry::{Alignment, Grid, Insets, Offset, Rect},
        layout_eckhart::{
            component::{Button, ButtonMsg, Header, HeaderMsg},
            constant, theme,
        },
        shape::{Bar, Renderer},
        ui_firmware::MAX_WORD_QUIZ_ITEMS,
    },
};

pub struct SelectWordScreen {
    header: Header,
    description: Label<'static>,
    buttons: [Button; MAX_WORD_QUIZ_ITEMS],
}

pub enum SelectWordMsg {
    Selected(usize),
    /// Right header button clicked
    Cancelled,
}

impl SelectWordScreen {
    const INSET: i16 = 24;
    const DESCRIPTION_HEIGHT: i16 = 52;

    pub fn new(
        share_words_vec: [TString<'static>; MAX_WORD_QUIZ_ITEMS],
        description: TString<'static>,
    ) -> Self {
        let buttons: [Button; MAX_WORD_QUIZ_ITEMS] =
            share_words_vec.map(|word| Button::with_text(word).styled(theme::button_select_word()));

        Self {
            header: Header::new(TString::empty()),
            description: Label::new(description, Alignment::Start, theme::TEXT_MEDIUM)
                .vertically_centered(),
            buttons,
        }
    }

    pub fn with_header(mut self, header: Header) -> Self {
        self.header = header;
        self
    }
}

impl Component for SelectWordScreen {
    type Msg = SelectWordMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        // assert full screen
        debug_assert_eq!(bounds.height(), constant::HEIGHT);
        debug_assert_eq!(bounds.width(), constant::WIDTH);

        let (header_area, rest) = bounds.split_top(Header::HEADER_HEIGHT);

        let (description_area, button_area) = rest.split_top(Self::DESCRIPTION_HEIGHT);

        let description_area = description_area.inset(Insets::sides(Self::INSET));
        let button_area = button_area.inset(Insets::uniform(Self::INSET));

        let grid = Grid::new(button_area, MAX_WORD_QUIZ_ITEMS, 1);
        for (index, button) in self.buttons.iter_mut().enumerate() {
            button.place(grid.cell(index));
        }

        self.description.place(description_area);

        self.header.place(header_area);

        bounds
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        if let Some(msg) = self.header.event(ctx, event) {
            match msg {
                HeaderMsg::Cancelled => return Some(SelectWordMsg::Cancelled),
                _ => {}
            }
        }

        for (i, button) in self.buttons.iter_mut().enumerate() {
            if let Some(ButtonMsg::Clicked) = button.event(ctx, event) {
                return Some(SelectWordMsg::Selected(i));
            }
        }
        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        self.header.render(target);
        self.description.render(target);

        let n_seps = self.buttons.len() - 1;

        for (i, button) in (&self.buttons).into_iter().enumerate() {
            // Render button
            button.render(target);

            if i < n_seps {
                // Render button separator line below th button
                let separator = Rect::from_bottom_left_and_size(
                    button.area().bottom_left(),
                    Offset::new(button.area().width(), 1),
                );
                Bar::new(separator)
                    .with_fg(theme::GREY_EXTRA_DARK)
                    .render(target);
            }
        }
    }
}

#[cfg(feature = "ui_debug")]
impl crate::trace::Trace for SelectWordScreen {
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("SelectWordScreen");
        t.in_list("buttons", &|button_list| {
            for button in &self.buttons {
                button_list.child(button);
            }
        });
    }
}
