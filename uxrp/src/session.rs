use redis::{AsyncCommands, Client};
use uuid::Uuid;
use uxrp_protocol::actix_web::HttpRequest;
use uxrp_protocol::async_trait::async_trait;
use uxrp_protocol::core::{Error, HttpPrincipalResolver, Result, UserPrincipal};

#[derive(Debug, Clone)]
pub struct RedisSessionStore {
	redis: Client,
}

impl RedisSessionStore {
	pub async fn new(connstring: &str) -> Result<Self> {
		let client = Client::open(connstring)?;
		Ok(RedisSessionStore { redis: client })
	}

	fn session_key(&self, token: &str) -> String {
		format!("sessions:{}", token)
	}

	pub async fn create(&self, user: UserPrincipal) -> Result<String> {
		let token = Uuid::new_v4().to_string();
		let mut conn = self.redis.get_async_connection().await?;
		conn.set(self.session_key(&token), serde_json::to_string(&user)?)
			.await?;
		Ok(token)
	}
}

#[async_trait(?Send)]
impl HttpPrincipalResolver<UserPrincipal> for RedisSessionStore {
	async fn resolve(&self, req: HttpRequest) -> Result<UserPrincipal> {
		let token = req
			.headers()
			.get("Authorization")
			.and_then(|h| h.to_str().ok())
			.and_then(|t| t.strip_prefix("Bearer "))
			.ok_or(Error::InvalidCredentials)?;

		let mut conn = self.redis.get_async_connection().await?;
		let data: Option<String> = conn.get(self.session_key(token)).await?;

		match data {
			Some(s) => Ok(serde_json::from_str(&s)?),
			None => Err(Error::InvalidCredentials),
		}
	}
}
