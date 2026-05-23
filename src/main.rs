use chrono::Local;
use iced::widget::{button, column, row, text, text_input};
use iced::{Element, Length};
use std::process::Command as ProcCommand;

pub fn main() -> iced::Result {
    iced::run("Protocol Alarm", update, view)
}

#[derive(Default)]
struct ProtocolAlarmApp {
    current_time: String,
    alarm_time_input: String,
    alarm_time_set: Option<String>,
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    AlarmTimeChanged(String),
    SetAlarmPressed,
    ReciteProtocol,
}

fn update(state: &mut ProtocolAlarmApp, message: Message) -> iced::Task<Message> {
    // Update current time on every update
    state.current_time = Local::now().format("%H:%M").to_string();

    match message {
        Message::AlarmTimeChanged(value) => {
            state.alarm_time_input = value;
        }
        Message::SetAlarmPressed => {
            if state.alarm_time_input.len() == 5 && &state.alarm_time_input[2..3] == ":" {
                state.alarm_time_set = Some(state.alarm_time_input.clone());
                state.status = format!("Alarm set for {}", state.alarm_time_input);
                
            } else {
                state.status = "Invalid time (use HH:MM)".into();
            }
        }

        Message::ReciteProtocol => {
            speak_protocol();
        }
    }
    iced::Task::none()
}

fn view(state: &ProtocolAlarmApp) -> Element<'_, Message> {
    let header = text("Protocol Alarm").size(30);

    let time_row = row![
        text("Current time:"),
        text(&state.current_time).size(20)
    ]
    .spacing(8);

    let recite_button_row = row![
            button("recite protocol").on_press(Message::ReciteProtocol),
        ]
    .spacing(8);

    let input_row = row![

        text("Alarm (HH:MM):"),
        text_input("e.g. 09:30", &state.alarm_time_input)
            .on_input(Message::AlarmTimeChanged)
            .width(Length::Fixed(80.0)),
            button("Set alarm").on_press(Message::SetAlarmPressed),

    ]
    .spacing(8);

    let alarm_info = text(
        match &state.alarm_time_set {
            Some(t) => format!("Alarm set: {}", t),
            None => "Alarm set: None".into(),
        },
    );

    let status = text(&state.status).size(16);

    column![header, time_row, recite_button_row, input_row, alarm_info, status]
        .spacing(12)
        .padding(16)
        .into()
}

fn speak_protocol() {
    let text = r#"Protocol.

Follow only these rules.

Rule 1: Pay attention only when price is near the middle Bollinger Band.
Rule 2: Never chase overbought or oversold prices.
Rule 3: Always use a stop loss.
Rule 4: Never average down.
Rule 5: Stop trading when the account is down to 300 at 0.01 lot size, or 3000 at 0.3 lot size.

For clarity:
Step 1: Meditate using box breathing as often as possible.
Step 2: Spend the remaining time practicing guitar."#;

    let status = ProcCommand::new("say")
        .arg("-r")
        .arg("180")
        .arg(text)
        .status();

    if let Err(e) = status {
        eprintln!("Failed to execute `say`: {}", e);
    }
}