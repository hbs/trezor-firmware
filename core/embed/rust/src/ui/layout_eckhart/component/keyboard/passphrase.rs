use crate::{
    strutil::{ShortString, TString},
    time::Duration,
    translations::TR,
    ui::{
        component::{
            swipe_detect::SwipeConfig, text::common::TextBox, Component, Event, EventCtx, Label,
            Swipe, Timer,
        },
        display::{self, Font},
        event::TouchEvent,
        geometry::{Alignment, Alignment2D, Direction, Insets, Offset, Rect},
        layout_eckhart::component::{
            button::{Button, ButtonContent, ButtonMsg, ButtonStyleSheet},
            keyboard::{
                common::{
                    render_pending_marker, DisplayStyle, MultiTapKeyboard, KEYBOARD_INPUT_HEIGHT,
                    KEYBOARD_INPUT_INSETS, KEYBOARD_KEYPAD_HEIGHT,
                },
                keypad::{ButtonState, ButtonType, Keypad, KeypadMsg},
            },
            theme,
        },
        shape::{self, Renderer},
        util::long_line_content_with_ellipsis,
    },
};

use core::cell::Cell;
use heapless::Vec;
use num_traits::ToPrimitive;

pub enum PassphraseKeyboardMsg {
    Confirmed(ShortString),
    Cancelled,
}

/// Enum keeping track of which keyboard is shown and which comes next. Keep the
/// number of values and the constant PAGE_COUNT in synch.
#[repr(u32)]
#[derive(Copy, Clone, ToPrimitive)]
enum KeyboardLayout {
    LettersLower = 0,
    LettersUpper = 1,
    Numeric = 2,
    Special = 3,
}

impl KeyboardLayout {
    fn next(self) -> Self {
        match self {
            Self::LettersLower => Self::LettersUpper,
            Self::LettersUpper => Self::Numeric,
            Self::Numeric => Self::Special,
            Self::Special => Self::LettersLower,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::LettersLower => Self::Special,
            Self::LettersUpper => Self::LettersLower,
            Self::Numeric => Self::LettersUpper,
            Self::Special => Self::Numeric,
        }
    }
}

impl From<KeyboardLayout> for ButtonContent {
    /// Used to get content for the "next keyboard" button
    fn from(kl: KeyboardLayout) -> Self {
        match kl {
            KeyboardLayout::LettersLower => ButtonContent::Text("abc".into()),
            KeyboardLayout::LettersUpper => ButtonContent::Text("ABC".into()),
            KeyboardLayout::Numeric => ButtonContent::Text("123".into()),
            KeyboardLayout::Special => ButtonContent::Icon(theme::ICON_ASTERISK),
        }
    }
}

pub struct PassphraseKeyboard {
    page_swipe: Swipe,
    input: PassphraseInput,
    input_prompt: Label<'static>,
    keypad: Keypad<KEY_COUNT>,
    next_btn: Button,
    active_layout: KeyboardLayout,
    fade: Cell<bool>,
    swipe_config: SwipeConfig, // FIXME: how about page_swipe
    internal_page_cnt: usize,
    multi_tap: MultiTapKeyboard,
}

const PAGE_COUNT: usize = 4;
const KEY_COUNT: usize = 10;
#[rustfmt::skip]
const KEYBOARD: [[&str; KEY_COUNT]; PAGE_COUNT] = [
    ["abc", "def", "ghi", "jkl", "mno", "pq", "rst", "uvw", "xyz", " *#"],
    ["ABC", "DEF", "GHI", "JKL", "MNO", "PQ", "RST", "UVW", "XYZ", " *#"],
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
    ["_<>", ".:@", "/|\\", "!()", "+%&", "-[]", "?{}", ",'`", ";\"~", "$^="],
    ];

const MAX_LENGTH: usize = 50;
const LAST_DIGIT_TIMEOUT_S: u32 = 1;

const NEXT_BTN_WIDTH: i16 = 103;
const NEXT_BTN_INSET: i16 = 14;
const NEXT_BTN_INSETS: Insets = Insets::new(NEXT_BTN_INSET, NEXT_BTN_INSET, 0, NEXT_BTN_INSET);

const MAX_LINES: usize = 10;

