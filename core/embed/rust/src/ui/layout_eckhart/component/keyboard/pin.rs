use crate::{
    strutil::{ShortString, TString},
    time::Duration,
    trezorhal::random,
    ui::{
        component::{text::TextStyle, Component, Event, EventCtx, Label, Timer},
        display::Font,
        event::TouchEvent,
        geometry::{Alignment, Alignment2D, Insets, Offset, Rect},
        layout_eckhart::component::{
            button::ButtonContent,
            keyboard::{
                common::{
                    DisplayStyle, FADING_ICON_COLORS, FADING_ICON_COUNT, KEYBOARD_INPUT_INSETS,
                    KEYBOARD_INPUT_RADIUS, KEYBOARD_INPUT_TOUCH_HEIGHT, KEYBOARD_KEYPAD_HEIGHT,
                    KEYBOARD_PROMPT_INSETS,
                },
                keypad::{ButtonState, ButtonType, Keypad, KeypadMsg},
            },
            theme,
        },
        shape::{Bar, Renderer, Text, ToifImage},
        util::long_line_content_with_ellipsis,
    },
};

use heapless::Vec;

pub enum PinKeyboardMsg {
    Confirmed,
    Cancelled,
}

const MAX_LENGTH: usize = 50;
const MAX_SHOWN_LEN: usize = 19; // max number of digis per line
const LAST_DIGIT_TIMEOUT_S: u32 = 1;
const DIGITS: usize = 10;

const MAX_LINES: usize = 10;

pub struct PinKeyboard<'a> {
    allow_cancel: bool,
    major_prompt: Label<'a>,
    minor_prompt: Label<'a>,
    major_warning: Option<Label<'a>>,
    keypad: Keypad<DIGITS>,
    textbox: PinInput,
    warning_timer: Timer,
    close_confirm: bool,
}

impl<'a> PinKeyboard<'a> {
    pub fn new(
        major_prompt: TString<'a>,
        minor_prompt: TString<'a>,
        major_warning: Option<TString<'a>>,
        allow_cancel: bool,
    ) -> Self {
        Self {
            allow_cancel,
            major_prompt: Label::left_aligned(major_prompt, theme::TEXT_SMALL),
            minor_prompt: Label::right_aligned(minor_prompt, theme::TEXT_SMALL),
            major_warning: major_warning.map(|text| Label::left_aligned(text, theme::TEXT_SMALL)),
            textbox: PinInput::new(theme::TEXT_MONO_LIGHT),
            keypad: Keypad::new_empty(allow_cancel)
                .with_keypad_content(Self::keypad_content(), theme::button_keyboard_numeric()),
            warning_timer: Timer::new(),
            close_confirm: false,
        }
    }

    fn keypad_content() -> Vec<ButtonContent, DIGITS> {
        let mut digits = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
        random::shuffle(&mut digits);

        digits
            .iter()
            .map(|&digit| ButtonContent::Text(digit.into()))
            .collect()
    }

    fn pin_modified(&mut self, ctx: &mut EventCtx) {
        let (keys_state, erase_state, confirm_state, cancel_state) =
            match (self.textbox.is_full(), self.textbox.is_empty()) {
                (true, _) => {
                    // Full textbox
                    (
                        ButtonState::Disabled, // Disable all key buttons
                        ButtonState::Enabled,  // Enable erase button
                        ButtonState::Enabled,  // Enable confirm button
                        ButtonState::Hidden,   // Hide cancel button
                    )
                }
                (_, true) => {
                    // Empty textbox
                    (
                        ButtonState::Enabled,  // Enable all key buttons
                        ButtonState::Hidden,   // Hide erase button
                        ButtonState::Disabled, // Disable confirm button
                        if self.allow_cancel {
                            ButtonState::Enabled // Enable cancel button if
                                                 // allowed
                        } else {
                            ButtonState::Hidden // Hide cancel button if not
                                                // allowed
                        },
                    )
                }
                _ => {
                    // Partially filled textbox
                    (
                        ButtonState::Enabled, // Enable all key buttons
                        ButtonState::Enabled, // Enable erase button
                        ButtonState::Enabled, // Enable confirm button
                        ButtonState::Hidden,  // Hide cancel button
                    )
                }
            };

        // Apply all button states in one place
        self.keypad.set_keys_state(ctx, &keys_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Erase, erase_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Confirm, confirm_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Cancel, cancel_state);
    }

