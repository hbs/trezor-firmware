use crate::ui::{
    component::{Component, Event, EventCtx, Maybe},
    display::Color,
    geometry::{Alignment, Insets, Offset, Rect},
    layout_eckhart::component::{
        button::{Button, ButtonContent, ButtonMsg, ButtonStyleSheet},
        theme,
    },
    shape::Renderer,
};

use heapless::Vec;

pub struct KeypadButton {
    button: Maybe<Button>,
}
impl Component for KeypadButton {
    type Msg = ButtonMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        self.button.place(bounds)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        self.button.event(ctx, event)
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        if self.button.is_visible() {
            self.button.render(target);
        }
    }
}

impl KeypadButton {
    pub fn new(bg_color: Color, inner: Button, visible: bool) -> Self {
        Self {
            button: Maybe::new(bg_color, inner, visible),
        }
    }
    pub fn hidden(bg_color: Color, inner: Button) -> Self {
        Self {
            button: Maybe::hidden(bg_color, inner),
        }
    }

    pub fn shown(bg_color: Color, inner: Button) -> Self {
        Self::new(bg_color, inner, true)
    }

    pub fn inner_mut(&mut self) -> &mut Button {
        self.button.inner_mut()
    }

    pub fn set_stylesheet(&mut self, styles: ButtonStyleSheet) {
        self.button.inner_mut().set_stylesheet(styles);
    }

    pub fn set_state(&mut self, ctx: &mut EventCtx, state: &ButtonState) {
        match state {
            ButtonState::Enabled => {
                self.button.show(ctx);
                self.button.inner_mut().enable(ctx);
            }
            ButtonState::Disabled => {
                self.button.show(ctx);
                self.button.inner_mut().disable(ctx);
            }
            ButtonState::Hidden => {
                self.button.hide(ctx);
            }
        }
    }

    pub fn content(&self) -> &ButtonContent {
        &self.button.inner().content()
    }

    pub fn set_expanded_touch_area(&mut self, insets: Insets) {
        self.button.inner_mut().set_expanded_touch_area(insets);
    }

    pub fn set_content(&mut self, content: ButtonContent) {
        self.button.inner_mut().set_content(content);
    }
}

pub enum ButtonType {
    Key(usize),
    Erase,   // Represents an erase button.
    Cancel,  // Represents a cancel button.
    Confirm, // Represents a confirm button.
    Back,    // Represents a back(previous) button.
}

pub enum ButtonState {
    Enabled,
    Disabled,
    Hidden,
}

pub struct Keypad<const KEY_COUNT: usize> {
    back_btn: KeypadButton,
    erase_btn: KeypadButton,
    cancel_btn: KeypadButton,
    confirm_btn: KeypadButton,
    keys: [KeypadButton; KEY_COUNT],
    pressed: Option<ButtonType>,
}

#[derive(PartialEq, Debug, Copy, Clone)]
#[cfg_attr(feature = "ui_debug", derive(ufmt::derive::uDebug))]
pub enum KeypadMsg {
    BackClicked,
    ConfirmClicked,
    EraseClicked,
    EraseLongPressed,
    CancelClicked,
    KeyClicked(usize),
}

impl<const KEY_COUNT: usize> Component for Keypad<KEY_COUNT> {
    type Msg = KeypadMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        let keypad_grid = OverlapGrid::<4, 3>::new(bounds);

        // Erase/cancel/back buttons.
        let erase_area = keypad_grid.border_of_cell(9);
        let erase_touch_inset = keypad_grid.insets_of_cell(9);

        self.erase_btn.place(erase_area);
        self.cancel_btn.place(erase_area);
        self.back_btn.place(erase_area);
        self.erase_btn
            .inner_mut()
            .set_expanded_touch_area(erase_touch_inset);
        self.cancel_btn
            .inner_mut()
            .set_expanded_touch_area(erase_touch_inset);
        self.back_btn
            .inner_mut()
            .set_expanded_touch_area(erase_touch_inset);

        // Confirm button.
        self.confirm_btn.place(keypad_grid.border_of_cell(11));

