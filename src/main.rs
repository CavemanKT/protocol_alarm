mod config;
use crate::config::{ load_protocol, save_protocol };

use chrono::{ Datelike, Local };
use iced::widget::{ button, column, container, row, scrollable, text, text_input, radio };
use iced::{ Element, Length, Task };
use std::collections::HashSet;
use std::process::Command as ProcCommand;
use tokio::time::{ sleep, Duration };

use iced::{ Theme, Color };

use iced::widget::text_editor; // if you have the multi-line editor in your version
// or keep text_input for now


pub fn main() -> iced::Result {
    iced::run(
        "Protocol Alarm",
        update,
        view,
    )
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
    time: String, // "HH:MM"
    pattern: DayPattern, // when it repeats
    recites: u32, // how many times to recite when it fires
}

#[derive(Default)]
struct ProtocolAlarmApp {
    current_time: String,
    alarm_time_input: String, // single HH:MM input
    recites_input: String, // user input for recites (string)
    selected_pattern: DayPattern, // pattern for new alarms
    alarms: Vec<Alarm>, // list of alarms
    status: String,
    ticking: bool,
    fired_minutes: HashSet<String>, // which HH:MM have already fired this minute

    protocol_text: String, // contents loaded from file
    protocol_dirty: bool, // whether user has unsaved edits
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

    ProtocolChanged(String),
    SaveProtocolPressed,
}

fn update(state: &mut ProtocolAlarmApp, message: Message) -> Task<Message> {
    match message {
        Message::Tick(now) => {
            state.current_time = now.clone();

            // If this is the first Tick after startup, start ticking
            if !state.ticking {
                state.ticking = true;
            }

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
                            speak_protocol(&state.protocol_text);
                        }
                    }
                }

                if any_fired {
                    state.fired_minutes.insert(now.clone());
                }
            }

            // Keep ticking while there is at least one alarm
            if state.ticking {
                Task::perform(tick_loop(), Message::Tick)
            } else {
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
            let exists = state.alarms
                .iter()
                .any(
                    |a| a.time == raw && a.pattern == state.selected_pattern && a.recites == recites
                );

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

                // Start ticking if we weren't already
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
            speak_protocol(&state.protocol_text); // manual trigger, single recite
            Task::none()
        }

        Message::PatternChanged(pattern) => {
            state.selected_pattern = pattern;
            Task::none()
        }

        Message::ProtocolChanged(value) => {
            state.protocol_text = value;
            state.protocol_dirty = true;
            Task::none()
        }

        Message::SaveProtocolPressed => {
            match save_protocol(&state.protocol_text) {
                Ok(_) => {
                    state.status = "Saved protocol.".into();
                    state.protocol_dirty = false;
                }
                Err(e) => {
                    state.status = format!("Failed to save protocol: {}", e);
                }
            }
            Task::none()
        }
    }
}

