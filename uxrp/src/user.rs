use aws_sdk_dynamodb::{model::AttributeValue, Blob, Client, Endpoint};
use rand::prelude::*;
use serde::Deserialize;
use uuid::Uuid;
use uxrp_protocol::core::{Error, Result};

#[derive(Clone, Deserialize)]
pub struct UserStoreConfig {
	ddb_endpoint: Option<String>,
	auth_table_name: String,
}

#[derive(Clone)]
pub struct UserStore {
	client: Client,
	auth_table_name: String,
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
		}
	}

	fn auth_item_id(&self, email: &str) -> String {
		format!("auth/email/{}", email)
	}

	pub async fn create(&self, email: &str, password: &str) -> Result<String> {
		let user_id = Uuid::new_v4().to_string();
		let mut password_salt = [0u8; 128];
		rand::thread_rng().fill_bytes(&mut password_salt);

		// TODO: store argon2 config versions and key auth items against them
		let password_hash = argon2::hash_raw(password.as_bytes(), &password_salt, &argon2::Config::default())?;

		self.client
			.put_item()
			.table_name(&self.auth_table_name)
			.item("id", AttributeValue::S(self.auth_item_id(email)))
			.item("user_id", AttributeValue::S(user_id.clone()))
			.item("password_hash", AttributeValue::B(Blob::new(password_hash)))
			.item("password_salt", AttributeValue::B(Blob::new(password_salt)))
			.condition_expression("attribute_not_exists(id)")
			.send()
			.await?;

		Ok(user_id)
	}

	pub async fn verify(&self, email: &str, password: &str) -> Result<String> {
		let res = self
			.client
			.get_item()
			.table_name(&self.auth_table_name)
			.key("id", AttributeValue::S(self.auth_item_id(email)))
			.send()
			.await?;

		let item = res.item.ok_or(Error::InvalidCredentials)?;

		let password_hash = item["password_hash"].as_b().unwrap();
		let password_salt = item["password_salt"].as_b().unwrap();

		if argon2::verify_raw(
			password.as_bytes(),
			password_salt.as_ref(),
			password_hash.as_ref(),
			&argon2::Config::default(),
		)? {
			Ok(item["user_id"].as_s().unwrap().clone())
		} else {
			Err(Error::InvalidCredentials)
		}
	}
}