    pub fn pin(&self) -> &str {
        self.textbox.pin()
    }
}

impl Component for PinKeyboard<'_> {
    type Msg = PinKeyboardMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        // Prompts and PIN dots display.
        // let (input_area, _) = bounds.split_top(KEYBOARD_INPUT_HEIGHT);

        let (_, keypad_area) = bounds.split_bottom(KEYBOARD_KEYPAD_HEIGHT);

        let (input_touch_area, _) = bounds.split_top(KEYBOARD_INPUT_TOUCH_HEIGHT);

        // Prompts and PIN dots display.
        self.textbox.place(input_touch_area);
        self.major_prompt
            .place(input_touch_area.inset(KEYBOARD_PROMPT_INSETS));
        self.minor_prompt
            .place(input_touch_area.inset(KEYBOARD_PROMPT_INSETS));
        self.major_warning
            .as_mut()
            .map(|c| c.place(input_touch_area));

        // Keypad placement
        self.keypad.place(keypad_area);

        bounds
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        match event {
            // Set up timer to switch off warning prompt.
            Event::Attach(_) if self.major_warning.is_some() => {
                self.warning_timer.start(ctx, Duration::from_secs(2));
            }
            // Hide warning, show major prompt.
            Event::Timer(_) if self.warning_timer.expire(event) => {
                self.major_warning = None;
            }

            _ => {}
        }

        match self.keypad.event(ctx, event) {
            Some(KeypadMsg::KeyClicked(idx)) => {
                if let ButtonContent::Text(text) = self.keypad.get_key_content(idx) {
                    text.map(|text| {
                        self.textbox.push(text);
                    });
                    self.textbox
                        .last_digit_timer
                        .start(ctx, Duration::from_secs(LAST_DIGIT_TIMEOUT_S));
                    self.textbox.display_style = DisplayStyle::LastOnly;
                    self.pin_modified(ctx);
                    return None;
                }
            }
            Some(KeypadMsg::EraseClicked) => {
                self.textbox.pop();
                self.pin_modified(ctx);
                return None;
            }
            Some(KeypadMsg::EraseLongPressed) => {
                self.textbox.clear();
                self.pin_modified(ctx);
                return None;
            }
            Some(KeypadMsg::CancelClicked) => {
                return Some(PinKeyboardMsg::Cancelled);
            }
            Some(KeypadMsg::ConfirmClicked) => {
                return Some(PinKeyboardMsg::Confirmed);
            }
            _ => {}
        }

        match self.textbox.event(ctx, event) {
            Some(PinInputMsg::TouchStart) => {
                self.textbox.display_style = DisplayStyle::Shown;

                self.keypad
                    .set_button_state(ctx, ButtonType::Confirm, ButtonState::Disabled);
                self.keypad
                    .set_button_state(ctx, ButtonType::Erase, ButtonState::Disabled);

                self.keypad.set_keys_state(ctx, &ButtonState::Disabled);
                return None;
            }
            Some(PinInputMsg::TouchEnd) => {
                self.textbox.display_style = DisplayStyle::Hidden;
                // Enable back the buttons.
                self.keypad
                    .set_button_state(ctx, ButtonType::Confirm, ButtonState::Enabled);
                self.keypad
                    .set_button_state(ctx, ButtonType::Erase, ButtonState::Enabled);

                if !self.textbox.is_full() {
                    self.keypad.set_keys_state(ctx, &ButtonState::Enabled);
                }

                return None;
            }
            Some(PinInputMsg::LasDigitTimeout) => {
                self.textbox.display_style = DisplayStyle::Hidden;
                ctx.request_paint();
                return None;
            }

            _ => {}
        }

        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        let empty = self.textbox.is_empty();

        // Render prompt when the pin is empty
        if empty {
            if let Some(ref w) = self.major_warning {
                w.render(target);
            } else {
                self.major_prompt.render(target);
            }
            self.minor_prompt.render(target);
        }

        // When the entire pin is shown, the input area might overlap the keypad so it
        // has to be render later
        match self.textbox.display_style {
            DisplayStyle::Shown if !empty => {
                self.keypad.render(target);
                self.textbox.render(target);
            }
            _ if !empty => {
                self.textbox.render(target);
                self.keypad.render(target);
            }
            _ => {
                self.keypad.render(target);
            }
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
#[cfg_attr(feature = "ui_debug", derive(ufmt::derive::uDebug))]
pub enum PinInputMsg {
    TouchStart,
    TouchEnd,
    LasDigitTimeout,
}

struct PinInput {
    area: Rect,
    style: TextStyle,
    digits: ShortString,
    display_style: DisplayStyle,
    last_digit_timer: Timer,
}

impl PinInput {
    const TWITCH: i16 = 4;
    const X_STEP: i16 = 12;
    const Y_STEP: i16 = 32;

    fn new(style: TextStyle) -> Self {
        Self {
            area: Rect::zero(),
            style,
            digits: ShortString::new(),
            display_style: DisplayStyle::Hidden,
            last_digit_timer: Timer::new(),
        }
    }

    fn size(&self) -> Offset {
        let ndots = self.digits.chars().count().min(MAX_SHOWN_LEN);
        let mut width = 10 * (ndots as i16);
        width += 3 * (ndots.saturating_sub(1) as i16);
        Offset::new(width, 6)
    }

    fn is_empty(&self) -> bool {
        self.digits.is_empty()
    }

    fn is_full(&self) -> bool {
        self.digits.len() >= MAX_LENGTH
    }

    fn clear(&mut self) {
        self.digits.clear();
    }

    fn push(&mut self, text: &str) {
        // This could happen only when `self.pin` is full and wasn't able to accept all
        // of `text`
        self.digits.push_str(text).unwrap();
    }

    fn pop(&mut self) {
        self.digits.pop();
    }

    fn pin(&self) -> &str {
        &self.digits
    }

    fn render_shown<'s>(&self, area: Rect, target: &mut impl Renderer<'s>) {
        assert_eq!(self.display_style, DisplayStyle::Shown);

        let style = theme::TEXT_MEDIUM;

        let lines = Self::split_text_into_lines(self.pin(), style.text_font, area.width());
        let line_num = lines.len();

        let area = area.outset(Insets::bottom((line_num as i16 - 1) * 32));

        Bar::new(area)
            .with_bg(theme::GREY_SUPER_DARK)
            .with_radius(KEYBOARD_INPUT_RADIUS)
            .render(target);

        if line_num == 1 {
            let center = area.center() + Offset::y(style.text_font.text_height() / 2);
            Text::new(center, &self.digits, style.text_font)
                .with_align(Alignment::Center)
                .with_fg(self.style.text_color)
                .render(target);
        } else {
            let mut text_baseline = area.top_left() + Offset::y(20 + style.text_font.text_height());

            for line in lines.into_iter() {
                Text::new(text_baseline, line, style.text_font)
                    .with_fg(style.text_color)
                    .render(target);
                text_baseline.y += Self::Y_STEP;
            }
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

    fn render_hidden<'s>(&self, area: Rect, target: &mut impl Renderer<'s>) {
        assert_ne!(self.display_style, DisplayStyle::Shown);

        let style = theme::TEXT_MEDIUM;
        let icon = theme::ICON_DASH_VERTICAL.toif;

        let mut cursor = self.size().snap(area.center(), Alignment2D::CENTER);

        let pin_len = self.digits.chars().count();
        let last_digit = self.display_style == DisplayStyle::LastOnly;

        if pin_len > 0 {
            // Find out how much text can fit into the textbox.
            // Accounting for the pending marker, which draws itself one extra pixel
            let truncated = long_line_content_with_ellipsis(
                self.digits.as_str(),
                "",
                style.text_font,
                area.width() - 1,
            );
            let shown_len = truncated.len();
            let visible_icons = shown_len - last_digit as usize;

            // Jiggle when overflowed.
            if pin_len > shown_len
                && pin_len % 2 == 0
                && (self.display_style == DisplayStyle::Hidden
                    || self.display_style == DisplayStyle::LastOnly)
            {
                cursor.x += Self::TWITCH;
            }

            let mut char_idx = 0;

            // Greyed out overflowing icons
            for (i, &fg_color) in FADING_ICON_COLORS.iter().enumerate() {
                if pin_len > shown_len + (FADING_ICON_COUNT - 1 - i) {
                    ToifImage::new(cursor, icon)
                        .with_align(Alignment2D::TOP_LEFT)
                        .with_fg(fg_color)
                        .render(target);
                    cursor.x += Self::X_STEP;
                    char_idx += 1;
                }
            }

            if visible_icons > 0 {
                // Classical dot(s)
                for _ in char_idx..visible_icons {
                    ToifImage::new(cursor, icon)
                        .with_align(Alignment2D::TOP_LEFT)
                        .with_fg(style.text_color)
                        .render(target);
                    cursor.x += Self::X_STEP;
                }
            }

            if last_digit {
                // Adapt y position for the character
                cursor.y = area.top_left().y + 26 + style.text_font.text_height();
                // This should not fail because all_chars > 0
                let last = &self.digits.as_str()[(pin_len - 1)..pin_len];
                // Paint the last character
                Text::new(cursor, last, style.text_font)
                    .with_align(Alignment::Start)
                    .with_fg(style.text_color)
                    .render(target);
            }
        }
    }
}

impl Component for PinInput {
    type Msg = PinInputMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        // self.pad.place(bounds);
        self.area = bounds;
        self.area
    }

    fn event(&mut self, _ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        // No touch events are handled when the textbox is empty
        if self.is_empty() {
            return None;
        }

        match event {
            Event::Touch(TouchEvent::TouchStart(pos)) if self.area.contains(pos) => {
                Some(PinInputMsg::TouchStart)
            }
            Event::Touch(TouchEvent::TouchEnd(pos)) if self.area.contains(pos) => {
                Some(PinInputMsg::TouchEnd)
            }
            // Timeout for showing the last digit.
            Event::Timer(_) if self.last_digit_timer.expire(event) => {
                Some(PinInputMsg::LasDigitTimeout)
            }
            _ => None,
        }
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        let render_area: Rect = self.area.inset(KEYBOARD_INPUT_INSETS);

        if !self.digits.is_empty() {
            match self.display_style {
                DisplayStyle::Shown => self.render_shown(render_area, target),
                _ => self.render_hidden(render_area, target),
            }
        }
    }
}

#[cfg(feature = "ui_debug")]
impl crate::trace::Trace for PinKeyboard<'_> {
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("PinKeyboard");
        // So that debuglink knows the locations of the buttons
        let mut digits_order = ShortString::new();

        for idx in 0..DIGITS {
            let btn_content = self.keypad.get_key_content(idx);
            if let ButtonContent::Text(text) = btn_content {
                text.map(|text| {
                    unwrap!(digits_order.push_str(text));
                });
            }
        }
        let display_style = uformat!("{:?}", self.textbox.display_style);
        t.string("digits_order", digits_order.as_str().into());
        t.string("pin", self.textbox.pin().into());
        t.string("display_style", display_style.as_str().into());
    }
}