        // Keys
        for (i, btn) in self.keys.iter_mut().enumerate() {
            let idx = if i < 9 { i } else { i + 1 }; // Adjust for "0" key position.
            let area = keypad_grid.border_of_cell(idx);
            let touch_inset = keypad_grid.insets_of_cell(idx);
            btn.place(area);
            btn.set_expanded_touch_area(touch_inset);
        }
        bounds
    }
    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        match self.confirm_btn.event(ctx, event) {
            Some(ButtonMsg::Clicked) => {
                self.pressed = None;
                return Some(KeypadMsg::ConfirmClicked);
            }
            Some(ButtonMsg::Pressed) => {
                self.pressed = Some(ButtonType::Confirm);
            }
            _ => {}
        }

        match self.erase_btn.event(ctx, event) {
            Some(ButtonMsg::Clicked) => {
                self.pressed = None;
                return Some(KeypadMsg::EraseClicked);
            }
            Some(ButtonMsg::LongPressed) => {
                self.pressed = None;
                return Some(KeypadMsg::EraseLongPressed);
            }
            Some(ButtonMsg::Pressed) => {
                self.pressed = Some(ButtonType::Erase);
            }
            _ => {}
        }

        match self.cancel_btn.event(ctx, event) {
            Some(ButtonMsg::Clicked) => {
                self.pressed = None;
                return Some(KeypadMsg::CancelClicked);
            }
            Some(ButtonMsg::Pressed) => {
                self.pressed = Some(ButtonType::Cancel);
            }
            _ => {}
        }

        match self.back_btn.event(ctx, event) {
            Some(ButtonMsg::Clicked) => {
                self.pressed = None;
                return Some(KeypadMsg::BackClicked);
            }
            Some(ButtonMsg::Pressed) => {
                self.pressed = Some(ButtonType::Back);
            }
            _ => {}
        }

        for (idx, btn) in &mut self.keys.iter_mut().enumerate() {
            match btn.event(ctx, event) {
                Some(ButtonMsg::Clicked) => {
                    self.pressed = None;
                    return Some(KeypadMsg::KeyClicked(idx));
                }
                Some(ButtonMsg::Pressed) => {
                    self.pressed = Some(ButtonType::Key(idx));
                }
                _ => {}
            }
        }
        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        for btn in self.keys.iter() {
            btn.render(target);
        }

        self.cancel_btn.render(target);
        self.erase_btn.render(target);
        self.back_btn.render(target);
        self.confirm_btn.render(target);
        self.render_pressed_button(target);
    }
}

impl<const KEY_COUNT: usize> Keypad<KEY_COUNT> {
    const KEYBOARD_BUTTON_HEIGHT: i16 = 70;
    const KEYBOARD_BUTTON_RADIUS: u8 = 11;

    pub fn with_keypad_content(mut self, keypad_content: Vec<ButtonContent, KEY_COUNT>, key_style: ButtonStyleSheet ) -> Self {
        for (i, key_content) in keypad_content.into_iter().enumerate() {
            self.keys[i].set_content(key_content);
            self.keys[i].set_stylesheet(key_style);
        }
        self
    }

    pub fn get_key_content(&self, idx: usize) -> &ButtonContent {
        &self.keys[idx].content()
    }

    pub fn set_key_content(&mut self, idx: usize, content: ButtonContent) {
        self.keys[idx].set_content(content);
    }

    pub fn set_keys_state(&mut self, ctx: &mut EventCtx, state: &ButtonState) {
        for btn in self.keys.iter_mut() {
            btn.set_state(ctx, state);
        }
    }

    pub fn set_button_stylesheet(&mut self, button: ButtonType, styles: ButtonStyleSheet) {
        let apply_state = |btn: &mut KeypadButton, styles: ButtonStyleSheet| {
            btn.set_stylesheet(styles);
        };

        match button {
            ButtonType::Key(idx) => apply_state(&mut self.keys[idx], styles),
            ButtonType::Erase => apply_state(&mut self.erase_btn, styles),
            ButtonType::Cancel => apply_state(&mut self.cancel_btn, styles),
            ButtonType::Confirm => apply_state(&mut self.confirm_btn, styles),
            ButtonType::Back => apply_state(&mut self.back_btn, styles),
        }
    }

