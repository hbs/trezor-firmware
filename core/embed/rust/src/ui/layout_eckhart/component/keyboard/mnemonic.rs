use crate::{
    strutil::TString,
    ui::{
        component::{Component, Event, EventCtx, Label, Maybe},
        geometry::{Insets, Rect},
        layout_eckhart::{
            component::{
                button::ButtonContent,
                keyboard::{
                    common::{
                        KEYBOARD_INPUT_HEIGHT, KEYBOARD_INPUT_INSETS, KEYBOARD_PROMPT_INSETS,
                    },
                    keypad::{ButtonState, ButtonType, Keypad, KeypadMsg},
                },
            },
            theme,
        },
        shape::Renderer,
    },
};

use heapless::Vec;

pub const MNEMONIC_KEY_COUNT: usize = 9;

const HEADER_INSETS: Insets = Insets::new(29, 18, 35, 18);

pub enum MnemonicKeyboardMsg {
    Confirmed,
    Previous,
}

pub struct MnemonicKeyboard<T> {
    /// Initial prompt, displayed on empty input.
    prompt: Maybe<Label<'static>>,
    /// Input area, acting as the auto-complete.
    input: Maybe<T>,
    /// Key buttons.
    keypad: Keypad<MNEMONIC_KEY_COUNT>,
    /// Whether going back is allowed (is not on the very first word).
    can_go_back: bool,
}

impl<T> MnemonicKeyboard<T>
where
    T: MnemonicInput,
{
    pub fn new(input: T, prompt: TString<'static>, can_go_back: bool) -> Self {
        // Input might be already pre-filled
        let prompt_visible = input.is_empty();

        let allow_keys: [_; MNEMONIC_KEY_COUNT] =
            core::array::from_fn(|idx| input.can_key_press_lead_to_a_valid_word(idx));

        let keypad_content = T::keys()
            .iter()
            .map(|&digit| ButtonContent::Text(digit.into()))
            .collect::<Vec<ButtonContent, MNEMONIC_KEY_COUNT>>();

        Self {
            prompt: Maybe::new(
                theme::BG,
                Label::centered(prompt, theme::TEXT_SMALL).vertically_centered(),
                prompt_visible,
            ),
            keypad: Keypad::<MNEMONIC_KEY_COUNT>::new(
                false,
                can_go_back && input.is_empty(),
                !input.is_empty(),
                input.can_be_confirmed(),
                allow_keys,
            )
            .with_keypad_content(keypad_content, theme::button_keyboard()),
            can_go_back,
            input: Maybe::new(theme::BG, input, !prompt_visible),
        }
    }

    fn on_input_change(&mut self, ctx: &mut EventCtx) {
        self.toggle_buttons(ctx);
        self.toggle_prompt_or_input(ctx);
    }

    /// Either enable or disable the key buttons, depending on the dictionary
    /// completion mask and the pending key.
    fn toggle_buttons(&mut self, ctx: &mut EventCtx) {
        // Enable or disable the key buttons based on their ability to form a valid
        // word.
        for idx in 0..MNEMONIC_KEY_COUNT {
            let enabled = self.input.inner().can_key_press_lead_to_a_valid_word(idx);

            let state = if enabled {
                ButtonState::Enabled
            } else {
                ButtonState::Disabled
            };
            self.keypad
                .set_button_state(ctx, ButtonType::Key(idx), state);
        }

        let input_empty = self.input.inner().is_empty();

        // Determine states for erase and back buttons
        let (erase_state, back_state) = match input_empty {
            true => (
                ButtonState::Hidden,
                if self.can_go_back {
                    ButtonState::Enabled
                } else {
                    ButtonState::Hidden
                },
            ),
            false => (ButtonState::Enabled, ButtonState::Hidden),
        };

        // Determine state and style for the confirm button based on input state

        let confirm_state =
            if self.input.inner().is_empty() || self.input.inner().mnemonic().is_none() {
                ButtonState::Hidden
            } else {
                ButtonState::Enabled
            };

        let confirm_style = if self.input.inner().mnemonic().is_some() {
            let any_press_can_lead_to_valid_word = || {
                (0..MNEMONIC_KEY_COUNT)
                    .any(|idx| self.input.inner().can_key_press_lead_to_a_valid_word(idx))
            };
            if any_press_can_lead_to_valid_word() {
                theme::button_keyboard()
            } else {
                theme::button_keyboard_confirm()
            }
        } else {
            theme::button_keyboard()
        };

        // Apply all button states
        self.keypad
            .set_button_state(ctx, ButtonType::Erase, erase_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Back, back_state);
        self.keypad
            .set_button_state(ctx, ButtonType::Confirm, confirm_state);

        // Apply the stylesheet for the confirm button
        self.keypad
            .set_button_stylesheet(ButtonType::Confirm, confirm_style);
    }

    /// After edit operations, we need to either show or hide the prompt, the
    /// input, the erase button and the back button.
    fn toggle_prompt_or_input(&mut self, ctx: &mut EventCtx) {
        let input_empty = self.input.inner().is_empty();
        // Prompt is shown if the input is empty.
        self.prompt.show_if(ctx, input_empty);
        self.input.show_if(ctx, !input_empty);
    }

    pub fn mnemonic(&self) -> Option<&'static str> {
        self.input.inner().mnemonic()
    }
}

