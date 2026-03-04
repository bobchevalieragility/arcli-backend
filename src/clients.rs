use std::io::Cursor;
use tiny_http::Response;
use url::Url;
use crate::models::errors::ArcError;

pub mod argo_client;
pub mod vault_client;

fn extract_query_param(url: &Url, key: &str) -> Result<String, ArcError> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| ArcError::UrlQueryParamError(url.clone(), key.to_string()))
}

pub(crate) fn auth_success_response(auth_type: &str) -> Result<Response<Cursor<Vec<u8>>>, ArcError> {
    // Embed the Digit robot drumming image from assets as base64
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let image_bytes = include_bytes!("../assets/digit_drumming.jpg");
    let image_base64 = STANDARD.encode(image_bytes);

    let html = format!(r#"
    <html>
    <head>
        <style>
            body {{
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                display: flex;
                flex-direction: column;
                align-items: center;
                justify-content: center;
                min-height: 100vh;
                margin: 0;
                background: url('data:image/jpeg;base64,{}') center/cover no-repeat;
                color: white;
            }}
            .container {{
                text-align: center;
                background: linear-gradient(135deg, rgba(20, 184, 166, 0.9) 0%, rgba(13, 148, 136, 0.9) 100%);
                padding: 3rem;
                border-radius: 20px;
                backdrop-filter: blur(10px);
                box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.37);
            }}
            h1 {{
                margin: 0 0 1rem 0;
                font-size: 2.5rem;
            }}
            p {{
                font-size: 1.2rem;
                margin: 1rem 0;
            }}
        </style>
    </head>
    <body>
        <div class="container">
            <h1>{} Authentication Successful</h1>
            <h2>You're ready to rock!</h2>
            <p>You can close this tab and return to the terminal.</p>
        </div>
    </body>
    </html>
    "#, image_base64, auth_type);

    let response = Response::from_string(html)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
                .map_err(|_| ArcError::HttpHeaderError("Failed to create Content-Type header".to_string()))?
        );
    Ok(response)
}
