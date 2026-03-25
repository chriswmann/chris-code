use crate::events::{AppEvent, InputEvent};
use crossterm::event::{self, Event};
use std::sync::mpsc::Sender;

/// Runs in a background thread. Reads crossterm events and sends them
/// through the channel. This function blocks forever (or until the channel
/// is closed).
pub fn run(tx: &Sender<AppEvent>) {
    loop {
        // crossterm::eent::read() blocks until a key is pressed
        // or a terminal event occurs.
        match event::read() {
            Ok(Event::Key(key_event)) => {
                let app_event = AppEvent::Input(InputEvent::Key(key_event));
                // send() returns Err if the receiver has been dropped,
                // which means the main thread has exited, so we should too.
                if tx.send(app_event).is_err() {
                    break;
                }
            }
            Ok(_) => {
                // Mouse events, paste events, resize events, etc. We'll ignore for now.
            }
            Err(_) => {
                // If reading fails, exit the thread.
                break;
            }
        }
    }
}
