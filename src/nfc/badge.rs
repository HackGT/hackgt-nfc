use std::fmt;
use std::borrow::Cow;
use url::Url;
use super::ndef::NDEF;

#[derive(Debug)]
pub struct CardResponse {
	pub status: [u8; 2],
	pub data: Vec<u8>,
}

/// Encapsulates PCSC errors and card response errors into a single error type
pub enum Error {
	PCSC(pcsc::Error),
	Response([u8; 2]),
	Message(&'static str),
}
impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::PCSC(pcsc_error) => write!(f, "{:?}", pcsc_error),
			Error::Response(bytes) => write!(f, "{:x?}", bytes),
			Error::Message(s) => write!(f, "{}", s),
		}
	}
}
impl From<pcsc::Error> for Error {
	fn from(err: pcsc::Error) -> Error {
		Error::PCSC(err)
	}
}
impl From<[u8; 2]> for Error {
	fn from(err: [u8; 2]) -> Error {
		Error::Response(err)
	}
}
impl From<&'static str> for Error {
	fn from(err: &'static str) -> Error {
		Error::Message(err)
	}
}

pub struct NFCBadge<'a> {
	card: &'a pcsc::Card,
}

impl NFCBadge<'_> {
	pub fn new(card: &pcsc::Card) -> NFCBadge {
		NFCBadge {
			card,
		}
	}

	pub fn get_user_id(&self) -> Result<String, Error> {
		/*
		Finally figured some cool stuff out:

		According to the datasheet (https://www.nxp.com/docs/en/data-sheet/NTAG213_215_216.pdf) for the NTAG213 series,
		the cards support a FAST_READ command that isn't part of the ISO/IEC 14443 standard. This command lets you
		read all of the memory pages on the card with one transaction. The standardized READ command (listed as
		Read Binary Blocks command in the ACR122U USB reader datasheet http://downloads.acs.com.hk/drivers/en/API-ACR122U-2.02.pdf)
		only allows reading up to 16 bytes at once (4 pages of 4 bytes each) which means reading the entire memory
		space of the card is extremely slow and error-prone (what happens if the tag is removed before all of the read
		operations have been executed?)

		These NXP-specific commands like FAST_READ don't use the typical APDU interface. Instead, we send:
		- A pseudo-APDU to the USB reader (0xFF, 0x00, 0x00, 0x00)
		- A length field telling the reader we're sending 5 raw bytes
		- The InCommunicateThru command (0xD4, 0x42) -- read by the card's PN532 NFC communcation controller
			See: https://www.nxp.com/docs/en/user-guide/141520.pdf section 7.3.9
		- The 0x3A FAST_READ command specified in the datasheet plus the start page and end page

		Outputs (according to the PN532 datasheet) will be: 0xD5, 0x43, and a status bytes (where 0x00 indicates success)

		This Stack Overflow answer has more related information:
		https://stackoverflow.com/questions/44237726/how-to-authenticate-ntag213-with-acr122u/44243037#44243037
		*/
		const START_PAGE: u8 = 0x04; // 0x00 through 0x03 contain tag-related info. User data starts at 0x04
		const END_PAGE: u8 = 0x27; // 0x27 is the last data page on the NTAG213
		let apdu = [0xFF, 0x00, 0x00, 0x00, 0x05, 0xD4, 0x42, 0x3A, START_PAGE, END_PAGE];
		let response = self.send_data(&apdu)?;

		if &response.data[0..3] != [0xD5, 0x43, 0x00] {
			return Err(Error::Message("Invalid PN532 response"));
		}
		let data = &response.data[3..];
		let message = NDEF::parse(data)?;
		let url = message.get_content().ok_or("NDEF message not URL")?;
		let url = Url::parse(&url).ok().ok_or("Invalid URL")?;

		for keyvalue in url.query_pairs() {
			match keyvalue.0 {
				Cow::Borrowed("user") => return Ok(keyvalue.1.to_string()),
				_ => {},
			}
		}
		Err(Error::Message("URL did not contain user ID"))
	}

	pub fn set_buzzer(&self, enabled: bool) -> Result<bool, Error> {
		let value = if enabled { 0xFF } else { 0x00 };
		let apdu = [0xFF, 0x00, 0x52, value, 0x00];
		self.send_data(&apdu)?;
		Ok(enabled)
	}

	pub(crate) fn send_data(&self, apdu: &[u8]) -> Result<CardResponse, Error> {
		let mut rapdu_buf = [0u8; pcsc::MAX_BUFFER_SIZE];
		let mut rapdu = self.card.transmit(apdu, &mut rapdu_buf)?.to_vec();

		if rapdu.len() < 2 {
			return Err(pcsc::Error::InvalidValue.into());
		}

		let status = [rapdu[rapdu.len() - 2], rapdu[rapdu.len() - 1]];
		rapdu.truncate(rapdu.len() - 2);
		// APDU response of 0x90, 0x00 means command executing successfully
		if status[0] == 0x90 && status[1] == 0x00 {
			Ok(CardResponse {
				status,
				data: rapdu,
			})
		}
		else {
			Err(status.into())
		}
	}
}
