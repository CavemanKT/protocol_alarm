use chrono::Local;
use iced::widget::{button, column, row, text, text_input};
use iced::{Element, Length, Task};
use std::collections::HashSet;
use std::process::Command as ProcCommand;
use tokio::time::{sleep, Duration};

pub fn main() -> iced::Result {
    iced::run("Protocol Alarm", update, view)
}

const MAX_RECITES_PER_MATCH: u32 = 2;

#[derive(Default)]
struct ProtocolAlarmApp {
    current_time: String,
    alarm_time_input: String,      // single HH:MM input
    alarm_times: Vec<String>,      // list of alarm times
    status: String,
    ticking: bool,
    fired_minutes: HashSet<String>, // which HH:MM have already fired
}

#[derive(Debug, Clone)]
enum Message {
    Tick(String),
    AlarmTimeChanged(String),
    AddAlarmPressed,
    CancelAlarm(String),
    ReciteProtocol,
}

fn update(state: &mut ProtocolAlarmApp, message: Message) -> Task<Message> {
    match message {
        Message::Tick(now) => {
            state.current_time = now.clone();

            // If we moved to a new minute, clear fired markers
            if !state.fired_minutes.contains(&now) {
                state.fired_minutes.clear();
            }

            // Check if current time matches any alarm and hasn’t fired this minute
            if !state.alarm_times.is_empty()
                && state.alarm_times.contains(&now)
                && !state.fired_minutes.contains(&now)
            {
                state.status =
                    format!("Alarm triggered at {} – reciting protocol...", now);
                state.fired_minutes.insert(now.clone());

                for _ in 0..MAX_RECITES_PER_MATCH {
                    speak_protocol();
                }
            }

            // Only keep ticking while there is at least one alarm
            if state.ticking && !state.alarm_times.is_empty() {
                Task::perform(tick_loop(), Message::Tick)
            } else {
                state.ticking = false;
                Task::none()
            }
        }

        Message::AlarmTimeChanged(value) => {
            state.alarm_time_input = value;
            Task::none()
        }

        Message::AddAlarmPressed => {
            let raw = state.alarm_time_input.trim().to_string();
            if raw.len() == 5 && &raw[2..3] == ":" {
                if !state.alarm_times.contains(&raw) {
                    state.alarm_times.push(raw.clone());
                    state.status = format!("Added alarm: {}", raw);
                    state.alarm_time_input.clear();
                    state.ticking = true;
                    state.fired_minutes.clear();
                    Task::perform(tick_loop(), Message::Tick)
                } else {
                    state.status = format!("Alarm {} already exists", raw);
                    Task::none()
                }
            } else {
                state.status = "Invalid time (use HH:MM)".into();
                Task::none()
            }
        }

        Message::CancelAlarm(time_str) => {
            state.alarm_times.retain(|t| t != &time_str);
            state.status = format!("Cancelled alarm: {}", time_str);

            if state.alarm_times.is_empty() {
                state.ticking = false;
                Task::none()
            } else if state.ticking {
                Task::perform(tick_loop(), Message::Tick)
            } else {
                Task::none()
            }
        }

        Message::ReciteProtocol => {
            speak_protocol();
            Task::none()
        }
    }
}

fn view(state: &ProtocolAlarmApp) -> Element<'_, Message> {
    let header = text("Protocol Alarm").size(30);

    let time_row = row![
        text("Current time:"),
        text(&state.current_time).size(20)
    ]
    .spacing(8);

    let recite_button_row = row![
        button("Recite protocol now").on_press(Message::ReciteProtocol),
    ]
    .spacing(8);

    // Input row: add alarms one by one
    let input_row = row![
        text("Alarm (HH:MM):"),
        text_input("e.g. 09:30", &state.alarm_time_input)
            .on_input(Message::AlarmTimeChanged)
            .width(Length::Fixed(80.0)),
        button("Add alarm").on_press(Message::AddAlarmPressed),
    ]
    .spacing(8);

    // List alarms with cancel buttons
    let mut alarm_list = column![];
    if state.alarm_times.is_empty() {
        alarm_list = alarm_list.push(text("Alarms: None set"));
    } else {
        alarm_list = alarm_list.push(text("Alarms:"));
        for t in &state.alarm_times {
            alarm_list = alarm_list.push(
                row![
                    text(t.clone()),
                    button("Cancel").on_press(Message::CancelAlarm(t.clone()))
                ]
                .spacing(8),
            );
        }
    }

    let status = text(&state.status).size(16);

    column![
        header,
        time_row,
        recite_button_row,
        input_row,
        alarm_list,
        status
    ]
    .spacing(12)
    .padding(16)
    .into()
}

async fn tick_loop() -> String {
    sleep(Duration::from_secs(1)).await;
    Local::now().format("%H:%M").to_string()
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