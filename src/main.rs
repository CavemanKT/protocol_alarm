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
    alarm_time_input: String,   // raw user input, comma-separated HH:MM
    alarm_times: Vec<String>,   // parsed list of alarm times
    status: String,
    ticking: bool,
    fired_minutes: HashSet<String>, // which HH:MM have already fired
}

#[derive(Debug, Clone)]
enum Message {
    Tick(String),
    AlarmTimeChanged(String),
    SetAlarmPressed,
    ReciteProtocol,
    StopScheduler,
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
            if state.alarm_times.contains(&now) && !state.fired_minutes.contains(&now) {
                state.status = format!("Alarm triggered at {} – reciting protocol...", now);
                state.fired_minutes.insert(now.clone());
                // Recite protocol limited by MAX_RECITES_PER_MATCH
                for _ in 0..MAX_RECITES_PER_MATCH {
                    speak_protocol();
                }
            }

            // Always schedule next tick if ticking is on
            if state.ticking {
                Task::perform(tick_loop(), Message::Tick)
            } else {
                Task::none()
            }
        }

        Message::AlarmTimeChanged(value) => {
            state.alarm_time_input = value;

            // Start ticking lazily when user starts typing
            if !state.ticking && !state.alarm_time_input.is_empty() {
                state.ticking = true;
                Task::perform(tick_loop(), Message::Tick)
            } else {
                Task::none()
            }
        }

        Message::SetAlarmPressed => {
            let raw = state.alarm_time_input.trim();
            if raw.is_empty() {
                state.status = "Please enter at least one time".into();
                return Task::none();
            }

            let mut new_times = Vec::new();
            let mut all_valid = true;

            for part in raw.split(',') {
                let t = part.trim().to_string();
                if t.len() == 5 && &t[2..3] == ":" {
                    new_times.push(t);
                } else {
                    all_valid = false;
                    break;
                }
            }

            if all_valid && !new_times.is_empty() {
                state.alarm_times = new_times;
                state.status = format!(
                    "Alarms set for: {}",
                    state.alarm_times.join(", ")
                );
                state.ticking = true;
                state.fired_minutes.clear();
                Task::perform(tick_loop(), Message::Tick)
            } else {
                state.status = "Invalid time(s) (use HH:MM, separated by commas)".into();
                Task::none()
            }
        }

        Message::ReciteProtocol => {
            speak_protocol();
            Task::none()
        }

        Message::StopScheduler => {
            state.ticking = false;
            state.status = "Scheduler stopped. No alarms will fire.".into();
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

    let input_row = row![
        text("Alarms (HH:MM, comma separated):"),
        text_input("e.g. 09:30, 13:00", &state.alarm_time_input)
            .on_input(Message::AlarmTimeChanged)
            .width(Length::FillPortion(2)),
        button("Set alarms").on_press(Message::SetAlarmPressed),
    ]
    .spacing(8);

    let alarm_info = text(if state.alarm_times.is_empty() {
        "Alarms set: None".into()
    } else {
        format!("Alarms set: {}", state.alarm_times.join(", "))
    });

    let status = text(&state.status).size(16);

    column![header, time_row, recite_button_row, input_row, alarm_info, status]
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