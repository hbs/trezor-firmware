use crate::{
    strutil::TString,
    time::{Duration, Stopwatch},
    ui::{
        component::{text::TextStyle, Component, Event, EventCtx, Label},
        display::{Color, Icon},
        geometry::{Alignment, Alignment2D, Insets, Offset, Rect},
        lerp::Lerp,
        shape::{self, Renderer},
        util::animation_disabled,
    },
};

use super::{
    button::{Button, ButtonContent, ButtonMsg, ButtonStyleSheet},
    theme,
};

const ANIMATION_TIME_MS: u32 = 1000;

#[derive(Default, Clone)]
struct AttachAnimation {
    pub timer: Stopwatch,
}

impl AttachAnimation {
    pub fn is_active(&self) -> bool {
        if animation_disabled() {
            return false;
        }

        self.timer
            .is_running_within(Duration::from_millis(ANIMATION_TIME_MS))
    }

    pub fn eval(&self) -> f32 {
        if animation_disabled() {
            return ANIMATION_TIME_MS as f32 / 1000.0;
        }
        self.timer.elapsed().to_millis() as f32 / 1000.0
    }

    pub fn get_title_offset(&self, t: f32) -> i16 {
        let fnc = pareen::constant(0.0).seq_ease_in_out(
            0.8,
            easer::functions::Cubic,
            0.2,
            pareen::constant(1.0),
        );
        i16::lerp(0, 25, fnc.eval(t))
    }

    pub fn start(&mut self) {
        self.timer.start();
    }

    pub fn reset(&mut self) {
        self.timer = Stopwatch::new_stopped();
    }
}

const BUTTON_EXPAND_BORDER: i16 = 32;

/// Component for the header of a screen. Eckhart UI shows the title (can be two
/// lines), optional icon button on the left, and optional icon button (typically for menu)
/// on the right.
pub struct Header {
    area: Rect,
    title: Label<'static>,
    title_style: TextStyle,
    /// button in the top-right corner
    right_button: Option<Button>,
    /// button in the top-left corner, can be just an icon
    left_button: Option<Button>,
    right_button_msg: HeaderMsg,
    left_button_msg: HeaderMsg,
    anim: Option<AttachAnimation>,
}

#[derive(Copy, Clone)]
pub enum HeaderMsg {
    Cancelled,
    Info,
}

impl Header {
    pub const HEADER_HEIGHT: i16 = 96; // [px]
    pub const HEADER_BUTTON_WIDTH: i16 = 80; // [px]

    pub const fn new(title: TString<'static>) -> Self {
        Self {
            area: Rect::zero(),
            title: Label::new(title, Alignment::Start, theme::label_title_main())
                .vertically_centered(),
            title_style: theme::label_title_main(),
            right_button: None,
            left_button: None,
            right_button_msg: HeaderMsg::Cancelled,
            left_button_msg: HeaderMsg::Cancelled,
            anim: None,
        }
    }

    #[inline(never)]
    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.title_style = style;
        self.title = self.title.styled(style);
        self
    }

    #[inline(never)]
    pub fn with_right_button(mut self, button: Button, msg: HeaderMsg) -> Self {
        debug_assert!(matches!(button.content(), ButtonContent::Icon(_)));
        let touch_area = Insets::uniform(BUTTON_EXPAND_BORDER);
        self.right_button = Some(button.with_expanded_touch_area(touch_area));
        self.right_button_msg = msg;
        self
    }

    #[inline(never)]
    pub fn with_left_button(mut self, button: Button, msg: HeaderMsg) -> Self {
        debug_assert!(matches!(button.content(), ButtonContent::Icon(_)));
        let touch_area = Insets::uniform(BUTTON_EXPAND_BORDER);
        self.left_button = Some(button.with_expanded_touch_area(touch_area));
        self.left_button_msg = msg;
        self
    }

    #[inline(never)]
    pub fn with_icon(mut self, icon: Icon, color: Color) -> Self {
        let left_button = Button::with_icon(icon)
            .initially_enabled(false)
            .styled(theme::button_default());
        self.with_left_button(left_button, HeaderMsg::Cancelled)
    }

    #[inline(never)]
    pub fn update_title(&mut self, ctx: &mut EventCtx, title: TString<'static>) {
        self.title.set_text(title);
        ctx.request_paint();
    }
}

impl Component for Header {
    type Msg = HeaderMsg;

    fn place(&mut self, bounds: Rect) -> Rect {
        debug_assert_eq!(bounds.height(), Self::HEADER_HEIGHT);

        let rest = if let Some(b) = &mut self.right_button {
            let (rest, button_area) = bounds.split_right(Self::HEADER_BUTTON_WIDTH);
            b.place(button_area);
            rest
        } else {
            bounds
        };

        let title_area = if let Some(b) = &mut self.left_button {
            let margin_left = 24; // [px]
            let margin_right = 16; // [px]
            let left_button_width = match b.content() {
                ButtonContent::Icon(icon) => icon.toif.width(),
                _ => 0,
            };
            let (rest, title_area) =
                rest.split_left(margin_left + left_button_width + margin_right);
            b.place(rest.split_center(left_button_width).1);
            title_area
        } else {
            rest
        };

        self.title.place(title_area);
        self.area = bounds;
        bounds
    }

    fn event(&mut self, ctx: &mut EventCtx, event: Event) -> Option<Self::Msg> {
        self.title.event(ctx, event);

        if let Some(anim) = &mut self.anim {
            if let Event::Attach(_) = event {
                anim.start();
                ctx.request_paint();
                ctx.request_anim_frame();
            }
            if let Event::Timer(EventCtx::ANIM_FRAME_TIMER) = event {
                if anim.is_active() {
                    ctx.request_anim_frame();
                    ctx.request_paint();
                }
            }
        }

        if let Some(ButtonMsg::Clicked) = self.left_button.event(ctx, event) {
            return Some(self.left_button_msg.clone());
        };
        if let Some(ButtonMsg::Clicked) = self.right_button.event(ctx, event) {
            return Some(self.right_button_msg.clone());
        };

        None
    }

    fn render<'s>(&'s self, target: &mut impl Renderer<'s>) {
        let offset = if let Some(anim) = &self.anim {
            Offset::x(anim.get_title_offset(anim.eval()))
        } else {
            Offset::zero()
        };

        // TODO: correct animation
        target.with_origin(offset, &|target| {
            self.right_button.render(target);
            self.left_button.render(target);
            self.title.render(target);
        });
    }
}

#[cfg(feature = "ui_debug")]
impl crate::trace::Trace for Header {
    fn trace(&self, t: &mut dyn crate::trace::Tracer) {
        t.component("Header");
        t.child("title", &self.title);
        if let Some(button) = &self.right_button {
            t.child("button", button);
        }
    }
}
