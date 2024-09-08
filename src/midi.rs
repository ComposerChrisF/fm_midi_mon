//use atomic_float::AtomicF32;
//use nih_plug::prelude::*;
//use nih_plug_iced::IcedState;
//use std::sync::Arc;


//struct MidiNoteId {
//    pub id: u16,    // 7 LSB = note, next 4 bits = channel, 5 MSB = interface.  Thus, max of 128 notes, 16 channels, 32 interfaces
//}
//
//impl MidiNoteId {
//    pub fn new(note: u8, channel: u8, interface: u8) -> Self {
//        MidiNoteId {
//            id: (note & 0x7F) as u16 | (((channel & 0xF) as u16) << 7) | (((interface & 0x1F) as u16) << 11),
//        }
//    }
//
//    pub fn get_note(&self) -> u8 {
//        self.id as u8 & 0x7F
//    }
//
//    pub fn get_channel(&self) -> u8 {
//        (self.id >> 7) as u8 & 0xF
//    }
//
//    pub fn get_interface(&self) -> u8 {
//        (self.id >> 11) as u8 & 0x1F
//    }
//}


#[derive(Copy, Clone, Debug)]
pub enum Cc {
    Controller(u8),
    NoteOn,
    VelocityOn,
    ChannelAftertouch,
    PitchWheel ,
    NoteAftertouch,
    NoteOff,
    VelocityOff,
}

pub const CC_MAX: usize = 135;      // Number of MIDI CC's, including our internal additions

impl Cc {
    pub fn to_index(self) -> u8 {
        match self {
            Cc::Controller(c)     => c & 0x7f,
            Cc::NoteOn            => 128,
            Cc::VelocityOn        => 129,
            Cc::ChannelAftertouch => 130,
            Cc::PitchWheel        => 131,
            Cc::NoteAftertouch    => 132,
            Cc::NoteOff           => 133,
            Cc::VelocityOff       => 134,
        }
    }

    pub fn from_index(i: u8) -> Self {
        match i {
            0..=127 => Cc::Controller(i),
            128 => Cc::NoteOn,
            129 => Cc::VelocityOn,
            130 => Cc::ChannelAftertouch,
            131 => Cc::PitchWheel,
            132 => Cc::NoteAftertouch,
            133 => Cc::NoteOff,
            134 => Cc::VelocityOff,
            _ => panic!(),
        }
    }
}

//struct MidiNote {
//    pub timestamp: u32,
//    pub pitch: u8,
//    pub channel: u8,
//    pub interface: u8,
//    pub is_on: bool,
//}
//
//pub struct MidiCc {
//    pub timestamp: u32,
//    pub cc: u8,
//    pub channel: u8,
//    pub interface: u8,
//    pub value_coarse: u8,
//    pub value_fine: u8,
//}
