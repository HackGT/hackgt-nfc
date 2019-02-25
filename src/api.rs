use std::fmt;
use url::Url;
use graphql_client::{ GraphQLQuery, Response };

pub enum Error {
	Network(reqwest::Error),
	Message(&'static str),
	GraphQL(Vec<graphql_client::Error>),
}
impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Network(err) => write!(f, "{:?}", err),
			Error::Message(s) => write!(f, "{}", s),
			Error::GraphQL(err) => write!(f, "{:?}", err),
		}
	}
}
impl From<reqwest::Error> for Error {
	fn from(err: reqwest::Error) -> Error {
		Error::Network(err)
	}
}
impl From<&'static str> for Error {
	fn from(err: &'static str) -> Error {
		Error::Message(err)
	}
}

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "schema.graphql",
	query_path = "api.graphql",
	response_derives = "Debug",
)]
struct UserSearch;

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "schema.graphql",
	query_path = "api.graphql",
	response_derives = "Debug",
)]
struct UserGet;

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "schema.graphql",
	query_path = "api.graphql",
	response_derives = "Debug",
)]
struct TagsGet;

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "schema.graphql",
	query_path = "api.graphql",
	response_derives = "Debug",
)]
struct CheckInTag;
pub type CheckInReturn = (bool, check_in_tag::UserData, check_in_tag::TagData);

pub struct CheckinAPI {
	base_url: Url,
	client: reqwest::Client,
	auth_token: String,
}

impl CheckinAPI {
	#[cfg(debug_assertions)]
	fn base_url() -> &'static str {
		"https://checkin.dev.hack.gt"
	}
	#[cfg(not(debug_assertions))]
	fn base_url() -> &'static str {
		"https://checkin.hack.gt"
	}

	pub fn login(username: &str, password: &str) -> Result<Self, Error> {
		let client = reqwest::Client::new();
		let base_url = Url::parse(CheckinAPI::base_url()).expect("Invalid base URL configured");

		let params = [("username", username), ("password", password)];
		let response = client.post(base_url.join("/api/user/login").unwrap())
			.form(&params)
			.send()?;

		if !response.status().is_success() {
			return Err("Invalid username or password".into());
		}

		let cookies = response.headers().get_all(reqwest::header::SET_COOKIE);
		let mut auth_token: Option<String> = None;
		let auth_regex = regex::Regex::new(r"^auth=(?P<token>[a-f0-9]+);").unwrap();
		for cookie in cookies.iter() {
			if let Ok(cookie) = cookie.to_str() {
				if let Some(capture) = auth_regex.captures(cookie) {
					auth_token = Some(capture["token"].to_owned());
				}
			}
		}

		match auth_token {
			Some(mut token) => {
				// Create a HTTP cookie header out of this token
				token.insert_str(0, "auth=");
				Ok(Self {
					base_url,
					client,
					auth_token: token,
				})
			},
			None => Err("No auth token set by server".into())
		}
	}

	pub fn from_token(mut auth_token: String) -> Self {
		let client = reqwest::Client::new();
		let base_url = Url::parse(CheckinAPI::base_url()).expect("Invalid base URL configured");
		// Create a HTTP cookie header out of this token
		auth_token.insert_str(0, "auth=");
		Self { base_url, client, auth_token }
	}

	// pub fn add_user(username: &str, password: &str) -> Result<Self, Error> {

	// }

	fn checkin_action(&self, check_in: bool, uuid: &str, tag: &str) -> Result<CheckInReturn, Error> {
		let body = CheckInTag::build_query(check_in_tag::Variables {
			id: uuid.to_string(),
			tag: tag.to_string(),
			checkin: check_in,
		});

		let response: Response<check_in_tag::ResponseData> = self.client.post(self.base_url.join("/graphql").unwrap())
			.header(reqwest::header::COOKIE, self.auth_token.as_str())
			.json(&body)
			.send()?
			.json()?;

		if let Some(errors) = response.errors {
			return Err(Error::GraphQL(errors));
		}
		if response.data.is_none() {
			return Err("Check in API returned no data".into());
		}

		let check_in_data = response.data.unwrap().check_in.unwrap();
		let user = check_in_data.user.user_data;
		if !user.accepted || !user.confirmed {
			return Err("User not accepted and confirmed".into());
		}

		let tag_details = check_in_data.tags.into_iter()
			.map(|item| item.tag_data)
			.find(|item| item.tag.name == tag)
			.unwrap(); // API ensures the tag we requested will be in the response so this won't panic

		Ok((
			tag_details.checkin_success,
			user,
			tag_details
		))
	}

	pub fn check_in(&self, uuid: &str, tag: &str) -> Result<CheckInReturn, Error> {
		self.checkin_action(true, uuid, tag)
	}
	pub fn check_out(&self, uuid: &str, tag: &str) -> Result<CheckInReturn, Error> {
		self.checkin_action(false, uuid, tag)
	}
}

#[cfg(test)]
mod checkin_api_tests {
	use super::CheckinAPI;

	#[test]
	fn login() {
		let username = std::env::var("USERNAME").unwrap();
		let password = std::env::var("PASSWORD").unwrap();

		let instance = CheckinAPI::login(username, password).unwrap();
		assert_eq!(instance.auth_token.len(), 64 + 5);

		dbg!(instance.check_in("7dd00021-89fd-49f1-9c17-bd0ba7dcf97e", "123").unwrap());
	}
}
