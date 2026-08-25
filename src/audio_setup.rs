use iced::{
    Element,
    widget::{button, checkbox, column, pick_list, row, text},
};

/// Default number of periods used by the shared audio hardware setup screen.
pub const DEFAULT_N_PERIODS: usize = 2;

/// State required to render the shared audio hardware setup screen.
#[derive(Debug, Clone)]
pub struct AudioSetupState<Backend, Device> {
    pub backends: Vec<Backend>,
    pub selected_backend: Backend,
    pub show_input_device: bool,
    pub input_devices: Vec<Device>,
    pub selected_input_device: Option<Device>,
    pub show_output_device: bool,
    pub output_devices: Vec<Device>,
    pub selected_output_device: Option<Device>,
    pub show_sample_rate: bool,
    pub sample_rates: Vec<i32>,
    pub selected_sample_rate: Option<i32>,
    pub show_bit_depth: bool,
    pub bit_depths: Vec<usize>,
    pub selected_bit_depth: Option<usize>,
    pub show_period_frames: bool,
    pub period_frames: Vec<usize>,
    pub selected_period_frames: Option<usize>,
    pub show_n_periods: bool,
    pub n_periods: Vec<usize>,
    pub selected_n_periods: Option<usize>,
    pub show_exclusive: bool,
    pub exclusive: bool,
    pub show_sync_mode: bool,
    pub sync_mode: bool,
    pub plugins_loaded: bool,
    pub can_start: bool,
    pub status_message: String,
}

/// Every user action that the shared audio setup screen can produce.
#[derive(Debug, Clone)]
pub enum AudioSetupAction<Backend, Device> {
    BackendSelected(Backend),
    InputDeviceSelected(Device),
    OutputDeviceSelected(Device),
    SampleRateSelected(i32),
    BitDepthSelected(usize),
    PeriodFramesSelected(usize),
    NPeriodsSelected(usize),
    ExclusiveToggled(bool),
    SyncModeToggled(bool),
    Start,
}

/// Renders the shared audio hardware setup screen.
pub fn audio_setup<Backend, Device, Message>(
    state: AudioSetupState<Backend, Device>,
    on_action: impl Fn(AudioSetupAction<Backend, Device>) -> Message + Clone + 'static,
) -> Element<'static, Message>
where
    Backend: Clone + PartialEq + std::fmt::Display + 'static,
    Device: Clone + PartialEq + std::fmt::Display + 'static,
    Message: Clone + 'static,
{
    let AudioSetupState {
        backends,
        selected_backend,
        show_input_device,
        input_devices,
        selected_input_device,
        show_output_device,
        output_devices,
        selected_output_device,
        show_sample_rate,
        sample_rates,
        selected_sample_rate,
        show_bit_depth,
        bit_depths,
        selected_bit_depth,
        show_period_frames,
        period_frames,
        selected_period_frames,
        show_n_periods,
        n_periods,
        selected_n_periods,
        show_exclusive,
        exclusive,
        show_sync_mode,
        sync_mode,
        plugins_loaded: _,
        can_start,
        status_message,
    } = state;

    let mut content = column![pick_row(
        "Backend:",
        backends,
        Some(selected_backend),
        {
            let on_action = on_action.clone();
            move |b| on_action(AudioSetupAction::BackendSelected(b))
        },
        "Choose backend",
    )]
    .spacing(10);

    if show_input_device {
        content = content.push(pick_row(
            "Input device:",
            input_devices,
            selected_input_device,
            {
                let on_action = on_action.clone();
                move |d| on_action(AudioSetupAction::InputDeviceSelected(d))
            },
            "Choose input device",
        ));
    }

    if show_output_device {
        content = content.push(pick_row(
            "Output device:",
            output_devices,
            selected_output_device,
            {
                let on_action = on_action.clone();
                move |d| on_action(AudioSetupAction::OutputDeviceSelected(d))
            },
            "Choose output device",
        ));
    }

    if show_sample_rate {
        content = content.push(pick_row(
            "Sample rate (Hz):",
            sample_rates,
            selected_sample_rate,
            {
                let on_action = on_action.clone();
                move |r| on_action(AudioSetupAction::SampleRateSelected(r))
            },
            "Choose sample rate",
        ));
    }

    if show_bit_depth {
        content = content.push(pick_row(
            "Bit depth:",
            bit_depths,
            selected_bit_depth,
            {
                let on_action = on_action.clone();
                move |b| on_action(AudioSetupAction::BitDepthSelected(b))
            },
            "Bit depth",
        ));
    }

    if show_period_frames {
        let latency_text = match (
            selected_period_frames,
            selected_sample_rate,
            selected_n_periods,
        ) {
            (Some(frames), Some(rate), Some(nperiods)) if rate > 0 => {
                let latency_ms = (frames as f64 * nperiods as f64 * 1000.0) / rate as f64;
                text(format!("{latency_ms:.1} ms"))
            }
            _ => text(""),
        };
        content = content.push(
            row![
                text("Period frames:"),
                pick_list(period_frames, selected_period_frames, {
                    let on_action = on_action.clone();
                    move |v| on_action(AudioSetupAction::PeriodFramesSelected(v))
                },)
                .placeholder("Period"),
                latency_text,
            ]
            .spacing(10),
        );
    }

    if show_n_periods {
        content = content.push(pick_row(
            "N periods:",
            n_periods,
            selected_n_periods,
            {
                let on_action = on_action.clone();
                move |n| on_action(AudioSetupAction::NPeriodsSelected(n))
            },
            "N periods",
        ));
    }

    if show_exclusive {
        content = content.push(checkbox(exclusive).label("Exclusive mode").on_toggle({
            let on_action = on_action.clone();
            move |v| on_action(AudioSetupAction::ExclusiveToggled(v))
        }));
    }

    if show_sync_mode {
        content = content.push(checkbox(sync_mode).label("Sync mode").on_toggle({
            let on_action = on_action.clone();
            move |v| on_action(AudioSetupAction::SyncModeToggled(v))
        }));
    }

    let mut submit = button("Open Audio");
    if can_start {
        submit = submit.on_press(on_action(AudioSetupAction::Start));
    }
    content = content.push(submit);

    if !status_message.is_empty() {
        content = content.push(text(status_message).size(12));
    }

    content.into()
}

fn pick_row<'a, T, Message>(
    label: &'static str,
    options: Vec<T>,
    selected: Option<T>,
    on_select: impl Fn(T) -> Message + 'a,
    placeholder: &'static str,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + std::fmt::Display + 'a,
    Message: Clone + 'a,
{
    row![
        text(label),
        pick_list(options, selected, on_select).placeholder(placeholder)
    ]
    .spacing(10)
    .into()
}
