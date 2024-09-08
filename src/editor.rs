use crossbeam::queue::ArrayQueue;
use nih_plug::prelude::{Editor, GuiContext};
use nih_plug_iced::*;
use std::sync::Arc;

use crate::{cc_mon::{self, CcValueTime, State}, MidiMonitorParams};

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(400, 250)
}

pub(crate) fn create(
    params: Arc<MidiMonitorParams>,
    cc_queue: Arc<ArrayQueue<CcValueTime>>,
    editor_state: Arc<IcedState>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<GainEditor>(editor_state, (params, cc_queue))
}

struct GainEditor {
    #[allow(dead_code)]
    params: Arc<MidiMonitorParams>,
    context: Arc<dyn GuiContext>,

    cc_mon_state: cc_mon::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    // /// Update a parameter's value.
    //ParamUpdate(nih_widgets::ParamMessage),
}

impl IcedEditor for GainEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = (Arc<MidiMonitorParams>, Arc<ArrayQueue<CcValueTime>>);

    fn new(
        (params, cc_queue): Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        let editor = GainEditor {
            params,
            context,
            cc_mon_state: State::new(cc_queue),
        };

        (editor, Command::none())
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        _message: Self::Message,
    ) -> Command<Self::Message> {
        //match message {
        //    Message::ParamUpdate(message) => self.handle_param_message(message),
        //}
        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new()
            .align_items(Alignment::Center)
            .push(
                Text::new("MIDI Mon")
                    .font(assets::NOTO_SANS_LIGHT)
                    .size(40)
                    .height(50.into())
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .vertical_alignment(alignment::Vertical::Bottom),
            )
            .push(Space::with_height(10.into()))
            .push(
                cc_mon::CcMeter::new(&mut self.cc_mon_state)
                    .width(Length::Fill)
                    .height(100.into())
            )
            .into()
    }

    fn background_color(&self) -> Color { Color { r: 0.94, g: 0.96, b: 0.98, a: 1.0 } }
}