impl PassphraseKeyboard {
    pub fn new() -> Self {
        let active_layout = KeyboardLayout::LettersLower;
        let layout = &KEYBOARD[active_layout.to_usize().unwrap()];

        let keypad_content = layout
            .iter()
            .map(|&text| Self::key_content(text))
            .collect::<Vec<ButtonContent, KEY_COUNT>>();

        let next_btn = Button::new(active_layout.next().into())
            .styled(theme::button_keyboard_next())
            .with_radius(12)
            .with_text_align(Alignment::Center)
            .with_expanded_touch_area(Insets::new(
                NEXT_BTN_INSET,
                NEXT_BTN_INSET,
                0,
                NEXT_BTN_INSET,
            ));

        Self {
            page_swipe: Swipe::horizontal(),
            input: PassphraseInput::new(),
            input_prompt: Label::left_aligned(
                TString::from_translation(TR::passphrase__title_enter),
                theme::TEXT_NORMAL,
            ),
            next_btn,
            keypad: Keypad::new_empty(false)
                .with_keypad_content(keypad_content, theme::button_keyboard()),
            active_layout,
            fade: Cell::new(false),
            swipe_config: SwipeConfig::new(),
            internal_page_cnt: 1,
            multi_tap: MultiTapKeyboard::new(),
        }
    }

    fn key_text(content: &ButtonContent) -> TString<'static> {
        match content {
            ButtonContent::Text(text) => *text,
            ButtonContent::Icon(theme::ICON_SPECIAL_CHARS) => " *#".into(),
            ButtonContent::Icon(_) => " ".into(),
            ButtonContent::IconAndText(_) => " ".into(),
            ButtonContent::Empty => "".into(),
        }
    }

    fn key_content(text: &'static str) -> ButtonContent {
        match text {
            " *#" => ButtonContent::Icon(theme::ICON_SPECIAL_CHARS),
            t => ButtonContent::Text(t.into()),
        }
    }

    fn key_style(layout: KeyboardLayout) -> ButtonStyleSheet {
        let layout = layout.to_usize().unwrap();
        if layout == KeyboardLayout::Numeric.to_usize().unwrap() {
            theme::button_keyboard_numeric()
        } else {
            theme::button_keyboard()
        }
    }

    fn on_page_change(&mut self, ctx: &mut EventCtx, swipe: Direction) {
        // Change the keyboard layout.
        self.active_layout = match swipe {
            Direction::Left => self.active_layout.next(),
            Direction::Right => self.active_layout.prev(),
            _ => self.active_layout,
        };
        if self.multi_tap.pending_key().is_some() {
            // Clear the pending state.
            self.multi_tap.clear_pending_state(ctx);
            // the character has been added, show it for a bit and then hide it
            self.input
                .last_char_timer
                .start(ctx, Duration::from_secs(LAST_DIGIT_TIMEOUT_S));
        }
        // Update keys.
        self.replace_keys_contents();
        // Reset backlight to normal level on next paint.
        self.fade.set(true);
        // So that swipe does not visually enable the input buttons when max length
        // reached
        self.update_input_btns_state(ctx);
    }

    fn replace_keys_contents(&mut self) {
        self.next_btn.set_content(self.active_layout.next().into());
        let layout = self.active_layout.to_usize().unwrap();
        let styles = Self::key_style(self.active_layout);

        for idx in 0..KEY_COUNT {
            let text = KEYBOARD[layout][idx];
            let content = Self::key_content(text);
            self.keypad.set_key_content(idx, content);
            self.keypad
                .set_button_stylesheet(ButtonType::Key(idx), styles);
        }
    }

    /// Possibly changing the buttons' state after change of the input.
    fn after_edit(&mut self, ctx: &mut EventCtx) {
        let (confirm_state, erase_state, cancel_state) = match self.input.textbox.is_empty() {
            true => (
                ButtonState::Hidden,  // Confirm button is hidden
                ButtonState::Hidden,  // Erase button is hidden
                ButtonState::Enabled, // Cancel button is enabled
            ),
            false => (
                ButtonState::Enabled, // Confirm button is enabled
                ButtonState::Enabled, // Erase button is enabled
                ButtonState::Hidden,  // Cancel button is hidden
            ),
        };

        // Apply the button states
        self.keypad
            .set_button_state(ctx, ButtonType::Confirm, confirm_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Erase, erase_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Cancel, cancel_state);

        self.update_input_btns_state(ctx);
    }

    /// When the input has reached max length, disable all the input buttons.
    fn update_input_btns_state(&mut self, ctx: &mut EventCtx) {
        let active_states = self.get_buttons_active_states();

        for idx in 0..KEY_COUNT {
            let state = if active_states[idx] {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            };
            self.keypad
                .set_button_state(ctx, ButtonType::Key(idx), state);
        }
    }

    /// Precomputing the active states not to overlap borrows in
    /// `self.keys.iter_mut` loop.
    fn get_buttons_active_states(&self) -> [bool; KEY_COUNT] {
        let mut active_states: [bool; KEY_COUNT] = [false; KEY_COUNT];
        for (key, state) in active_states.iter_mut().enumerate() {
            *state = self.is_button_active(key);
        }
        active_states
    }

    /// We should disable the input when the passphrase has reached maximum
    /// length and we are not cycling through the characters.
    fn is_button_active(&self, key: usize) -> bool {
        let textbox_not_full = self.input.textbox.len() < MAX_LENGTH;
        let key_is_pending = {
            if let Some(pending) = self.multi_tap.pending_key() {
                pending == key
            } else {
                false
            }
        };
        textbox_not_full || key_is_pending
    }

    pub fn passphrase(&self) -> &str {
        self.input.textbox.content()
    }
}

