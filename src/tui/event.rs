use crate::tui::action::Action;
use crossterm::event::{Event as CrosstermEvent, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Action>,
    #[allow(dead_code)]
    sender: mpsc::UnboundedSender<Action>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let event_sender = sender.clone();

        tokio::spawn(async move {
            loop {
                let polled = crossterm::event::poll(Duration::from_millis(50)).unwrap_or(false);
                if polled {
                    match crossterm::event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            let is_press = key.kind == KeyEventKind::Press;
                            if is_press && event_sender.send(Action::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::FocusGained) => {
                            let _ = event_sender.send(Action::FocusChange);
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if event_sender.send(Action::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                tokio::task::yield_now().await;
            }
        });

        let tick_sender = sender.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tick_sender.send(Action::Tick).is_err() {
                    break;
                }
            }
        });

        Self { receiver, sender }
    }

    pub async fn next(&mut self) -> Option<Action> {
        self.receiver.recv().await
    }
}
