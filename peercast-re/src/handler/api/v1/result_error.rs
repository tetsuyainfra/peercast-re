#![allow(unused)]
use http::StatusCode;
use serde::Serialize;

// RFC 9457 Standard Error Response Format を参考にしています
// https://www.rfc-editor.org/rfc/rfc9457.html
// https://tex2e.github.io/rfc-translater/html/rfc9457.html

#[derive(Serialize)]
struct SuccessResponse<S> {
    result: S,
}

// エラーレスポンス
#[derive(Serialize)]
struct ErrorResponse {
    /// エラーの種類を示すURI
    #[serde(rename = "type")]
    type_: ErrorType,

    /// 人間が理解できる簡潔なエラーのタイトル
    title: String,

    /// クライアントが問題を修正できるのに役立つ詳細なエラー説明
    detail: String,

    /// エラーが発生したオリジンサーバのHTTPステータスコード
    status: u16,
    // / エラーのインスタンスを一意に識別するためのURI
    // instance: String,
}

#[derive(Serialize)]
pub enum ErrorType {
    ValidationError,
    NotFound,
    InternalServerError,
}

// エラーレスポンスを生成するヘルパー関数
pub fn create_error(
    status: StatusCode,
    type_: ErrorType,
    title: &str,
    detail: &str,
) -> impl axum::response::IntoResponse {
    let error_response = ErrorResponse {
        type_,
        title: title.into(),
        detail: detail.into(),
        status: status.as_u16(),
        // instance: todo!(),
    };
    (status, axum::Json(error_response))
}

// // 使用例
// async fn handler() -> impl IntoResponse {
//     // 例: 何か問題があった場合
//     let error_message = "Invalid parameter";
//     create_error(StatusCode::BAD_REQUEST, error_message)
// }

#[cfg(test)]
mod tests {
    use std::usize;

    use super::*;
    use axum::{body::Body, response::IntoResponse};
    use http::{Response, StatusCode};
    use serde_json::json;

    #[tokio::test]
    async fn test_create_error() {
        let status = StatusCode::BAD_REQUEST;
        let type_ = ErrorType::ValidationError;
        let title = "Invalid Request";
        let detail = "The request parameters are invalid.";

        let response = create_error(status, type_, title, detail).into_response();
        assert_eq!(response.status(), status);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json_body: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let expected_body = json!({
            "type": "ValidationError",
            "title": title,
            "detail": detail,
            "status": status.as_u16(),
        });

        assert_eq!(json_body, expected_body);
        println!("Error Response JSON: {}", json_body);
    }
}