impl<T> Component for MnemonicKeyboard<T>
where
    T: MnemonicInput,
{
    type Msg = MnemonicKeyboardMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        let (input_area, keypad_area) = bounds.split_top(KEYBOARD_INPUT_HEIGHT);

        let prompt_area = input_area.inset(KEYBOARD_PROMPT_INSETS);
        let input_area = input_area.inset(KEYBOARD_INPUT_INSETS);

        // Prompt/input placement
        self.prompt.place(prompt_area);
        self.input.place(input_area);

        // Keypad placement
        self.keypad.place(keypad_area);

        bounds
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        match self.input.event(ctx, event) {
            Some(MnemonicInputMsg::Confirmed) => {
                // Confirmed, bubble up.
                return Some(MnemonicKeyboardMsg::Confirmed);
            }
            Some(_) => {
                // Either a timeout or a completion.
                self.on_input_change(ctx);
                return None;
            }
            _ => {}
        }

        match self.keypad.event(ctx, event) {
            Some(KeypadMsg::KeyClicked(idx)) => {
                self.input.inner_mut().on_key_click(ctx, idx);
                self.on_input_change(ctx);
                return None;
            }
            Some(KeypadMsg::BackClicked) => {
                // Back button will cause going back to the previous word when allowed.
                if self.can_go_back {
                    return Some(MnemonicKeyboardMsg::Previous);
                }
            }
            Some(KeypadMsg::EraseClicked) => {
                self.input.inner_mut().on_backspace_click(ctx);
                self.on_input_change(ctx);
                return None;
            }
            Some(KeypadMsg::EraseLongPressed) => {
                self.input.inner_mut().on_backspace_long_press(ctx);
                self.on_input_change(ctx);
                return None;
            }
            Some(KeypadMsg::ConfirmClicked) => {
                match self.input.inner_mut().on_confirm_click(ctx) {
                    Some(MnemonicInputMsg::Confirmed) => {
                        // Confirmed, bubble up.
                        return Some(MnemonicKeyboardMsg::Confirmed);
                    }
                    Some(_) => {
                        // Either a timeout or a completion.
                        self.on_input_change(ctx);
                        return None;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        if self.input.inner().is_empty() {
            self.prompt.render(target);
        } else {
            self.input.render(target);
        }

        self.keypad.render(target);
    }
}

pub trait MnemonicInput: Component<Msg = MnemonicInputMsg> {
    fn keys() -> [&'static str; MNEMONIC_KEY_COUNT];
    fn can_key_press_lead_to_a_valid_word(&self, key: usize) -> bool;
    fn can_be_confirmed(&self) -> bool;
    fn on_key_click(&mut self, ctx: &mut EventCtx, key: usize);
    fn on_backspace_click(&mut self, ctx: &mut EventCtx);
    fn on_confirm_click(&mut self, ctx: &mut EventCtx) -> Option<MnemonicInputMsg>;
    fn on_backspace_long_press(&mut self, ctx: &mut EventCtx);
    fn is_empty(&self) -> bool;
    fn mnemonic(&self) -> Option<&'static str>;
}

pub enum InputState {
    ValidWord,
    InvalidWord,
    PartialWord,
}

pub enum MnemonicInputMsg {
    Confirmed,
    Completed,
    TimedOut,
}

#[cfg(feature = "ui_debug")]
impl<T> crate::trace::Trace for MnemonicKeyboard<T>
where
    T: MnemonicInput + crate::trace::Trace,
{
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("MnemonicKeyboard");
        t.child("prompt", &self.prompt);
        t.child("input", &self.input);
    }
}
