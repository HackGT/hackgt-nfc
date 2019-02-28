use pcsc::*;
use std::thread::{ self, JoinHandle };
use std::collections::HashMap;
use std::ffi::CStr;

mod badge;
mod ndef;
pub use badge::NFCBadge;

pub fn handle_cards<F>(handler: F) -> JoinHandle<()>
	where F: Fn(&Card, &CStr, usize),
		  F: Send + 'static
{
	thread::spawn(move || {
		let ctx = Context::establish(Scope::User).expect("Failed to establish context");

		let mut readers_buf = [0; 2048];
		let mut reader_states = vec![
			// Listen for reader insertions/removals, if supported
			ReaderState::new(PNP_NOTIFICATION(), State::UNAWARE),
		];
		// Keeps track of which readers have an active card
		let mut readers = HashMap::new();
		loop {
			// Remove dead readers
			fn is_invalid(rs: &ReaderState) -> bool {
				rs.event_state().intersects(State::UNKNOWN | State::IGNORE)
			}
			reader_states.retain(|rs| !is_invalid(rs));

			// Add new readers
			let names = ctx.list_readers(&mut readers_buf).expect("Failed to list readers");
			for name in names {
				// Ignore the pseudo reader created by Windows Hello
				if !reader_states.iter().any(|rs| rs.name() == name) && !name.to_str().unwrap().contains("Windows Hello") {
					println!("Adding {:?}", name);
					reader_states.push(ReaderState::new(name, State::UNAWARE));
				}
			}

			// Update the view of the state to wait on
			for rs in &mut reader_states {
				rs.sync_current_state();
			}

			// Wait until the state changes
			ctx.get_status_change(None, &mut reader_states).expect("Failed to get status change");
			for (reader_index, rs) in reader_states.iter().enumerate() {
				if rs.name() == PNP_NOTIFICATION() { continue; }

				let name = rs.name().to_owned();
				// Debounce repeated events
				if rs.event_state().intersects(State::PRESENT) {
					if !readers.get(&name).unwrap_or(&false) {
						// Card is tapped
						// Connect to the card.
						match ctx.connect(rs.name(), ShareMode::Shared, Protocols::ANY) {
							Ok(card) => handler(&card, rs.name(), reader_index),
							Err(Error::NoSmartcard) => {
								eprintln!("A smartcard is not present in the reader");
							}
							Err(err) => {
								eprintln!("Failed to connect to card: {}", err);
							}
						};
					}
					readers.insert(name, true);
				}
				else if rs.event_state().intersects(State::EMPTY) {
					readers.insert(name, false);
				}
			}
		}
	})
}
