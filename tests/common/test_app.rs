use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;

pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        let request = Request::builder()
            .uri(path)
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = self.router.clone().oneshot(request).await.unwrap();
        TestResponse::new(response).await
    }

    pub async fn post(&self, path: &str, body: Body) -> TestResponse {
        let request = Request::builder()
            .uri(path)
            .method("POST")
            .header("content-type", "application/json")
            .body(body)
            .unwrap();

        let response = self.router.clone().oneshot(request).await.unwrap();
        TestResponse::new(response).await
    }

    pub async fn post_multipart(&self, path: &str, body: Body, content_type: &str) -> TestResponse {
        let request = Request::builder()
            .uri(path)
            .method("POST")
            .header("content-type", content_type)
            .body(body)
            .unwrap();

        let response = self.router.clone().oneshot(request).await.unwrap();
        TestResponse::new(response).await
    }

    pub async fn put(&self, path: &str, body: Body) -> TestResponse {
        let request = Request::builder()
            .uri(path)
            .method("PUT")
            .header("content-type", "application/json")
            .body(body)
            .unwrap();

        let response = self.router.clone().oneshot(request).await.unwrap();
        TestResponse::new(response).await
    }

    pub async fn delete(&self, path: &str) -> TestResponse {
        let request = Request::builder()
            .uri(path)
            .method("DELETE")
            .body(Body::empty())
            .unwrap();

        let response = self.router.clone().oneshot(request).await.unwrap();
        TestResponse::new(response).await
    }
}

pub struct TestResponse {
    pub status: StatusCode,
    pub body: String,
}

impl TestResponse {
    async fn new(response: axum::response::Response) -> Self {
        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();

        Self { status, body }
    }

    pub fn assert_status(&self, expected: StatusCode) {
        assert_eq!(self.status, expected, "Response body: {}", self.body);
    }

    pub fn assert_json<T>(&self, expected: &T)
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        let expected_json = serde_json::to_string(expected).unwrap();
        assert_eq!(self.body, expected_json);
    }

    pub fn json<T>(&self) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str(&self.body).unwrap()
    }
}