impl Component for PassphraseKeyboard {
    type Msg = PassphraseKeyboardMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        // Area for the input and the next button.
        let (top_area, _) = bounds.split_top(KEYBOARD_INPUT_HEIGHT);

        let (_, keypad_area) = bounds.split_bottom(KEYBOARD_KEYPAD_HEIGHT);

        let (input_touch_area, next_btn_touch_area) =
            top_area.split_right(NEXT_BTN_WIDTH + 2 * NEXT_BTN_INSET);

        let next_btn_area = next_btn_touch_area.inset(NEXT_BTN_INSETS);

        self.input.total_area = top_area;

        self.page_swipe.place(bounds);
        self.input.place(input_touch_area);
        self.input_prompt.place(input_touch_area);

        self.keypad.place(keypad_area);

        self.next_btn.place(next_btn_area);

        bounds
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        match event {
            Event::Timer(_) if self.multi_tap.timeout_event(event) => {
                self.multi_tap.clear_pending_state(ctx);
                self.input
                    .last_char_timer
                    .start(ctx, Duration::from_secs(LAST_DIGIT_TIMEOUT_S));
                self.input.marker = false;
                return None;
            }

            _ => {}
        }

        if let Some(swipe) = self.page_swipe.event(ctx, event) {
            // We have detected a horizontal swipe. Change the keyboard page.
            self.on_page_change(ctx, swipe);
            return None;
        }

        if let Some(ButtonMsg::Clicked) = self.next_btn.event(ctx, event) {
            self.on_page_change(ctx, Direction::Left);
        }

        match self.keypad.event(ctx, event) {
            Some(KeypadMsg::KeyClicked(idx)) => {
                let text = Self::key_text(self.keypad.get_key_content(idx));
                let edit = text.map(|c| self.multi_tap.click_key(ctx, idx, c));
                self.input.textbox.apply(ctx, edit);

                self.after_edit(ctx);
                if text.len() == 1 {
                    // If the key has just one character, it is immediately applied and the last
                    // digit timer should be started
                    self.input
                        .last_char_timer
                        .start(ctx, Duration::from_secs(LAST_DIGIT_TIMEOUT_S));
                } else {
                    // multi tap timer is runnig, the last digit timer should be stopped
                    self.input.last_char_timer.stop();
                    self.input.marker = true;
                }
                self.input.display_style = DisplayStyle::LastOnly;

                return None;
            }
            Some(KeypadMsg::EraseClicked) => {
                self.input.display_style = DisplayStyle::Hidden;
                self.multi_tap.clear_pending_state(ctx);
                self.input.textbox.delete_last(ctx);
                self.after_edit(ctx);
                return None;
            }
            Some(KeypadMsg::EraseLongPressed) => {
                self.multi_tap.clear_pending_state(ctx);
                self.input.textbox.clear(ctx);
                self.after_edit(ctx);
                self.input.display_style = DisplayStyle::Hidden;
                return None;
            }
            Some(KeypadMsg::CancelClicked) => {
                return Some(PassphraseKeyboardMsg::Cancelled);
            }
            Some(KeypadMsg::ConfirmClicked) => {
                return Some(PassphraseKeyboardMsg::Confirmed(unwrap!(
                    ShortString::try_from(self.passphrase())
                )));
            }
            _ => {}
        }

