//! A comprensive midi monitor

use crossbeam::queue::ArrayQueue;
use nih_plug_iced::renderer::Quad;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nih_plug_iced::backend::Renderer;
use nih_plug_iced::renderer::Renderer as GraphicsRenderer;
use nih_plug_iced::text::{self, Renderer as TextRenderer};
use nih_plug_iced::{
    alignment, layout, renderer, Color, Element, Font, Layout, Length, Point, Rectangle, Size, Widget
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

pub fn create_cc_histories() -> Arc<Mutex<Vec<CcValueTimeHistory>>> {
    let mut histories = Vec::with_capacity(midi::CC_MAX);
    for _ in 0..midi::CC_MAX {
        histories.push(CcValueTimeHistory { history: VecDeque::with_capacity(MAX_HISTORY) });
    }

    Arc::new(Mutex::new(histories))
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


/// State for a [`CcMeter`].
#[derive(Debug)]
pub struct State {
    pub cc_queue: Arc<ArrayQueue<CcValueTime>>,

    /// The current cc values.  While the values stored here are owned by the GUI (by cc_mon.rs), 
    /// the storage itself is created and (from a Rust lifetime perspective) "owned" by lib.rs's 
    /// MidiMonitor--it is simply re-passed in as a parameter to State::new() every time GUI is 
    /// recreated.  This way, we don't lose our history when the GUI is closed!
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

    pub const CC_VERT_SPACER:       f32 = 2.0;
    pub const CC_WIDTH_MIN:         f32 = 3.0;
    pub const CC_WIDTH_SHOW_LABELS: f32 = 11.0;
    pub const CC_WIDTH_MAX:         f32 = 35.0;
    pub const CC_SLIDER_HEIGHT:     f32 = 128.0;
    pub const CC_VALUE_HEIGHT:      f32 = 15.0;
    pub const CC_NAME_HEIGHT:       f32 = Self::CC_VALUE_HEIGHT;
    pub const CC_CANONICAL_HEIGHT:  f32 = Self::CC_VALUE_HEIGHT;
    pub const UI_HEIGHT:            f32 = Self::CC_VERT_SPACER + Self::CC_SLIDER_HEIGHT + Self::CC_VALUE_HEIGHT + Self::CC_NAME_HEIGHT + Self::CC_CANONICAL_HEIGHT + Self::CC_VERT_SPACER;

    /// Creates a new [`CcMeter`] which displays the current value of CCs as well as a visual 
    /// history of values.
    pub fn new(state: &'a mut State) -> Self {
        Self {
            state,

            width:  length_of_f32(Self::CC_WIDTH_MIN * midi::CC_MAX as f32),
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

    pub fn count_of_active_ccs(&self) -> usize {
        let cc_histories = self.state.cc_histories.lock().unwrap();
        cc_histories.iter().filter(|h| !h.history.is_empty()).count()
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

    
fn lerp_jolt(frac: f32, v0_jolt: f32, v0: f32, v1: f32, v1_jolt: f32) -> f32 {
    if frac <= 0.0 { return v0_jolt; }
    if frac >= 1.0 { return v1_jolt; }
    v0 + (v1 - v0) * frac
}
fn lerp_jolt_color(frac: f32, c0_jolt: Color, c0: Color, c1:Color, c1_jolt: Color) -> Color {
    Color::from_rgb(
        lerp_jolt(frac, c0_jolt.r, c0.r, c1.r, c1_jolt.r), 
        lerp_jolt(frac, c0_jolt.g, c0.g, c1.g, c1_jolt.g), 
        lerp_jolt(frac, c0_jolt.b, c0.b, c1.b, c1_jolt.b), 
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
        let cc_count = self.count_of_active_ccs();

        let bounds = layout.bounds();
        let oy_vert_spacer = (Self::CC_VERT_SPACER / Self::UI_HEIGHT) * bounds.height;
        let width_cc = (bounds.width / cc_count as f32).clamp(Self::CC_WIDTH_MIN, Self::CC_WIDTH_MAX);
        let width_dx_max = (width_cc * 0.5).max(1.0);
        let should_show_labels = true || width_cc >= Self::CC_WIDTH_SHOW_LABELS;
        let height_cc_slider = (Self::CC_SLIDER_HEIGHT / Self::UI_HEIGHT) * bounds.height;
        let height_value =     (Self::CC_VALUE_HEIGHT  / Self::UI_HEIGHT) * bounds.height;
        let height_name =      (Self::CC_NAME_HEIGHT   / Self::UI_HEIGHT) * bounds.height;
        let y_cc_slider  = bounds.y + oy_vert_spacer;
        let y_value_text = y_cc_slider + height_cc_slider + oy_vert_spacer;
        let y_name_text  = y_value_text + height_value;
        let y_canonical_text = y_name_text + height_name;
        let color_text = Color::from_rgb(0.0, 0.0, 1.0);
        // TODO: Font should scale with bounds.height/.width changing, too!  Need to measure?

        let back_color = Color::from_rgb(0.2, 0.1, 0.1);
        let now = Instant::now();

        // Draw a black box around where all of the sliders go
        // FUTURE: Size to exactly the sliders shown, rather than full width available
        {
            let mut q = quad_from_bounds(bounds.x, bounds.y, bounds.width, oy_vert_spacer + height_cc_slider + oy_vert_spacer);
            q.border_color = Color::BLACK;
            q.border_width = 2.0;
            renderer.fill_quad(q, Color::TRANSPARENT);
        }

        // Show the current value as a white line
        let should_show_only_active = true;
        let cc_histories = self.state.cc_histories.lock().unwrap();
        let mut i_cc = 0_usize;
        for (cc_num, cc_history) in cc_histories.iter().enumerate() {
            let i = if should_show_only_active { 
                if cc_history.history.is_empty() { continue; }
                let i = i_cc;
                i_cc += 1;
                i
            } else { 
                cc_num 
            };
            
            let x = bounds.x + (i as f32 * width_cc);

            // Fill the cc "slider" background for CC UI (i.e. LED, slider, and text areas)
            renderer.fill_quad(quad_from_bounds(x, y_cc_slider, width_cc, height_cc_slider), back_color);

            // Draw the text
            let last = cc_history.history.len() - 1;
            if should_show_labels {
                let cc_value = cc_history.history[last].value;
                let text_value = format!("{}", CcValueTime::f32_value_to_u8(cc_value));
                let text_num   = &format!("c{cc_num}");
                let text_name  = midi::NAMES[cc_num].unwrap_or(text_num);
                let text_size = self
                    .text_size
                    .unwrap_or_else(|| (renderer.default_size() as f32 * 0.9).round() as u16);
                renderer.fill_text(text::Text{
                    content: &text_value,
                    font: self.font,
                    size: text_size as f32,
                    bounds: Rectangle { x: x + width_cc * 0.5, y: y_value_text, width: width_cc, height: height_value },
                    color: color_text,
                    horizontal_alignment: alignment::Horizontal::Center,
                    vertical_alignment: alignment::Vertical::Top,
                });
                renderer.fill_text(text::Text{
                    content: &text_name,
                    font: self.font,
                    size: text_size as f32,
                    bounds: Rectangle { x: x + width_cc * 0.5, y: y_name_text, width: width_cc, height: height_value },
                    color: color_text,
                    horizontal_alignment: alignment::Horizontal::Center,
                    vertical_alignment: alignment::Vertical::Top,
                });
                if cc_num <= 127 {
                    renderer.fill_text(text::Text{
                        content: &text_num,
                        font: self.font,
                        size: text_size as f32,
                        bounds: Rectangle { x: x + width_cc * 0.5, y: y_canonical_text, width: width_cc, height: height_value },
                        color: color_text,
                        horizontal_alignment: alignment::Horizontal::Center,
                        vertical_alignment: alignment::Vertical::Top,
                    });
                }
            }
            
            // Now draw individual historical values, ending with the most recent.
            let index_offset = MAX_HISTORY - (last + 1);
            for (i, cc_info) in cc_history.history.iter().enumerate() {
                let sec = now.duration_since(cc_info.instant).as_secs_f32(); 
                const SEC_FADEOUT: f32 = 4.0;
                // Remove 20ms to account for transit time from lib.rs thread to here.  This
                // becomes important when we use lerp_jolt_color() below, as if sec can never
                // be zero, then no "jolt" can happen on that side of things.  By removing
                // 20msec (and clamping the result), we might get as much as 20msec of jolt
                // on the seconds == 0.0 side of the color fading, which would be nice.
                let sec = (sec - 0.020).clamp(0.0, SEC_FADEOUT);
                let age_frac = 1.0 - (sec / SEC_FADEOUT);
                //let age_frac = age_frac.clamp(0.1, 1.0);
                let order_frac = (i + index_offset) as f32 / MAX_HISTORY as f32;
                // NOTE: We use lerp instead of actual alpha values to avoid artefacts of old 
                // values stacking on top of each other.  With using alpha, these would appear 
                // significantly brighter, even after "fading", but using lerp(), they don't
                // get brighter when they stack, since the most recent one just "wins".
                let (ox, color) = if i != last { 
                    (lerp(order_frac, width_dx_max, 1.0), lerp_color(age_frac.clamp(0.1, 1.0), back_color, Color::from_rgb(0.2, 0.9, 0.2))) 
                } else { 
                    (lerp(age_frac, (4.0_f32).min(width_dx_max), 0.0), 
                     // NOTE: the "jolt" normally wouldn't work on the age_frac == 1.0 side, 
                     // since it is unlikely that at least an instant hasn't passed since the CC
                     // was transmitted.  But we "fix" this manually above, by subtracting 20msec,
                     // thus sec < 20msec is treated as zero.
                     lerp_jolt_color(age_frac, Color::from_rgb(0.6, 0.85, 0.6), Color::from_rgb(0.7, 0.9, 0.7), Color::from_rgb(1.0, 1.0, 1.0), Color::from_rgb(1.0, 1.0, 1.0))
                    )
                };
                let y = y_cc_slider + height_cc_slider  * (1.0 - cc_info.value);
                renderer.fill_quad(quad_from_bounds(x + ox, y, width_cc - ox - ox, 1.0), color);
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
