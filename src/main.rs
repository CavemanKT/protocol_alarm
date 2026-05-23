use chrono::{Datelike, Local};
use iced::widget::{button, column, row, text, text_input, radio};
use iced::{Element, Length, Task};
use std::collections::HashSet;
use std::process::Command as ProcCommand;
use tokio::time::{sleep, Duration};

pub fn main() -> iced::Result {
    iced::run("Protocol Alarm", update, view)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DayPattern {
    #[default]
    Weekdays,
    Weekends,
    Everyday,
}

impl DayPattern {
    fn label(&self) -> &'static str {
        match self {
            DayPattern::Weekdays => "Weekdays",
            DayPattern::Weekends => "Weekends",
            DayPattern::Everyday => "Everyday",
        }
    }

    fn applies_today(&self) -> bool {
        // ISO weekday: Monday = 1, Sunday = 7
        let today = Local::now().weekday().number_from_monday();
        match self {
            DayPattern::Weekdays => (1..=5).contains(&today),
            DayPattern::Weekends => (6..=7).contains(&today),
            DayPattern::Everyday => true,
        }
    }
}

#[derive(Clone)]
struct Alarm {
    time: String,        // "HH:MM"
    pattern: DayPattern, // when it repeats
    recites: u32,        // how many times to recite when it fires
}

#[derive(Default)]
struct ProtocolAlarmApp {
    current_time: String,
    alarm_time_input: String,      // single HH:MM input
    recites_input: String,         // user input for recites (string)
    selected_pattern: DayPattern,  // pattern for new alarms
    alarms: Vec<Alarm>,            // list of alarms
    status: String,
    ticking: bool,
    fired_minutes: HashSet<String>, // which HH:MM have already fired this minute
}

#[derive(Debug, Clone)]
enum Message {
    Tick(String),
    AlarmTimeChanged(String),
    RecitesChanged(String),
    AddAlarmPressed,
    CancelAlarm(usize),
    ReciteProtocol,
    PatternChanged(DayPattern),
}

fn update(state: &mut ProtocolAlarmApp, message: Message) -> Task<Message> {
    match message {
        Message::Tick(now) => {
            state.current_time = now.clone();

            // New minute? Clear fired markers so alarms can fire again later.
            if !state.fired_minutes.contains(&now) {
                state.fired_minutes.clear();
            }

            if !state.alarms.is_empty() && !state.fired_minutes.contains(&now) {
                let mut any_fired = false;

                for alarm in &state.alarms {
                    if alarm.time == now && alarm.pattern.applies_today() {
                        any_fired = true;
                        state.status = format!(
                            "Alarm at {} ({}, x{}) – reciting...",
                            alarm.time,
                            alarm.pattern.label(),
                            alarm.recites
                        );
                        for _ in 0..alarm.recites {
                            speak_protocol();
                        }
                    }
                }

                if any_fired {
                    state.fired_minutes.insert(now.clone());
                }
            }

            // Only keep ticking while there is at least one alarm
            if state.ticking && !state.alarms.is_empty() {
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

        Message::RecitesChanged(value) => {
            // Only allow digits; empty means "use default 1"
            if value.chars().all(|c| c.is_ascii_digit()) {
                state.recites_input = value;
            }
            Task::none()
        }

        Message::AddAlarmPressed => {
            let raw = state.alarm_time_input.trim().to_string();
            if raw.len() != 5 || &raw[2..3] != ":" {
                state.status = "Invalid time (use HH:MM)".into();
                return Task::none();
            }

            let recites = if state.recites_input.is_empty() {
                1
            } else {
                state.recites_input.parse::<u32>().unwrap_or(1)
            };

            if recites == 0 {
                state.status = "Recites must be at least 1".into();
                return Task::none();
            }

            // Avoid duplicates with same time + pattern + recites
            let exists = state
                .alarms
                .iter()
                .any(|a| a.time == raw && a.pattern == state.selected_pattern && a.recites == recites);

            if !exists {
                state.alarms.push(Alarm {
                    time: raw.clone(),
                    pattern: state.selected_pattern,
                    recites,
                });
                state.status = format!(
                    "Added alarm: {} ({}, x{})",
                    raw,
                    state.selected_pattern.label(),
                    recites
                );
                state.alarm_time_input.clear();
                state.recites_input.clear();
                state.ticking = true;
                state.fired_minutes.clear();
                Task::perform(tick_loop(), Message::Tick)
            } else {
                state.status = format!(
                    "Alarm {} ({}, x{}) already exists",
                    raw,
                    state.selected_pattern.label(),
                    recites
                );
                Task::none()
            }
        }

        Message::CancelAlarm(index) => {
            if index < state.alarms.len() {
                let removed = state.alarms.remove(index);
                state.status = format!(
                    "Cancelled alarm: {} ({}, x{})",
                    removed.time,
                    removed.pattern.label(),
                    removed.recites
                );
            }

            if state.alarms.is_empty() {
                state.ticking = false;
                Task::none()
            } else if state.ticking {
                Task::perform(tick_loop(), Message::Tick)
            } else {
                Task::none()
            }
        }

        Message::ReciteProtocol => {
            speak_protocol(); // manual trigger, single recite
            Task::none()
        }

        Message::PatternChanged(pattern) => {
            state.selected_pattern = pattern;
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

    // Day pattern selection
    let pattern_row = row![
        text("Repeat on: "),
        radio(
            "Weekdays",
            DayPattern::Weekdays,
            Some(state.selected_pattern),
            Message::PatternChanged
        ),
        radio(
            "Weekends",
            DayPattern::Weekends,
            Some(state.selected_pattern),
            Message::PatternChanged
        ),
        radio(
            "Everyday",
            DayPattern::Everyday,
            Some(state.selected_pattern),
            Message::PatternChanged
        ),
    ]
    .spacing(8);

    // Input row: add alarms one by one, with recites
    let input_row = row![
        text("Alarm (HH:MM):"),
        text_input("e.g. 09:30", &state.alarm_time_input)
            .on_input(Message::AlarmTimeChanged)
            .width(Length::Fixed(80.0)),
        text("Recites:"),
        text_input("1", &state.recites_input)
            .on_input(Message::RecitesChanged)
            .width(Length::Fixed(40.0)),
        button("Add alarm").on_press(Message::AddAlarmPressed),
    ]
    .spacing(8);

    // List alarms with cancel buttons
    let mut alarm_list = column![];
    if state.alarms.is_empty() {
        alarm_list = alarm_list.push(text("Alarms: None set"));
    } else {
        alarm_list = alarm_list.push(text("Alarms:"));
        for (i, alarm) in state.alarms.iter().enumerate() {
            let label = format!(
                "{} ({}, x{})",
                alarm.time,
                alarm.pattern.label(),
                alarm.recites
            );
            alarm_list = alarm_list.push(
                row![
                    text(label),
                    button("Cancel").on_press(Message::CancelAlarm(i))
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
        pattern_row,
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