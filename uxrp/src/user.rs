use aws_sdk_dynamodb::{model::AttributeValue, Client, Endpoint};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use uxrp_protocol::core::{Error, Result};

#[derive(Clone, Deserialize)]
pub struct UserStoreConfig {
	ddb_endpoint: Option<String>,
	auth_table_name: String,

	// test support
	#[serde(default)]
	randomise_id_prefix: bool,
}

#[derive(Clone)]
pub struct UserStore {
	client: Client,
	auth_table_name: String,
	id_prefix: String,
}

#[derive(Serialize, Deserialize)]
struct UserCredentials {
	id: String,
	user_id: String,
	password_hash: Vec<u8>,
	password_salt: Vec<u8>,
	email_verified: bool,
}

impl UserStore {
	pub async fn new(config: UserStoreConfig) -> Self {
		let mut ddb_config = aws_sdk_dynamodb::config::Builder::from(&aws_config::load_from_env().await);
		if let Some(ddb_endpoint) = config.ddb_endpoint {
			ddb_config =
				ddb_config.endpoint_resolver(Endpoint::immutable(ddb_endpoint.parse().expect("invalid ddb url")));
		}

		let client = Client::from_conf(ddb_config.build());
		Self {
			client,
			auth_table_name: config.auth_table_name,
			id_prefix: if config.randomise_id_prefix {
				format!("{}/", Uuid::new_v4())
			} else {
				"".to_owned()
			},
		}
	}

	fn auth_item_id(&self, email: &str) -> String {
		format!("{}auth/email/{}", self.id_prefix, email)
	}

	pub async fn create(&self, email: &str, password: &str) -> Result<String> {
		let user_id = Uuid::new_v4().to_string();
		let mut password_salt = Vec::new();
		password_salt.resize(128, 0);
		rand::thread_rng().fill_bytes(&mut password_salt);

		// TODO: store argon2 config versions and key auth items against them
		let password_hash = argon2::hash_raw(password.as_bytes(), &password_salt, &argon2::Config::default())?;

		let user_item = serde_dynamo::to_item(UserCredentials {
			id: self.auth_item_id(email),
			user_id: user_id.clone(),
			password_hash,
			password_salt,
			email_verified: false,
		})?;

		self.client
			.put_item()
			.table_name(&self.auth_table_name)
			.set_item(Some(user_item))
			.condition_expression("attribute_not_exists(id)")
			.send()
			.await?;

		Ok(user_id)
	}

	pub async fn authenticate(&self, email: &str, password: &str) -> Result<String> {
		let res = self
			.client
			.get_item()
			.table_name(&self.auth_table_name)
			.key("id", AttributeValue::S(self.auth_item_id(email)))
			.send()
			.await?;

		let creds: UserCredentials = serde_dynamo::from_item(res.item.ok_or(Error::InvalidCredentials)?)?;

		if argon2::verify_raw(
			password.as_bytes(),
			&creds.password_salt,
			&creds.password_hash,
			&argon2::Config::default(),
		)? {
			Ok(creds.user_id)
		} else {
			Err(Error::InvalidCredentials)
		}
	}
}
