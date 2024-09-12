//! A comprensive midi monitor

use crossbeam::queue::ArrayQueue;
use nih_plug_iced::renderer::Quad;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nih_plug_iced::backend::Renderer;
use nih_plug_iced::renderer::Renderer as GraphicsRenderer;
use nih_plug_iced::text::Renderer as TextRenderer;
use nih_plug_iced::{
    layout, renderer, Color, Element, Font, Layout, Length, Point, Rectangle, Size, Widget
};

use crate::midi;



/// A simple horizontal peak meter.
pub struct CcMeter<'a, Message> {
    state: &'a mut State,


    height: Length,
    width: Length,
    text_size: Option<u16>,
    font: Font,

    /// We don't emit any messages, but iced requires us to define some message type anyways.
    _phantom: PhantomData<Message>,
}


pub const MAX_HISTORY: usize = 100;

#[derive(Clone, Debug)]
pub struct CcValueTimeHistory {
    pub history: VecDeque<CcValueTime>,     // "back" is most recent; "front" is oldest.  When size == MAX_HISTORY, then discard one from front
}

#[derive(Copy, Clone, Debug)]
pub struct CcValueTime {
    pub cc_num:  u8,
    /// `value` ranges from 0.0..=1.0 (i.e. 0.0 to 1.0, inclusive).  Traditional 0-127 MIDI values
    /// are encoded as `byte as f32 / 127.0`.  14-bit controllers are encoded analogously encoded
    /// into the 0.0..=1.0 range.
    pub value:   f32,   // 0.0..=1.0; Most CCs are a single 7-bit byte / 127.0; PitchBend is two 7-bit bytes (14-bits total), still encoded 0.0..=1.0
    pub instant: Instant,
}

impl CcValueTime {
    pub fn f32_value_to_u8(value: f32) -> u8 { (value * 127.0).round() as u8 }
    pub fn u8_value_to_f32(value: u8) -> f32 { (value as f32) / 127.0 }

    pub fn get_value_as_u8(&self) -> u8 {
        (self.value * 127.0).round() as u8
    }

    pub fn get_value_as_127(&self) -> f32 {
        self.value * 127.0
    }
}

pub fn create_cc_histories() -> Arc<Mutex<Vec<CcValueTimeHistory>>> {
    let mut histories = Vec::with_capacity(midi::CC_MAX);
    for _ in 0..midi::CC_MAX {
        histories.push(CcValueTimeHistory { history: VecDeque::with_capacity(MAX_HISTORY) });
    }

    Arc::new(Mutex::new(histories))
}

/// State for a [`CcMeter`].
#[derive(Debug)]
pub struct State {
    //pub receiver: Receiver<CcValueTime>,
    pub cc_queue: Arc<ArrayQueue<CcValueTime>>,

    /// The current cc values.  This value is created and "owned" my lib.rs's MidiMonitor, and 
    /// simply re-passed in as a parameter to State::new() every time GUI is recreated.  This
    /// way, we don't lose our history when the GUI is closed!
    cc_histories: Arc<Mutex<Vec<CcValueTimeHistory>>>,    // Must be created with an entry for each CC
}

impl State {
    pub fn new(cc_queue: Arc<ArrayQueue<CcValueTime>>, cc_histories: Arc<Mutex<Vec<CcValueTimeHistory>>>) -> Self {
        State {
            cc_queue,
            cc_histories,
        }
    }
}

fn length_of_f32(f: f32) -> Length {
    Length::Units(f.round() as u16)
}

impl<'a, Message> CcMeter<'a, Message> {
    pub const CC_MAX: usize = midi::CC_MAX;

    pub const CC_WIDTH:         f32 = 5.0;
    pub const LED_HEIGHT:       f32 = Self::CC_WIDTH;
    pub const CC_SLIDER_HEIGHT: f32 = 128.0;
    pub const CC_NAME_HEIGHT:   f32 = Self::CC_WIDTH;
    pub const CC_VALUE_HEIGHT:  f32 = Self::CC_NAME_HEIGHT;
    pub const UI_HEIGHT:        f32 = Self::LED_HEIGHT + Self::CC_SLIDER_HEIGHT + Self::CC_NAME_HEIGHT + Self::CC_VALUE_HEIGHT;

    /// Creates a new [`CcMeter`] which displays the current value of CCs as well as a visual 
    /// history of values.
    pub fn new(state: &'a mut State) -> Self {
        Self {
            state,

            width:  length_of_f32(Self::CC_WIDTH * midi::CC_MAX as f32),
            height: length_of_f32(Self::UI_HEIGHT),
            text_size: None,
            font: <Renderer as TextRenderer>::Font::default(),

            _phantom: PhantomData,
        }
    }

    pub fn update_from_state(&self) {
        let mut cc_histories = self.state.cc_histories.lock().unwrap();
        while let Some(item) = self.state.cc_queue.pop() {
            Self::add_entry(&mut cc_histories, &item);
        }
    }

    /// Updates current and historical values for a newly received CC message.
    pub fn add_entry(cc_histories: &mut [CcValueTimeHistory], value: &CcValueTime) {
        let cc_num = value.cc_num as usize;
        if cc_num >= cc_histories.len() { return; }
        let entry = &mut cc_histories[cc_num];
        if entry.history.len() == MAX_HISTORY { entry.history.pop_front(); }
        entry.history.push_back(*value);
    }

    /// Sets the width of the [`CcMeter`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the [`CcMeter`].
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the text size of the [`CcMeter`]'s ticks bar.
    pub fn text_size(mut self, size: u16) -> Self {
        self.text_size = Some(size);
        self
    }

    /// Sets the font of the [`CcMeter`]'s ticks bar.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }
}
    
