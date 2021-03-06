use pcsc::*;
use std::thread::{ self, JoinHandle };
use std::collections::HashMap;
use std::ffi::CStr;

mod badge;
mod ndef;
pub use badge::NFCBadge;

pub fn handle_cards<F, G>(card_handler: F, reader_handler: G) -> JoinHandle<()>
	where F: Fn(&Card, &CStr, usize),
		  F: Send + 'static,
		  G: Fn(&CStr, bool),
		  G: Send + 'static,
{
	thread::spawn(move || {
		let mut ctx = Context::establish(Scope::User).expect("Failed to establish context");

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
			reader_states.retain(|rs| {
				let should_keep = !is_invalid(rs);
				if !should_keep {
					// Notify about removal
					reader_handler(rs.name(), false);
				}
				should_keep
			});

			// Add new readers
			let names = match ctx.list_readers(&mut readers_buf) {
				Ok(names) => names,
				Err(pcsc::Error::ServiceStopped) | Err(pcsc::Error::NoService) => {
					// Windows will kill the SmartCard service when the last reader is disconnected
					// Restart it and wait (sleep) for a new reader connection if that occurs
					ctx = Context::establish(Scope::User).expect("Failed to establish context");
					continue;
				}
				Err(err) => { panic!("Failed to list readers: {:?}", err) }
			};

			for name in names {
				// Ignore the pseudo reader created by Windows Hello
				if !reader_states.iter().any(|rs| rs.name() == name) && !name.to_str().unwrap().contains("Windows Hello") {
					reader_handler(name, true);
					reader_states.push(ReaderState::new(name, State::UNAWARE));
				}
			}

			// Update the view of the state to wait on
			for rs in &mut reader_states {
				rs.sync_current_state();
			}

			// Wait until the state changes
			match ctx.get_status_change(None, &mut reader_states) {
				Ok(()) => {},
				Err(pcsc::Error::ServiceStopped) | Err(pcsc::Error::NoService) => {
					// Windows will kill the SmartCard service when the last reader is disconnected
					// Restart it and wait (sleep) for a new reader connection if that occurs
					ctx = Context::establish(Scope::User).expect("Failed to establish context");
					continue;
				}
				Err(err) => { panic!("Failed to get status change: {:?}", err) }
			};

			for (reader_index, rs) in reader_states.iter().enumerate() {
				if rs.name() == PNP_NOTIFICATION() { continue; }

				let name = rs.name().to_owned();
				// Debounce repeated events
				if rs.event_state().intersects(State::PRESENT) {
					if !readers.get(&name).unwrap_or(&false) {
						// Card is tapped
						// Connect to the card.
						match ctx.connect(rs.name(), ShareMode::Shared, Protocols::ANY) {
							Ok(card) => card_handler(&card, rs.name(), reader_index),
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
