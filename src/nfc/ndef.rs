use std::str;

#[derive(Debug, PartialEq)]
enum ParserState {
	None,
	NDEFInitial,
	NDEFTypeLength,
	NDEFPayloadLength,
	NDEFRecordType,
	NDEFData
}
#[derive(Debug, PartialEq)]
pub enum WellKnownType {
	Unknown,
	Text,
	URI
}

/// A very simple (and probably buggy) NDEF message parser based on TypeScript code I wrote for HackGT 5: https://github.com/HackGT/checkin-labels/blob/master/index.ts
pub struct NDEF {
	pub ndef_type: WellKnownType,
	pub data: Vec<u8>,
}

impl NDEF {
	pub fn parse(buffer: &[u8]) -> Result<Self, &'static str> {
		let mut state = ParserState::None;
		let mut data = Vec::with_capacity(0);
		let mut data_index: usize = 0;
		let mut ndef_type = WellKnownType::Unknown;

		let mut i: usize = 0;
		while i < buffer.len() {
			let byte = buffer[i];
			match state {
				ParserState::None => {
					if byte == 0x00 {
						// NULL block, skip
						i += 1;
					}
					else if byte == 0x03 && buffer.len() > i + 2 && buffer[i + 2] == 0xD1 {
						// NDEF message
						// Skip length field for now
						i += 1;
						state = ParserState::NDEFInitial;
					}
				},
				ParserState::NDEFInitial => {
					if (byte & 1 << 0) != 1 {
						return Err("Only NFC Well Known Records are supported");
					}
					if (byte & 1 << 4) == 0 {
						return Err("Only short records supported currently");
					}
					if (byte & 1 << 6) == 0 {
						return Err("Message must be end message currently");
					}
					if (byte & 1 << 7) == 0 {
						return Err("Message must be beginning message currently");
					}
					state = ParserState::NDEFTypeLength;
				},
				ParserState::NDEFTypeLength => {
					state = ParserState::NDEFPayloadLength;
				},
				ParserState::NDEFPayloadLength => {
					data = Vec::with_capacity(byte as usize);
					data_index = 0;
					state = ParserState::NDEFRecordType;
				},
				ParserState::NDEFRecordType => {
					ndef_type = match byte {
						0x54 => WellKnownType::Text,
						0x55 => WellKnownType::URI,
						_ => WellKnownType::Unknown,
					};
					state = ParserState::NDEFData;
				},
				ParserState::NDEFData => {
					// 0xFE terminates an NDEF message
					if byte == 0xFE {
						state = ParserState::None;
					}
					else {
						data.insert(data_index, byte);
						data_index += 1;
					}
				},
			}
			i += 1;
		}

		Ok(Self {
			ndef_type,
			data
		})
	}

	fn get_uri(&self) -> Option<String> {
		if self.data.len() < 2 || self.ndef_type != WellKnownType::URI {
			return None;
		}
		let url = str::from_utf8(&self.data[1..]).ok();
		url.map(|value| NDEF::get_protocol(self.data[0]).to_owned() + value)
	}

	fn get_text(&self) -> Option<String> {
		if self.data.len() < 4 || self.ndef_type != WellKnownType::Text {
			return None;
		}
		let language_code_length = self.data[0] as usize;
		str::from_utf8(&self.data[1 + language_code_length..]).ok().map(|value| value.to_owned())
	}

	pub fn get_content(&self) -> Option<String> {
		match self.ndef_type {
			WellKnownType::Text => self.get_text(),
			WellKnownType::URI => self.get_uri(),
			_ => None
		}
	}

	fn get_protocol(identifier: u8) -> &'static str {
		match identifier {
			0x00 => "",
			0x01 => "http://www.",
			0x02 => "https://www.",
			0x03 => "http://",
			0x04 => "https://",
			0x05 => "tel:",
			0x06 => "mailto:",
			0x07 => "ftp://anonymous:anonymous@",
			0x08 => "ftp://ftp.",
			0x09 => "ftps://",
			0x0A => "sftp://",
			0x0B => "smb://",
			0x0C => "nfs://",
			0x0D => "ftp://",
			0x0E => "dav://",
			0x0F => "news:",
			0x10 => "telnet://",
			0x11 => "imap:",
			0x12 => "rtsp://",
			0x13 => "urn:",
			0x14 => "pop:",
			0x15 => "sip:",
			0x16 => "sips:",
			0x17 => "tftp:",
			0x18 => "btspp://",
			0x19 => "btl2cap://",
			0x1A => "btgoep://",
			0x1B => "tcpobex://",
			0x1C => "irdaobex://",
			0x1D => "file://",
			0x1E => "urn: epc: id:",
			0x1F => "urn: epc: tag:",
			0x20 => "urn: epc: pat:",
			0x21 => "urn: epc: raw:",
			0x22 => "urn: epc:",
			0x23 => "urn: nfc:",
			_ => "",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::NDEF;
	fn compare_data(data: &[u8], answer: &str) {
		let parsed = NDEF::parse(&data).unwrap();
		assert_eq!(parsed.get_content().unwrap(), answer);
	}
	#[test]
	fn parse_uri() {
		let data = [0x1, 0x3, 0xa0, 0xc, 0x34, 0x3, 0x3b, 0xd1, 0x1, 0x37, 0x55, 0x4, 0x6c, 0x69, 0x76, 0x65, 0x2e, 0x68, 0x61, 0x63, 0x6b, 0x2e, 0x67, 0x74, 0x3f, 0x75, 0x73, 0x65, 0x72, 0x3d, 0x37, 0x64, 0x64, 0x30, 0x30, 0x30, 0x32, 0x31, 0x2d, 0x38, 0x39, 0x66, 0x64, 0x2d, 0x34, 0x39, 0x66, 0x31, 0x2d, 0x39, 0x63, 0x31, 0x37, 0x2d, 0x62, 0x64, 0x30, 0x62, 0x61, 0x37, 0x64, 0x63, 0x66, 0x39, 0x37, 0x65, 0xfe, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
		compare_data(&data, "https://live.hack.gt?user=7dd00021-89fd-49f1-9c17-bd0ba7dcf97e");
		let data = [0x0, 0x0, 0x1, 0x3, 0xa0, 0xc, 0x34, 0x3, 0x3c, 0xd1, 0x1, 0x38, 0x55, 0x4, 0x6c, 0x69, 0x76, 0x65, 0x2e, 0x68, 0x61, 0x63, 0x6b, 0x2e, 0x67, 0x74, 0x2f, 0x3f, 0x75, 0x73, 0x65, 0x72, 0x3d, 0x63, 0x65, 0x65, 0x32, 0x30, 0x35, 0x32, 0x30, 0x2d, 0x61, 0x65, 0x66, 0x30, 0x2d, 0x34, 0x36, 0x32, 0x31, 0x2d, 0x61, 0x66, 0x39, 0x37, 0x2d, 0x30, 0x62, 0x35, 0x31, 0x63, 0x38, 0x30, 0x63, 0x30, 0x64, 0x39, 0x63, 0xfe];
		compare_data(&data, "https://live.hack.gt/?user=cee20520-aef0-4621-af97-0b51c80c0d9c");
	}
}