        match self.input.event(ctx, event) {
            Some(PassphraseInputMsg::TouchStart) => {
                if self.multi_tap.pending_key().is_some() {
                    // Clear the pending state.
                    self.multi_tap.clear_pending_state(ctx);
                }
                self.input.display_style = DisplayStyle::Shown;
                ctx.request_paint();
                return None;
            }
            Some(PassphraseInputMsg::TouchEnd) => {
                self.input.display_style = DisplayStyle::Hidden;
                ctx.request_paint();
                return None;
            }
            Some(PassphraseInputMsg::LasCharTimeout) => {
                self.input.display_style = DisplayStyle::Hidden;
                ctx.request_paint();
                return None;
            }
            _ => {}
        }

        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        match self.input.display_style {
            DisplayStyle::Shown => {
                self.keypad.render(target);
                self.input.render(target);
            }
            _ => {
                self.input.render(target);
                self.next_btn.render(target);
                self.keypad.render(target);
            }
        }

        if self.fade.take() {
            // Note that this is blocking and takes some time.
            display::fade_backlight(theme::backlight::get_backlight_normal());
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
#[cfg_attr(feature = "ui_debug", derive(ufmt::derive::uDebug))]
pub enum PassphraseInputMsg {
    TouchStart,
    TouchEnd,
    LasCharTimeout,
}

struct PassphraseInput {
    touch_area: Rect,
    total_area: Rect,
    textbox: TextBox,
    display_style: DisplayStyle,
    marker: bool,
    last_char_timer: Timer,
}

impl PassphraseInput {
    const TWITCH: i16 = 4;
    const X_STEP: i16 = 12;
    const Y_STEP: i16 = 32;

    const ONE_LINE: i16 = 72;
    const TWO_LINES: i16 = 104;

    fn new() -> Self {
        Self {
            touch_area: Rect::zero(),
            total_area: Rect::zero(),
            textbox: TextBox::empty(MAX_LENGTH),
            display_style: DisplayStyle::LastOnly,
            marker: false,
            last_char_timer: Timer::new(),
        }
    }

    fn render_shown<'s>(&self, target: &mut impl Renderer<'s>) {
        assert!(!self.textbox.is_empty());

        let area = self.touch_area.inset(KEYBOARD_INPUT_INSETS);

        let style = theme::TEXT_NORMAL;
        let mut text_baseline = area.top_left() + Offset::y(20 + style.text_font.text_height());

        let outsets = Insets::new(0, 80, 0, 1);
        let area = area.outset(outsets);

        let lines =
            Self::split_text_into_lines(self.textbox.content(), style.text_font, area.width());
        let line_num = lines.len();

        let area = area.outset(Insets::bottom((line_num as i16 - 1) * 32));

        shape::Bar::new(area)
            .with_bg(theme::GREY_SUPER_DARK)
            .with_radius(12)
            .render(target);

        for line in lines.into_iter() {
            shape::Text::new(text_baseline, line, style.text_font)
                .with_fg(style.text_color)
                .render(target);
            text_baseline.y += Self::Y_STEP;
        }
    }

    fn split_text_into_lines<'a>(
        text: &'a str,
        text_font: Font,
        line_width: i16,
    ) -> Vec<&'a str, MAX_LINES> {
        let mut lines: Vec<&'a str, MAX_LINES> = Vec::new();
        let mut remaining_text = text;

        while !remaining_text.is_empty() && lines.len() < MAX_LINES {
            let chars_fit = text_font.longest_prefix_length(line_width, remaining_text);
            let line = &remaining_text[..chars_fit];
            lines.push(line).unwrap(); // should not fail because we checked the length
            remaining_text = &remaining_text[chars_fit..];
        }

