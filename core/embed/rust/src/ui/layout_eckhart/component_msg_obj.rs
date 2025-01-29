use crate::{
    error::Error,
    micropython::obj::Obj,
    ui::{
        component::{
            base,
            text::paragraphs::{ParagraphSource, Paragraphs},
            Component, Timeout,
        },
        layout::{
            obj::ComponentMsgObj,
            result::{CANCELLED, CONFIRMED},
        },
    },
};

use super::component::{
    FormattedScreen, FormattedScreenMsg, MnemonicInput, MnemonicKeyboard, MnemonicKeyboardMsg,
    PinKeyboard, PinKeyboardMsg,
};

impl ComponentMsgObj for PinKeyboard<'_> {
    fn msg_try_into_obj(&self, msg: Self::Msg) -> Result<Obj, Error> {
        match msg {
            PinKeyboardMsg::Confirmed => self.pin().try_into(),
            PinKeyboardMsg::Cancelled => Ok(CANCELLED.as_obj()),
        }
    }
}

impl<T> ComponentMsgObj for MnemonicKeyboard<T>
where
    T: MnemonicInput,
{
    fn msg_try_into_obj(&self, msg: Self::Msg) -> Result<Obj, Error> {
        match msg {
            MnemonicKeyboardMsg::Confirmed => {
                if let Some(word) = self.mnemonic() {
                    word.try_into()
                } else {
                    fatal_error!("Invalid mnemonic")
                }
            }
            MnemonicKeyboardMsg::Previous => "".try_into(),
        }
    }
}

// Clippy/compiler complains about conflicting implementations
// TODO move the common impls to a common module
#[cfg(not(feature = "clippy"))]
impl<'a, T> ComponentMsgObj for Paragraphs<T>
where
    T: ParagraphSource<'a>,
{
    fn msg_try_into_obj(&self, _msg: Self::Msg) -> Result<Obj, Error> {
        unreachable!()
    }
}

// Clippy/compiler complains about conflicting implementations
#[cfg(not(feature = "clippy"))]
impl<T> ComponentMsgObj for (Timeout, T)
where
    T: Component<Msg = ()>,
{
    fn msg_try_into_obj(&self, _msg: Self::Msg) -> Result<Obj, Error> {
        Ok(CANCELLED.as_obj())
    }
}

impl ComponentMsgObj for FormattedScreen {
    fn msg_try_into_obj(&self, msg: Self::Msg) -> Result<Obj, Error> {
        match msg {
            FormattedScreenMsg::Cancelled => Ok(CANCELLED.as_obj()),
            FormattedScreenMsg::Confirmed => Ok(CONFIRMED.as_obj()),
        }
    }
}
