use redis::{AsyncCommands, Client, RedisError};
use uuid::Uuid;
use uxrp_protocol::actix_web::{HttpRequest, HttpResponse};
use uxrp_protocol::async_trait::async_trait;
use uxrp_protocol::core::{HttpPrincipalResolver, UserPrincipal};

#[derive(thiserror::Error, Debug)]
pub enum SessionError {
	#[error("redis error: {0}")]
	Redis(#[from] RedisError),
	#[error("serde error: {0}")]
	Serde(#[from] serde_json::Error),
}

pub struct UserPrincipalResolver {
	redis: Client,
}

impl UserPrincipalResolver {
	pub async fn new(connstring: &str) -> Result<Self, SessionError> {
		let client = Client::open(connstring)?;
		Ok(UserPrincipalResolver { redis: client })
	}

	fn session_key(&self, token: &str) -> String {
		format!("sessions:{}", token)
	}

	pub async fn create_session(&self, user: UserPrincipal) -> Result<String, SessionError> {
		let token = Uuid::new_v4().to_string();
		let mut conn = self.redis.get_async_connection().await?;
		conn.set(self.session_key(&token), serde_json::to_string(&user)?)
			.await?;
		Ok(token)
	}
}

#[async_trait(?Send)]
impl HttpPrincipalResolver<UserPrincipal> for UserPrincipalResolver {
	async fn resolve(&self, req: HttpRequest) -> Result<UserPrincipal, HttpResponse> {
		let token = req
			.headers()
			.get("Authorization")
			.and_then(|h| h.to_str().ok())
			.and_then(|t| t.strip_prefix("Bearer "))
			.ok_or_else(|| HttpResponse::Unauthorized().finish())?;

		let mut conn = self
			.redis
			.get_async_connection()
			.await
			.expect("failed to retrieve redis conn");

		let data: Option<String> = conn
			.get(self.session_key(token))
			.await
			.expect("failed to query session");

		match data {
			Some(s) => Ok(serde_json::from_str(&s).expect("invalid json data")),
			None => Err(HttpResponse::Unauthorized().finish()),
		}
	}
}
