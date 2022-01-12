use aws_sdk_dynamodb::{model::AttributeValue, Endpoint};
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
	#[serde(default)]
	skip_email_verification: bool,
}

#[derive(Clone)]
pub struct UserStore {
	ddb: aws_sdk_dynamodb::Client,
	ses: aws_sdk_ses::Client,
	auth_table_name: String,
	id_prefix: String,
	skip_email_verification: bool,
}

#[derive(Serialize, Deserialize)]
struct UserCredentials {
	id: String,
	user_id: String,
	password_hash: Vec<u8>,
	password_salt: Vec<u8>,
	email_verified: bool,
}

#[derive(Serialize, Deserialize)]
struct Verification {
	id: String,
	email: String,
}

impl UserStore {
	pub async fn new(config: UserStoreConfig) -> Self {
		let aws_config = aws_config::load_from_env().await;
		let mut ddb_config = aws_sdk_dynamodb::config::Builder::from(&aws_config);
		if let Some(ddb_endpoint) = config.ddb_endpoint {
			ddb_config =
				ddb_config.endpoint_resolver(Endpoint::immutable(ddb_endpoint.parse().expect("invalid ddb url")));
		}

		let client = aws_sdk_dynamodb::Client::from_conf(ddb_config.build());
		Self {
			ddb: client,
			ses: aws_sdk_ses::Client::new(&aws_config),
			auth_table_name: config.auth_table_name,
			id_prefix: if config.randomise_id_prefix {
				format!("{}/", Uuid::new_v4())
			} else {
				"".to_owned()
			},
			skip_email_verification: config.skip_email_verification,
		}
	}

	fn creds_item_id(&self, email: &str) -> String {
		format!("{}auth/email/{}", self.id_prefix, email)
	}

	fn verification_item_id(&self, token: &str) -> String {
		format!("{}verification/{}", self.id_prefix, token)
	}

	pub async fn create(&self, email: &str, password: &str) -> Result<String> {
		if !self.skip_email_verification {
			let verification_token = Uuid::new_v4().to_string();
			self.ddb
				.put_item()
				.table_name(&self.auth_table_name)
				.set_item(Some(serde_dynamo::to_item(Verification {
					id: self.verification_item_id(&verification_token),
					email: email.to_string(),
				})?))
				.send()
				.await?;

			// TODO: actually build this request
			self.ses.send_email().send().await?;
		}

		let user_id = Uuid::new_v4().to_string();
		let mut password_salt = Vec::new();
		password_salt.resize(128, 0);
		rand::thread_rng().fill_bytes(&mut password_salt);

		// TODO: store argon2 config versions and key auth items against them
		let password_hash = argon2::hash_raw(password.as_bytes(), &password_salt, &argon2::Config::default())?;

		self.ddb
			.put_item()
			.table_name(&self.auth_table_name)
			.set_item(Some(serde_dynamo::to_item(UserCredentials {
				id: self.creds_item_id(email),
				user_id: user_id.clone(),
				password_hash,
				password_salt,
				email_verified: self.skip_email_verification,
			})?))
			.condition_expression("attribute_not_exists(id)")
			.send()
			.await?;

		Ok(user_id)
	}

	pub async fn verify(&self, token: &str) -> Result<()> {
		let res = self
			.ddb
			.get_item()
			.table_name(&self.auth_table_name)
			.key("id", AttributeValue::S(self.verification_item_id(token)))
			.send()
			.await?;

		let verification: Verification = serde_dynamo::from_item(res.item.ok_or(Error::InvalidCredentials)?)?;

		self.ddb
			.update_item()
			.key("id", AttributeValue::S(self.creds_item_id(&verification.email)))
			.update_expression("SET verified = :verified")
			.expression_attribute_values(":verified", AttributeValue::Bool(true))
			.send()
			.await?;

		Ok(())
	}

	pub async fn authenticate(&self, email: &str, password: &str) -> Result<String> {
		let res = self
			.ddb
			.get_item()
			.table_name(&self.auth_table_name)
			.key("id", AttributeValue::S(self.creds_item_id(email)))
			.send()
			.await?;

		let creds: UserCredentials = serde_dynamo::from_item(res.item.ok_or(Error::InvalidCredentials)?)?;

		if !creds.email_verified {
			return Err(Error::UserUnverified);
		}

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