fn lerp(frac: f32, v0: f32, v1: f32) -> f32 {
    v0 + (v1 - v0) * frac
}
fn lerp_color(frac: f32, c0: Color, c1: Color) -> Color {
    Color::from_rgb(
        lerp(frac, c0.r, c1.r), 
        lerp(frac, c0.g, c1.g), 
        lerp(frac, c0.b, c1.b), 
    )
}

fn quad_from_bounds(x: f32, y: f32, width: f32, height: f32) -> Quad {
    Quad { 
        border_color:  Color::TRANSPARENT,
        border_radius: 0.0,
        border_width:  0.0,
        bounds: Rectangle { x, y, width, height },
    }
}

impl<'a, Message> Widget<Message, Renderer> for CcMeter<'a, Message>
where
    Message: Clone,
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(Size::ZERO);

        layout::Node::new(size)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        self.update_from_state();

        let bounds = layout.bounds();

        let cc_width = Self::CC_WIDTH;
        let cc_slider_height = (Self::CC_SLIDER_HEIGHT / Self::UI_HEIGHT) * bounds.height;
        let cc_slider_oy     = bounds.y + (Self::LED_HEIGHT / Self::UI_HEIGHT) * bounds.height;

        //let text_size = self
        //    .text_size
        //    .unwrap_or_else(|| (renderer.default_size() as f32 * 1.0).round() as u16);
        //
        //let s = format!("V: x{} y{} w{} h{}", viewport.x, viewport.y, viewport.width, viewport.height);
        //let s = format!("{s}\nL: x{} y{} w{} h{}", bounds.x, bounds.y, bounds.width, bounds.height);
        //renderer.fill_text(text::Text {
        //    content: &s,
        //    font: self.font,
        //    size: text_size as f32,
        //    bounds: Rectangle { x: bounds.x, y: bounds.y + bounds.height, width: bounds.width, height: bounds.height },
        //    color: Color::from_rgb(0.0, 0.0, 0.5),
        //    horizontal_alignment: alignment::Horizontal::Left,
        //    vertical_alignment: alignment::Vertical::Bottom,
        //});
        let back_color = Color::from_rgb(0.2, 0.1, 0.1);
        let now = Instant::now();

        // Show the current value as a white line
        let cc_histories = self.state.cc_histories.lock().unwrap();
        for (cc_num, cc_history) in cc_histories.iter().enumerate() {
            let x = bounds.x + (cc_num as f32 * Self::CC_WIDTH);
            // Fill the entire background for CC UI (i.e. LED, slider, and text areas)
            renderer.fill_quad(quad_from_bounds(x, bounds.y, cc_width, bounds.height), back_color);

            // Draw slider info
            // TODO: Allow some form of external theming, perhaps including two (or more) fade out times and colors.
            // TODO: This could be via a theme.txt file that gets checked periodically and hot-loaded... this would let me quickly try different combinations
            // TODO: Params: backcolor, current color, history0 color, fade time, fade pow, history1 color, fade time, fade pow, history2 color, min alpha, maxhistory event count
            // TODO: More: min CC_WIDTH, min line height, line height
            // TODO: Gui Settings: Show only active CCs (seconds); Show only used CCs (sticky); Show all CCs; Auto CC width (max width), horizontal scroll? (Would be nice for multiple CC histories?)
            // TOOD: Features: Mute, map (both to different CC, and scale values), show before/after (toggle), show history, right-click menus (hide this CC, unhide CC (choose from list), show history)
            // TODO: Where to show CC history: Inline or in **seperate graph below**?
            let last = cc_history.history.len() - 1;
            for (i, cc_info) in cc_history.history.iter().enumerate() {
                let sec = now.duration_since(cc_info.instant).as_secs_f32(); 
                const SEC_FADEOUT: f32 = 4.0;
                let sec = sec.clamp(0.0, SEC_FADEOUT);
                let frac = 1.0 - (sec / SEC_FADEOUT);
                let alpha = frac.clamp(0.1, 1.0);
                // NOTE: We use lerp instead of actual alpha values to avoid artefacts of old values stacking on top of each other.
                // With using alpha, these would appear significantly brighter, even after "fading", but using lerp(), they don't
                // stack, and the most recent one "wins".
                let color = if i != last { 
                    lerp_color(alpha, back_color, Color::from_rgb(0.2, 0.9, 0.2)) 
                } else { 
                    lerp_color(alpha, Color::from_rgb(0.85, 0.9, 0.85), Color::from_rgb(1.0, 1.0, 1.0)) 
                };
                //log_this(&format!("cc:{}, a:{alpha:.3}, i_last:{}, c:{color:?}, sec:{sec:.3}, now:{now:?}, cc_inst:{:?}", cc_info.cc, i != last, cc_info.instant));
                let y = cc_slider_oy + cc_slider_height  * (1.0 - cc_info.value);
                renderer.fill_quad(quad_from_bounds(x, y, cc_width, 1.0), color);
            }
        }
    }

    fn on_event(
        &mut self,
        _event: nih_plug_iced::Event,
        _layout: Layout<'_>,
        _cursor_position: Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn nih_plug_iced::Clipboard,
        _shell: &mut nih_plug_iced::Shell<'_, Message>,
    ) -> nih_plug_iced::event::Status {
        self.update_from_state();
        nih_plug_iced::event::Status::Ignored
    }
    
    fn mouse_interaction(
        &self,
        _layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> nih_plug_iced::mouse::Interaction {
        nih_plug_iced::mouse::Interaction::Idle
    }
}

impl<'a, Message> From<CcMeter<'a, Message>> for Element<'a, Message> where Message: 'a + Clone {
    fn from(widget: CcMeter<'a, Message>) -> Self {
        Element::new(widget)
    }
}
