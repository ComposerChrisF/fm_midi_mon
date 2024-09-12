

#[derive(Copy, Clone, Debug)]
pub enum Cc {
    Controller(u8),
    NoteOn,
    VelocityOn,
    ChannelAftertouch,
    PitchBend ,
    NoteAftertouch,
    NoteOff,
    VelocityOff,
}

pub const CC_MAX: usize = 135;      // Number of MIDI CC's, including our internal additions

impl Cc {
    pub fn to_index(self) -> u8 {
        match self {
            Cc::Controller(c)     => c & 0x7f,
            Cc::NoteAftertouch    => 128,   // This value matches VST3's IMidiMapping
            Cc::PitchBend         => 129,   // This value matches VST3's IMidiMapping
            Cc::ChannelAftertouch => 130,
            Cc::NoteOn            => 131,
            Cc::VelocityOn        => 132,
            Cc::NoteOff           => 133,
            Cc::VelocityOff       => 134,
        }
    }

    pub fn from_index(i: u8) -> Self {
        match i {
            0..=127 => Cc::Controller(i),
            128 => Cc::NoteAftertouch,
            129 => Cc::PitchBend,
            130 => Cc::ChannelAftertouch,
            131 => Cc::NoteOn,
            132 => Cc::VelocityOn,
            133 => Cc::NoteOff,
            134 => Cc::VelocityOff,
            _ => panic!(),
        }
    }
}

// TODO: Add test to make sure round-trip mapping works! And that CC_MAX is correct (exactly one enum value + 1 should equal CC_MAX)
