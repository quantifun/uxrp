use uxrp_protocol::core::Result;

#[derive(Clone)]
pub struct UserStore {}

impl UserStore {
	pub fn new() -> Self {
		Self {}
	}

	pub async fn create(&self, email: &str, password: &str) -> Result<String> {
		todo!();
	}

	pub async fn verify(&self, email: &str, password: &str) -> Result<String> {
		todo!()
	}
}