fn view(state: &ProtocolAlarmApp) -> Element<'_, Message> {
    // Header styled a bit more like your target UI
    let header = container(
        column![
            text("Recite Protocol").size(24),
            text("Scheduled sentence recitation for macOS").size(14),
            text("Tip: grant Accessibility permission for full kiosk lock during recitation.").size(
                14
            )
        ].spacing(4)
    )
        .width(Length::Fill)
        .padding(8);

    // Current time + manual recite
    let time_row = row![
        text("Current time:").size(16),
        text(&state.current_time).size(32),
        // Recite now button
        button("Recite now")
            .padding([6, 12])
            .style(primary_button_style)
            .width(Length::Fixed(100.0))
            .on_press(Message::ReciteProtocol)
    ].spacing(16);

    let time_card = container(time_row).padding(12).width(Length::Fill);

    // Day pattern selection
    let pattern_row = row![
        text("Repeat on:").size(16),
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
        )
    ].spacing(12);

    // Input row: time + recites
    let input_row = row![
        column![
            text("Alarm time").size(14),
            text_input("HH:MM", &state.alarm_time_input)
                .on_input(Message::AlarmTimeChanged)
                .width(Length::Fixed(80.0))
        ].spacing(4),
        column![
            text("Recites").size(14),
            text_input("1", &state.recites_input)
                .on_input(Message::RecitesChanged)
                .width(Length::Fixed(50.0))
        ].spacing(4),
        // Add alarm button
        button("Add alarm")
            .padding([8, 14])
            .style(primary_button_style)
            .width(Length::Fixed(100.0))
            .on_press(Message::AddAlarmPressed)
    ].spacing(16);

    let config_card = container(column![pattern_row, input_row].spacing(12))
        .padding(12)
        .width(Length::Fill);

    // List alarms with cancel buttons
    let mut alarm_list = column![];
    if state.alarms.is_empty() {
        alarm_list = alarm_list.push(text("No alarms set yet.").size(14));
    } else {
        for (i, alarm) in state.alarms.iter().enumerate() {
            let label = format!("{} • {} • x{}", alarm.time, alarm.pattern.label(), alarm.recites);
            alarm_list = alarm_list.push(
                container(
                    row![
                        text(label).size(16),
                        // Cancel button
                        button("Cancel")
                            .padding([4, 10])
                            .style(secondary_button_style)
                            .width(Length::Fixed(100.0))
                            .on_press(Message::CancelAlarm(i))
                    ].spacing(12)
                ).padding(8)
            );
        }
    }

    let alarm_card = container(
        column![
            text("Scheduled alarms").size(18),
            scrollable(alarm_list).height(Length::Fixed(180.0))
        ].spacing(8)
    )
        .padding(12)
        .width(Length::Fill);

    let status_text = if state.status.is_empty() {
        text("Ready.").size(14)
    } else {
        text(&state.status).size(14)
    };

    let protocol_section = container(
        column![
            text("Protocol text").size(18),
            // if you have text_editor in your version:
            // text_editor(&state.protocol_text)
            //     .on_edit(Message::ProtocolChanged)
            //     .height(Length::Fixed(150.0)),

            // Fallback: simple text_input (single line)
            text_input("Protocol...", &state.protocol_text)
                .on_input(Message::ProtocolChanged)
                .width(Length::Fill),

            row![
                button(if state.protocol_dirty { "Save protocol" } else { "Saved" })
                    .padding([6, 12])
                    .style(primary_button_style)
                    .on_press(Message::SaveProtocolPressed)
            ].spacing(8)
        ].spacing(8)
    )
        .padding(12)
        .width(Length::Fill);

    let content = column![header, time_card, config_card, alarm_card, protocol_section, status_text]
        .spacing(16)
        .padding(16)
        .max_width(520);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

async fn tick_loop() -> String {
    sleep(Duration::from_secs(1)).await;
    Local::now().format("%H:%M").to_string()
}

fn primary_button_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        // dark-ish rectangle
        background: Some(iced::Background::Color(Color::from_rgb(0.2, 0.22, 0.25))),
        // bright text
        text_color: Color::from_rgb(0.95, 0.95, 0.97),
        // subtle rounded border
        border: iced::Border {
            color: Color::from_rgb(0.35, 0.35, 0.4),
            width: (1.0).into(),
            radius: (6.0).into(),
        },
        shadow: iced::Shadow::default(),
    }
}

fn secondary_button_style(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.16, 0.17, 0.19))),
        text_color: Color::from_rgb(0.8, 0.82, 0.86),
        border: iced::Border {
            color: Color::from_rgb(0.3, 0.32, 0.35),
            width: (1.0).into(),
            radius: (6.0).into(),
        },
        shadow: iced::Shadow::default(),
    }
}

fn speak_protocol(text: &str) {
    let status = ProcCommand::new("say").arg("-r").arg("180").arg(load_protocol()).status();

    if let Err(e) = status {
        eprintln!("Failed to execute `say`: {}", e);
    }
}
