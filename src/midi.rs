

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


// TODO: Make this table one that gets loaded from disk (perhaps with a watch on the file for easy
// reloading).  This will help prepare for load/save of user settings.
pub const NAMES: [Option<&str>; CC_MAX] = [
    Some("BS"),     // 0: Bank Select (MSB) (see CC32)
    Some("mod"),    // 1: Mod Wheel (MSB) (see CC33)
    Some("br"),     // 2: Breath Controller (MSB) (see CC34)
    None,
    None,           // 4: Foot Controller (MSB) (see CC36)
    Some("tim"),    // 5: Portomento Time (MSB) (see CC37)
    Some("DEM"),    // 6: Data Entry (MSB) (see CC38)
    Some("vol"),    // 7: Volume (MSB) (see CC39)
    Some("bal"),    // 8: Balance (MSB) (see CC40)
    None,
    Some("pan"),    // 10: Pan (MBB) (see CC42)
    Some("exp"),    // 11: Expression Pedal (MSB) (see CC43)
    None,           // 12: Effect Controller 1 (MSB) (see CC44)
    None,           // 13: Effect Controller 2 (MSB) (see CC45)
    None,
    None,           // 15
    None,           // 16: General Controller 1 (Knob/Slider/Ribbon) (see CC48)
    None,           // 17: General Controller 2 (Knob/Slider/Ribbon) (see CC49)
    None,           // 18: General Controller 3 (Knob/Slider/Ribbon) (see CC50)
    None,           // 19: General Controller 4 (Knob/Slider/Ribbon) (see CC51)
    None,           // 20
    None,
    None,
    None,
    None,
    None,           // 25
    None,
    None,
    None,
    None,
    None,           // 30
    None,
    Some("BSL"),    // 32: Bank Select LSB (see CC0)
    None,           // 33: Mod Wheel LSB (see CC1)
    None,           // 34: Breath Controller LSB (See CC2)
    None,           // 35
    None,
    None,           // 37: Portamento Time LSB (see CC5)
    Some("DEL"),    // 38: Data Entry LSB (see CC6)
    None,           // 39: Volume LSB (see CC7)
    None,           // 40: Balance LSB (see CC8)
    None,
    None,           // 42: Pan LSB (see CC10)
    None,           // 43: Expression LSB (see CC11)
    None,           // 44: Effect Control 1 LSB (see CC12)
    None,           // 45: Effect Control 2 LSB (see CC13)
    None,
    None,
    None,           // 48: General Controller 1 LSB (see CC16)
    None,           // 49: General Controller 2 LSB (see CC17)
    None,           // 50: General Controller 3 LSB (see CC18)
    None,           // 51: General Controller 4 LSB (see CC19)
    None,
    None,
    None,
    None,           // 55
    None,
    None,
    None,
    None,
    None,           // 60
    None,
    None,
    None,
    Some("sus"),    // 64: Sustain Pedal (on/off)
    Some("prt"),    // 65: Portamento (on/off)
    Some("sos"),    // 66: Sostenuto (on/off)
    Some("sft"),    // 67: Soft Pedal (on/off)
    Some("leg"),    // 68: Legato (on/off)
    Some("hld"),    // 69: Hold Pedal 2
    None,           // 70: Sound Controller 1
    None,           // 71: Sound Controller 2 (Sound Variation)
    None,           // 72: Sound Controller 3 (Release Time)
    None,           // 73: Sound Controller 4 (Attack Time)
    None,           // 74: Sound Controller 5 (Brightness)
    None,           // 75: Sound Controller 6
    None,           // 76: Sound Controller 7
    None,           // 77: Sound Controller 8
    None,           // 78: Sound Controller 9
    None,           // 79: Sound Controller 10
    None,           // 80: General Purpose 1 (on/off)
    None,           // 81: General Purpose 2 (on/off)
    None,           // 82: General Purpose 3 (on/off)
    None,           // 83: General Purpose 4 (on/off)
    None,           // 84: Portamento Controls (value = Source Note)
    None,           // 85
    None,
    None,
    None,
    None,
    None,           // 90
    None,           // 91: Effect 1 Depth
    None,           // 92: Effect 2 Depth
    None,           // 93: Effect 3 Depth
    None,           // 94: Effect 4 Depth
    None,           // 95: Effect 5 Depth
    Some("+1"),     // 96: Data Increment (+1) for SysEx, NRPN, RPN
    Some("-1"),     // 97: Data Decrement (-1) for SysEx, NRPN, RPN
    Some("nrL"),    // 98: NRPN LSB (Selects the NRPN variable for CC6/CC38, CC96, CC97)
    Some("nrM"),    // 99: NRPN MSB
    Some("rpL"),    // 100: RPN LSB
    Some("rpM"),    // 101: RPN MSB
    None,
    None,
    None,
    None,           // 105
    None,
    None,
    None,
    None,
    None,           // 110
    None,
    None,
    None,
    None,
    None,           // 115
    None,
    None,
    None,
    None,
    Some("mut"),    // 120: Channel Mute
    Some("RAC"),    // 121: Reset All Controllers
    Some("LOC"),    // 122: Local Control (on/off) (0=Off, 127=On)
    Some("AN"),     // 123: All Notes (value = 0)
    Some("OMF"),    // 124: OMNI Mode OFF (value = 0; includes All Notes Off)
    Some("OMN"),    // 125: OMNI Mode ON (value = 0; includes All Notes Off)
    None,           // 126: Mono Mode (includes All Notes Off, Poly Off)
    None,           // 127: Poly Mode (value = 0; includes All Notes Off, Mono Off)
    Some("pat"),    // 128: Polyphonic Aftertouch
    Some("pit"),    // 129: PitchBend (float)
    Some("cat"),    // 130: Channel Aftertouch
    Some("on"),     // 131: Note On
    Some("vel"),    // 132: Velocity (Note On)
    Some("off"),    // 133: Note Off
    Some("ov"),     // 134: Off Velocity (Note Off)
];