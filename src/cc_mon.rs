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

use crate::midi::{self, Cc, SpecialControllers};



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


pub const MAX_HISTORY: usize = 200;



#[derive(Clone, Debug)]
pub struct CcValueTime {
    pub channel: u8,
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

#[derive(Debug)]
pub struct ChannelInfo {
    pub instant_last_active: Instant,   // is_active() -> bool is based on this (something like: if duration < 1 sec && no notes pressed)
    pub id: u8,                         // if is_active(), then this is the GUI display index
    pub notes_on: [bool; 128],          // MIDI pitch of notes still pressed on this channel (accounts for sustain pedal and all notes off)
    pub note_count: u8,
}

impl Default for ChannelInfo {
    fn default() -> Self {
        Self { 
            instant_last_active: Instant::now(), 
            id: 0, 
            notes_on: [false; 128], 
            note_count: 0 
        }
    }
}

impl ChannelInfo {
    pub fn is_active(&self, cur_time: Instant) -> bool {
        self.note_count != 0 || (cur_time - self.instant_last_active).as_millis() < 1_000
    }

    pub fn process(&mut self, value: &CcValueTime) {
        if self.instant_last_active < value.instant { self.instant_last_active = value.instant; }
        if SpecialControllers::does_cc_num_imply_all_notes_off(value.cc_num) {
            for on in self.notes_on.iter_mut() {
                *on = false;
            }
            self.note_count = 0;
        }
        match Cc::from_index(value.cc_num) {
            Cc::NoteOn => {
                let note = value.get_value_as_127() as usize;
                if !self.notes_on[note] { 
                    self.note_count += 1;
                    self.notes_on[note] = true;
                }
            }
            Cc::NoteOff => {
                let note = value.get_value_as_127() as usize;
                if self.notes_on[note] { 
                    self.note_count -= 1;
                    self.notes_on[note] = false;
                }
            }
            _ => { }
        }
    }
}


#[derive(Clone, Debug)]
pub struct CcValueTimeHistory {
    pub history: VecDeque<CcValueTime>,     // "back" is most recent; "front" is oldest.  When size == MAX_HISTORY, then discard one from front
}

impl CcValueTimeHistory {
    pub fn is_empty_for_channel(&self, channel: u8) -> bool {
        !self.history.iter().any(|v| v.channel == channel)
    }
}

#[derive(Debug)]
pub struct Histories {
    pub history_per_cc:   Vec<CcValueTimeHistory>,
    pub info_per_channel: [ChannelInfo; 16],
    pub channel_id_count: u8,
}

impl Histories {
    pub fn new() -> Self {
        let mut history_per_cc = Vec::with_capacity(midi::CC_MAX);
        for _ in 0..midi::CC_MAX {
            history_per_cc.push(CcValueTimeHistory { history: VecDeque::with_capacity(MAX_HISTORY) });
        }
    
        Self {
            history_per_cc,
            info_per_channel: [ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), 
                               ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default(), ChannelInfo::default()],
            channel_id_count: 0,
        }
    }
}


/// State for a [`CcMeter`].
#[derive(Debug)]
pub struct State {
    /// MIDI messages get queued here, where we periodically process them into cc_histories.
    pub cc_queue: Arc<ArrayQueue<CcValueTime>>,

    /// The current cc values.  While the values stored here are owned by the GUI (by cc_mon.rs), 
    /// the storage itself is created and (from a Rust lifetime perspective) "owned" by lib.rs's 
    /// MidiMonitor--it is simply re-passed in as a parameter to State::new() every time GUI is 
    /// recreated.  This way, we don't lose our history when the GUI is closed!
    histories: Arc<Mutex<Histories>>,    // Must be created with an entry for each CC
}

