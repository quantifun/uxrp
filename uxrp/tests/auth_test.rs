use actix_test::{start, TestServer};
use serde::{de::DeserializeOwned, Serialize};
use uxrp::server::get_app_config;
use uxrp_protocol::actix_web::{http::StatusCode, App};
use uxrp_protocol::auth::*;

async fn make_server() -> TestServer {
	let app_config = get_app_config().await;
	start(move || App::new().configure(app_config.clone()))
}

async fn call<Req: Serialize, Res: DeserializeOwned>(
	srv: &TestServer,
	token: Option<String>,
	path: &str,
	req: Req,
) -> Res {
	let mut client_req = srv.post(path);
	if let Some(token) = token {
		client_req = client_req.bearer_auth(token);
	}

	let mut res = client_req.send_json(&req).await.expect("sending request failed");

	match res.status() {
		StatusCode::OK => res.json::<Res>().await.expect("failed to deserialise response"),
		status => {
			let body = res.body().await.expect("failed to retrieve body");
			panic!(
				"request to {} failed with status {}: {}",
				path,
				status,
				String::from_utf8(body.to_vec()).expect("failed to decode body")
			)
		}
	}
}

#[actix_rt::test]
async fn registration_flow() {
	let srv = make_server().await;

	let email = "test@test.com".to_owned();
	let password = "supersecure".to_owned();

	let _: RegisterResponse = call(
		&srv,
		None,
		"/auth/register",
		RegisterRequest {
			email: email.clone(),
			password: password.clone(),
		},
	)
	.await;

	let login_res: LoginResponse = call(
		&srv,
		None,
		"/auth/login",
		LoginRequest {
			email: email.clone(),
			password: password.clone(),
		},
	)
	.await;

	let test_res: TestResponse = call(&srv, Some(login_res.token), "/auth/test", TestRequest {}).await;
	assert_eq!(test_res.principal_id.len(), 36);
}