    pub fn set_button_state(&mut self, ctx: &mut EventCtx, button: ButtonType, state: ButtonState) {
        let apply_state =
            |btn: &mut KeypadButton, state: ButtonState, ctx: &mut EventCtx| match state {
                ButtonState::Enabled => {
                    btn.button.show(ctx);
                    btn.inner_mut().enable(ctx);
                }
                ButtonState::Disabled => {
                    btn.button.show(ctx);
                    btn.inner_mut().disable(ctx);
                }
                ButtonState::Hidden => {
                    btn.button.hide(ctx);
                }
            };

        match button {
            ButtonType::Key(idx) => apply_state(&mut self.keys[idx], state, ctx),
            ButtonType::Erase => apply_state(&mut self.erase_btn, state, ctx),
            ButtonType::Cancel => apply_state(&mut self.cancel_btn, state, ctx),
            ButtonType::Confirm => apply_state(&mut self.confirm_btn, state, ctx),
            ButtonType::Back => apply_state(&mut self.back_btn, state, ctx),
        }
    }

    pub fn new(
        allow_cancel: bool,
        allow_back: bool,
        allow_erase: bool,
        allow_confirm: bool,
        allow_keys: [bool; KEY_COUNT],
    ) -> Self {
        // Make sure that the number of keys is less than or equal to 10.
        assert!(KEY_COUNT <= 10, "Too many key buttons");

        let back_btn = Button::with_icon(theme::ICON_CHEVRON_LEFT)
            .styled(theme::button_keyboard_numeric())
            .with_radius(Self::KEYBOARD_BUTTON_RADIUS)
            .initially_enabled(allow_back);

        let cancel_btn = Button::with_icon(theme::ICON_CROSS)
            .styled(theme::button_keyboard_cancel())
            .with_radius(Self::KEYBOARD_BUTTON_RADIUS)
            .initially_enabled(allow_cancel);

        let confirm_btn = Button::with_icon(theme::ICON_CHECKMARK)
            .styled(theme::button_keyboard_confirm())
            .initially_enabled(allow_confirm)
            .with_radius(Self::KEYBOARD_BUTTON_RADIUS)
            .initially_enabled(allow_confirm);

        let erase_btn = Button::with_icon(theme::ICON_DELETE)
            .styled(theme::button_keyboard())
            .with_long_press(theme::ERASE_HOLD_DURATION)
            .initially_enabled(allow_erase)
            .with_radius(Self::KEYBOARD_BUTTON_RADIUS)
            .initially_enabled(allow_erase);

        // Create the array of buttons.
        let keys: [KeypadButton; KEY_COUNT] = core::array::from_fn(|idx| {
            let btn = Button::empty()
                .with_radius(Self::KEYBOARD_BUTTON_RADIUS)
                .styled(theme::button_keyboard_numeric())
                .with_text_align(Alignment::Center)
                .initially_enabled(allow_keys[idx]);
            KeypadButton::shown(theme::BG, btn)
        });

        Self {
            back_btn: KeypadButton::new(theme::BG, back_btn, allow_back),
            cancel_btn: KeypadButton::new(theme::BG, cancel_btn, allow_cancel),
            confirm_btn: KeypadButton::new(theme::BG, confirm_btn, allow_confirm),
            erase_btn: KeypadButton::new(theme::BG, erase_btn, allow_erase),
            keys,
            pressed: None,
        }
    }

    pub fn new_empty(allow_cancel: bool) -> Self {
        Self::new(allow_cancel, false, false, false, [true; KEY_COUNT])
    }