impl State {
    pub fn new(cc_queue: Arc<ArrayQueue<CcValueTime>>, histories: Arc<Mutex<Histories>>) -> Self {
        State {
            cc_queue,
            histories,
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
    pub const CC_CHANNEL_NAME_HT:   f32 = Self::CC_VALUE_HEIGHT;
    pub const UI_HEIGHT:            f32 = Self::CC_VERT_SPACER + Self::CC_SLIDER_HEIGHT + Self::CC_VALUE_HEIGHT 
        + Self::CC_NAME_HEIGHT + Self::CC_CANONICAL_HEIGHT + Self::CC_CHANNEL_NAME_HT + Self::CC_VERT_SPACER;

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

    
    // Group by modes:
    // (A) Group by CCs
    // *** (B) Group by channel
    //      - With no config, works pretty well for MPE and poly
    //      - For MPE, would want show/hide channels based on current note(s); 
    //          whereas poly, just straight 0-16 might be preferable.
    //      - Therefore, start with straight 0-16, then implement MPE/smart note option(s)
    //      - With this approach, columnizing CCs data isn't relevant, so can be deferred.
    //
    // Channel modes (when grouped by CCs) 
    // *** (1) MPE - Treat channel 0 as special; single CC for all cc's except note-related, pitch-wheel (x), y, channel/poly_pressure (z);  Note pitch-wheel is for 0 and per-note!
    //      Thus, CCs are either 
    //          (a) ONLY for channel 0 (e.g. most CCs); 
    //          (b) ONLY for per-note (non-channel 0) (e.g. note, vel, noteX, velX, Y, channel/poly_pressure (Z)); or 
    //          (c) BOTH channel 0 and per-note (e.g. pitch-bend)
    //      So "a" is never split; "b" is split based on number of notes; "c" is channel 0 + number of notes.
    // *** (2) Per channel - simply divide each CC independently of others based on what channels have seen those CC messages
    //      For true MPE controllers, this should work out to be similar to "1" (MPE mode), except when note doesn't include, e.g., Z or Y information.
    // (3) All equal - based on global use of channels, split *every* CC into channels, even if only one channel uses that CC;
    //      This was my first GUI implemenation... not very good user experience.

    // Choosing channel ID:
    // (a) Based on channel #, so find active channels and number them from lowest channel to highest.
    //      This was my first GUI implementation... suffers from some randomness in assignment.
    // (b) Based on (initial) note.  So, find active channels and their (lowest(?), if more than one) pitch, and assign from channel with lowest pitch to highest, perhaps with MPE reserving 1st channel for itself.
    //      More predictable for the user.
    // *** (c) Based on (initial) use time.  So, find active channels, and their initial activation time, and assign from channel with oldest activation time to newest activation time, perhaps with MPE reserving 1st channel for itself.
    //      More predictable for the user.  Probably based (primarily) on NoteOn/Off times...

    // Display:
    // (a) Display all without columnizing per channel (based on above considerations) -- old original non-channel aware way.  Still useful...
    //      - Perhaps with per-channel color option?
    //      - Filtering or focus options?
    // (b) Strict columnizing
    //      - Perhaps allow for min channel width, so might overlap a bit horizontally.
    //      (i) Allow growing width for max # of simultaneous channels seen so far (or forced max)
    //      (ii) Fixed width (original implementation)
    //          - Channels can get 1 pixel small (or smaller if width < 16 pixels!)!  If we have min width, we either need to overlap, or allow width to grow.
    // (c) Allocate new meter per channel seen
    //      - Need to optimize for readability, since channels come and go in MPE!

    // Information needed:
    // Per Channel:
    // - What notes are currently on and their channel (including handling all-notes-off messages, etc.)
    // - What CCs are on which channels.  Auto-classify as: (a) MPE control channel only; (b) MPE note channel only (pitch/X, CC74/Y, pressure/Z); (c) all channels (poly mode)
    //      - what channels are represented by CCs (active+history)

    pub fn update_from_state(&self) {
        let mut histories = self.state.histories.lock().unwrap();
        while let Some(item) = self.state.cc_queue.pop() {
            histories.info_per_channel[item.channel as usize].process(&item);
            Self::add_entry(&mut histories.history_per_cc, item);
        }
        let mut id = 0;     // Fist id used is 1, so ids are all in the range 1..=16
        let now = Instant::now();
        for ch_info in histories.info_per_channel.iter_mut() {
            if !ch_info.is_active(now) { 
                ch_info.id = 0;
                continue; 
            }
            id += 1;
            ch_info.id = id;
        }
        histories.channel_id_count = id;
    }

    /// Updates current and historical values for a newly received CC message.
    pub fn add_entry(history_per_cc: &mut [CcValueTimeHistory], value: CcValueTime) {
        let cc_num = value.cc_num as usize;
        if cc_num >= history_per_cc.len() { return; }   // Make sure cc_num isn't an invalid value; ignore if so
        let entry = &mut history_per_cc[cc_num];
        if entry.history.len() == MAX_HISTORY { entry.history.pop_front(); }
        entry.history.push_back(value);
    }

    pub fn count_of_active_ccs(&self) -> usize {
        let histories = self.state.histories.lock().unwrap();
        histories.history_per_cc.iter().filter(|h| !h.history.is_empty()).count()
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
        // FUTURE (multi-channels in one CC lane): let cc_count = self.count_of_active_ccs();

        let bounds = layout.bounds();
        let oy_vert_spacer = (Self::CC_VERT_SPACER / Self::UI_HEIGHT) * bounds.height;
        let width_cc = Self::CC_WIDTH_MAX; // FUTURE (multi-channels in one CC lane): (bounds.width / cc_count as f32).clamp(Self::CC_WIDTH_MIN, Self::CC_WIDTH_MAX);
        let width_dx_max = width_cc; // FUTURE (multi-channels in one CC lane): (width_cc * 0.5).max(1.0);
        let should_show_labels = true || width_cc >= Self::CC_WIDTH_SHOW_LABELS;
        let height_cc_slider =  (Self::CC_SLIDER_HEIGHT    / Self::UI_HEIGHT) * bounds.height;
        let height_value =      (Self::CC_VALUE_HEIGHT     / Self::UI_HEIGHT) * bounds.height;
        let height_name =       (Self::CC_NAME_HEIGHT      / Self::UI_HEIGHT) * bounds.height;
        let height_cannonical = (Self::CC_CANONICAL_HEIGHT / Self::UI_HEIGHT) * bounds.height;
        let y_cc_slider  = bounds.y + oy_vert_spacer;
        let y_value_text = y_cc_slider + height_cc_slider + oy_vert_spacer;
        let y_name_text  = y_value_text + height_value;
        let y_canonical_text = y_name_text + height_name;
        let y_channel_text   = y_canonical_text + height_cannonical;
        let color_text = Color::from_rgb(0.0, 0.0, 1.0);
        // TODO: Font should scale with bounds.height/.width changing, too!  Need to measure?

        let back_colors = [ Color::from_rgb(0.2, 0.1, 0.1), Color::from_rgb(0.1, 0.1, 0.2) ];
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
        let hide_note_info = true;
        let histories = self.state.histories.lock().unwrap();
        let mut i_cc = 0_usize;
        for channel in 0..16 {
            for (cc_num, history_per_cc) in histories.history_per_cc.iter().enumerate() {
                if hide_note_info {
                    match Cc::from_index(cc_num as u8) {
                        Cc::NoteOn      => { continue; }
                        Cc::NoteOff     => { continue; }
                        Cc::VelocityOn  => { continue; }
                        Cc::VelocityOff => { continue; }
                        _ => { }
                    }
                }
                let i = if should_show_only_active { 
                    if history_per_cc.is_empty_for_channel(channel) { continue; }
                    let i = i_cc;
                    i_cc += 1;
                    i
                } else { 
                    cc_num 
                };
                
                let x = bounds.x + (i as f32 * width_cc);
                let back_color = back_colors[(channel & 1) as usize];

                // Fill the cc "slider" background for CC UI (i.e. LED, slider, and text areas)
                renderer.fill_quad(quad_from_bounds(x, y_cc_slider, width_cc, height_cc_slider), back_color);

                // Draw the text
                let last = history_per_cc.history.len() - 1;
                if should_show_labels {
                    let cc_value = history_per_cc.history[last].value;
                    let text_value = format!("{}", CcValueTime::f32_value_to_u8(cc_value));
                    let text_num   = &format!("c{cc_num}");
                    let text_name  = midi::NAMES[cc_num].unwrap_or(text_num);
                    let text_channel = &format!("{}", channel + 1);
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
                    renderer.fill_text(text::Text{
                        content: &text_channel,
                        font: self.font,
                        size: text_size as f32,
                        bounds: Rectangle { x: x + width_cc * 0.5, y: y_channel_text, width: width_cc, height: height_value },
                        color: color_text,
                        horizontal_alignment: alignment::Horizontal::Center,
                        vertical_alignment: alignment::Vertical::Top,
                    });
                }
                
                // Now draw individual historical values, ending with the most recent.
                let index_offset = MAX_HISTORY - (last + 1);
                for (i, cc_info) in history_per_cc.history.iter().enumerate() {
                    if cc_info.channel != channel { continue; }
                    //let id_channel = 0; // FUTURE: histories.info_per_channel[channel as usize].id;
                    let cx_channel = width_dx_max; // FUTURE: If multi channels in one CC column: if histories.channel_id_count == 0 { width_dx_max } else { width_dx_max / histories.channel_id_count as f32 };
                    let ox_channel = 0_f32; // FUTURE: cx_channel * ((id_channel - 1) as f32);
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
                        (lerp(order_frac, cx_channel, 1.0), lerp_color(age_frac.clamp(0.1, 1.0), back_color, Color::from_rgb(0.2, 0.9, 0.2))) 
                    } else { 
                        (lerp(age_frac, (4.0_f32).min(cx_channel), 0.0), 
                        // NOTE: the "jolt" normally wouldn't work on the age_frac == 1.0 side, 
                        // since it is very likely that at least one instant has passed since the CC
                        // was transmitted.  But we "fix" this manually above, by subtracting 20msec,
                        // thus sec < 20msec is treated as zero, allow the jolt to show for up to 20msec.
                        lerp_jolt_color(age_frac, Color::from_rgb(0.6, 0.85, 0.6), Color::from_rgb(0.7, 0.9, 0.7), Color::from_rgb(1.0, 1.0, 1.0), Color::from_rgb(1.0, 1.0, 1.0))
                        )
                    };
                    let y = y_cc_slider + height_cc_slider  * (1.0 - cc_info.value);
                    renderer.fill_quad(quad_from_bounds(x + ox_channel + ox, y, cx_channel - ox - ox, 1.0), color);
                }
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