        lines
    }

    fn render_hidden<'s>(&self, target: &mut impl Renderer<'s>) {
        let area = self.touch_area;
        let style = theme::TEXT_NORMAL;
        let bullet = theme::ICON_DASH_VERTICAL.toif;
        let mut cursor = area.left_center();
        let all_chars = self.textbox.content().len();
        let last_char = self.display_style == DisplayStyle::LastOnly;

        if all_chars > 0 {
            // Find out how much text can fit into the textbox.
            // Accounting for the pending marker, which draws itself one extra pixel
            let truncated = long_line_content_with_ellipsis(
                self.textbox.content(),
                "",
                style.text_font,
                area.width() - 1,
            );
            let visible_chars = truncated.len();
            let visible_dots = visible_chars - last_char as usize;

            // Jiggle when overflowed.
            if all_chars > visible_chars
                && all_chars % 2 == 0
                && (self.display_style == DisplayStyle::Hidden
                    || self.display_style == DisplayStyle::LastOnly)
            {
                cursor.x += Self::TWITCH;
            }

            let mut char_idx = 0;

            // Greyed out overflowing icons
            let fg_colors = [
                theme::GREY_SUPER_DARK,
                theme::GREY_EXTRA_DARK,
                theme::GREY_DARK,
                theme::GREY,
            ];

            for (i, &fg_color) in fg_colors.iter().enumerate() {
                if all_chars > visible_chars + (3 - i) {
                    shape::ToifImage::new(cursor, theme::ICON_DASH_VERTICAL.toif)
                        .with_align(Alignment2D::TOP_LEFT)
                        .with_fg(fg_color)
                        .render(target);
                    cursor.x += Self::X_STEP;
                    char_idx += 1;
                }
            }

            if visible_dots > 0 {
                // Classical dot(s)
                for _ in char_idx..visible_dots {
                    shape::ToifImage::new(cursor, bullet)
                        .with_align(Alignment2D::TOP_LEFT)
                        .with_fg(style.text_color)
                        .render(target);
                    cursor.x += Self::X_STEP;
                }
            }

            if last_char {
                // Adapt y position for the character
                cursor.y = area.top_left().y + style.text_font.text_height() + 30;
                // This should not fail because all_chars > 0
                let last = &self.textbox.content()[(all_chars - 1)..all_chars];
                // Paint the last character
                shape::Text::new(cursor, last, style.text_font)
                    .with_align(Alignment::Start)
                    .with_fg(style.text_color)
                    .render(target);
                // Paint the pending marker.
                if self.marker {
                    render_pending_marker(target, cursor, last, style.text_font, style.text_color);
                }
            }
        }
    }
}

impl Component for PassphraseInput {
    type Msg = PassphraseInputMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        self.touch_area = bounds;
        self.touch_area
    }

    fn event(&mut self, _ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        // No touch events are handled when the textbox is empty
        if self.textbox.is_empty() {
            return None;
        }

        match event {
            Event::Touch(TouchEvent::TouchStart(pos)) if self.touch_area.contains(pos) => {
                Some(PassphraseInputMsg::TouchStart)
            }
            Event::Touch(TouchEvent::TouchEnd(pos)) if self.touch_area.contains(pos) => {
                Some(PassphraseInputMsg::TouchEnd)
            }
            // Timeout for showing the last digit.
            Event::Timer(_) if self.last_char_timer.expire(event) => {
                Some(PassphraseInputMsg::LasCharTimeout)
            }
            _ => None,
        }
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        if !self.textbox.content().is_empty() {
            match self.display_style {
                DisplayStyle::Shown => self.render_shown(target),
                _ => self.render_hidden(target),
            }
        }
    }
}

#[cfg(feature = "micropython")]
impl crate::ui::flow::Swipable for PassphraseKeyboard {
    fn get_swipe_config(&self) -> SwipeConfig {
        self.swipe_config
    }

    fn get_internal_page_count(&self) -> usize {
        self.internal_page_cnt
    }
}

#[cfg(feature = "ui_debug")]
impl crate::trace::Trace for PassphraseKeyboard {
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("PassphraseKeyboard");
        t.string("passphrase", self.passphrase().into());
    }
}
