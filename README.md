# hackgt-nfc

A portable Rust library for working with HackGT's NFC badges

Includes support for:
- The [HackGT Check-In](https://github.com/HackGT/checkin2) GraphQL API
	- Check users in / out and detect repeated check-in attempts
	- Get information about a user, given their user ID
	- Get information about current check-in tags
- Adding / removing users from a Check-In instance
- Finding and initializing ACR122U USB NFC readers
- Parsing NFC badge NDEF content into a usable user ID

Used by:
- [checkin-embedded](https://github.com/HackGT/checkin-embedded)
- [checkin-labels](https://github.com/HackGT/checkin-labels)