    fn render_pressed_button<'s>(&'s self, target: &mut impl Renderer<'s>) {
        match self.pressed {
            Some(ButtonType::Key(idx)) => {
                self.keys[idx].render(target);
            }
            Some(ButtonType::Cancel) => {
                self.cancel_btn.render(target);
            }
            Some(ButtonType::Erase) => {
                self.erase_btn.render(target);
            }
            Some(ButtonType::Confirm) => {
                self.confirm_btn.render(target);
            }
            Some(ButtonType::Back) => {
                self.back_btn.render(target);
            }
            None => {}
        }
    }
}

pub struct OverlapGrid<const ROWS: usize, const COLS: usize> {
    visible_area: Rect,
    border_rects: [[Rect; COLS]; ROWS],
    touch_insets: [[Insets; COLS]; ROWS],
    pub lr_half_overlap: i16,
    pub ud_half_spacing: i16,
}

impl<const ROWS: usize, const COLS: usize> OverlapGrid<ROWS, COLS> {
    const BUTTON_WIDTH: i16 = 121;
    const BUTTON_HEIGHT: i16 = 70;
    /// Creates a new `KeyboardGrid` with fixed dimensions of 4x3.
    pub const fn new(visible_area: Rect) -> Self {
        assert!(ROWS > 0);
        assert!(COLS > 0);

        let ud_half_spacing =
            (visible_area.height() - ROWS as i16 * Self::BUTTON_HEIGHT) / (ROWS as i16 + 1) / 2;
        let lr_half_overlap = (visible_area.width() - COLS as i16 * Self::BUTTON_WIDTH) / 2;

        let mut border_rects: [[Rect; COLS]; ROWS] = [[Rect::zero(); COLS]; ROWS];
        let mut touch_insets: [[Insets; COLS]; ROWS] = [[Insets::uniform(0); COLS]; ROWS];

        let mut row = 0;

        while row < ROWS {
            let mut col = 0;
            while col < COLS {
                touch_insets[row][col] = Insets::new(
                    -ud_half_spacing,
                    -lr_half_overlap,
                    -ud_half_spacing,
                    -lr_half_overlap,
                );
                if row == ROWS - 1 {
                    touch_insets[row][col].bottom = 0;
                }
                if col == 0 {
                    touch_insets[row][col].left = 0;
                } else if col == COLS - 1 {
                    touch_insets[row][col].right = 0;
                }
                border_rects[row][col] = Rect::from_top_left_and_size(
                    visible_area.top_left().ofs(Offset::new(
                        col as i16 * Self::BUTTON_WIDTH,
                        row as i16 * (Self::BUTTON_HEIGHT + 2 * ud_half_spacing),
                    )),
                    Offset::new(
                        Self::BUTTON_WIDTH + 2 * lr_half_overlap,
                        Self::BUTTON_HEIGHT + 4 * ud_half_spacing,
                    ),
                );
                col += 1;
            }
            row += 1;
        }

        Self {
            visible_area,
            border_rects,
            touch_insets,
            lr_half_overlap,
            ud_half_spacing,
        }
    }

    /// Retrieves the button border `Rect` at the specified index.
    pub const fn border_of_cell(&self, index: usize) -> Rect {
        let (row, col) = self.cell2row_col(index);
        self.border_of_row_col(row, col)
    }

    /// Converts a cell index to a (row, col) tuple.
    const fn cell2row_col(&self, index: usize) -> (usize, usize) {
        let row = index / COLS as usize;
        let col = index % COLS as usize;
        (row, col)
    }

    /// Retrieves the button border `Rect` for the given row and column.
    pub const fn border_of_row_col(&self, row: usize, col: usize) -> Rect {
        assert!(row < ROWS);
        assert!(col < COLS);
        self.border_rects[row][col]
    }

    /// Retrieves the button touch `Insets` at the specified index.
    pub const fn insets_of_cell(&self, index: usize) -> Insets {
        let (row, col) = self.cell2row_col(index);
        self.insets_of_row_col(row, col)
    }

    /// Retrieves the button touch `Insets` for the given row and column.
    pub const fn insets_of_row_col(&self, row: usize, col: usize) -> Insets {
        assert!(row < ROWS);
        assert!(col < COLS);
        self.touch_insets[row][col]
    }
}
