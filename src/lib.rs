use cc_mon::{CcValueTime, CcValueTimeHistory};
use crossbeam::queue::ArrayQueue;
use midi::Cc;
use nih_plug::prelude::*;
use nih_plug_iced::IcedState;
use std::{sync::{Arc, Mutex}, time::Instant};

mod editor;
mod midi;
mod cc_mon;


/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
struct MidiMonitor {
    params: Arc<MidiMonitorParams>,
    cc_queue: Arc<ArrayQueue<CcValueTime>>,
    cc_histories: Arc<Mutex<Vec<CcValueTimeHistory>>>,    // Must be created with an entry for each CC.  Don't access directly from MidiMonitor thread, only from GUI thread!
}

#[derive(Params)]
struct MidiMonitorParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,

    // #[id = "gain"]
    // pub gain: FloatParam,
}

impl Default for MidiMonitor {
    fn default() -> Self {
        Self {
            params:   Arc::new(MidiMonitorParams::default()),
            cc_queue: Arc::new(ArrayQueue::<CcValueTime>::new(1000)),
            cc_histories: cc_mon::create_cc_histories(),
        }
    }
}

impl Default for MidiMonitorParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            // // See the main gain example for more details
            // gain: FloatParam::new(
            //     "Gain",
            //     util::db_to_gain(0.0),
            //     FloatRange::Skewed {
            //         min: util::db_to_gain(-30.0),
            //         max: util::db_to_gain(30.0),
            //         factor: FloatRange::gain_skew_factor(-30.0, 30.0),
            //     },
            // )
            // .with_smoother(SmoothingStyle::Logarithmic(50.0))
            // .with_unit(" dB")
            // .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            // .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl MidiMonitor {
    fn send_u8( &mut self, cc_num: u8, value: u8 ) { self.send_f32(cc_num, CcValueTime::u8_value_to_f32(value)) }
    fn send_f32(&mut self, cc_num: u8, value: f32) {
        let cc = CcValueTime { cc_num, value, instant: Instant::now() };
        // When our GUI is not visible, the queue will fill up, since there's nothing reading from
        // the queue. This is perfectly okay, and force_push() will simply overwrite the oldest 
        // values in the queue.  This could cause some missed events, most notably, Note_Off 
        // events, so we may want *some* kind of state tracking on this side in addition to just 
        // forwarding the events (perhaps the *current* state/values?)
        self.cc_queue.force_push(cc);
    }
}

impl Plugin for MidiMonitor {
    const NAME:   &'static str = "FM MIDI Monitor (iced)";
    const VENDOR: &'static str = "Fraley Music";
    const URL:    &'static str = "https://www.FraleyMusic.com/";
    const EMAIL:  &'static str = "chris@fraleymusic.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    const MIDI_INPUT:  MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        //let receiver = self.receiver.take().unwrap();
        editor::create(
            self.params.clone(),
            self.cc_queue.clone(),
            self.cc_histories.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // This example converts one of the two messages into the other
        while let Some(event) = context.next_event() {
            //self.sender.send(CcValueTime { cc: 0, value: 42, instant: Instant::now() }).unwrap();
            match event {
                NoteEvent::MidiCC { timing:_, channel:_, cc, value } => {
                    //self.sender.send(CcValueTime { cc: 1, value: 80, instant: Instant::now() }).unwrap();
                    self.send_f32(cc, value,);
                }
                NoteEvent::NoteOn { timing:_, voice_id:_, channel:_, note, velocity } => {
                    //self.sender.send(CcValueTime { cc: 2, value: 110, instant: Instant::now() }).unwrap();
                    self.send_u8( Cc::NoteOn    .to_index(), note);
                    self.send_f32(Cc::VelocityOn.to_index(), velocity);
                }
                NoteEvent::NoteOff { timing:_, voice_id:_, channel:_, note, velocity } => {
                    //self.sender.send(CcValueTime { cc: 11, value: 10, instant: Instant::now() }).unwrap();
                    self.send_u8( Cc::NoteOff    .to_index(), note);
                    self.send_f32(Cc::VelocityOff.to_index(), velocity);
                }
                NoteEvent::MidiPitchBend { timing:_, channel:_, value } => {
                    //self.sender.send(CcValueTime { cc: 10, value: 10, instant: Instant::now() }).unwrap();
                    self.send_f32(Cc::PitchBend.to_index(), value);
                }
                _ => {},
            }
            context.send_event(event);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MidiMonitor {
    const CLAP_ID: &'static str = "com.fraleymusic.midi-monitor";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A MIDI monitor");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::NoteEffect, 
        ClapFeature::Utility
    ];
}

impl Vst3Plugin for MidiMonitor {
    const VST3_CLASS_ID: [u8; 16] = *b"FmMidiMon       ";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Tools,
    ];
}

nih_export_clap!(MidiMonitor);
nih_export_vst3!(MidiMonitor);







pub fn log_this(s: &str) {
    use std::fs::OpenOptions;
    use std::io::prelude::*;
    
    let mut file = OpenOptions::new()
        .append(true)
        .open("/Users/chris/Downloads/nih_plug_log.txt")
        .unwrap();

    #[allow(clippy::redundant_pattern_matching)]   
    if let Err(_) = writeln!(file, "{s}") { }
}
