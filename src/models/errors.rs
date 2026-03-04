use std::time::SystemTimeError;
use aws_runtime::env_config::error::EnvConfigFileLoadError;
use aws_sdk_secretsmanager::config::http::HttpResponse;
use aws_sdk_secretsmanager::error::SdkError;
use aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueError;
use aws_sdk_secretsmanager::operation::list_secrets::ListSecretsError;
use aws_sdk_ssooidc::operation::register_client::RegisterClientError;
use aws_sdk_ssooidc::operation::start_device_authorization::StartDeviceAuthorizationError;
use aws_sdk_ssooidc::operation::create_token::CreateTokenError;
use aws_sdk_ssooidc::config::http::HttpResponse as SsoHttpResponse;
use aws_sdk_ssooidc::error::SdkError as SsoSdkError;
use openidconnect::{HttpClientError, StandardErrorResponse};
use openidconnect::core::CoreErrorResponseType;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum ArcError {
    #[error("AWS Config Error: {0}")]
    AwsEnvConfigError(#[from] EnvConfigFileLoadError),

    #[error("AWS SDK error: {0}")]
    AwsGetSecretError(#[from] SdkError<GetSecretValueError, HttpResponse>),

    #[error("AWS SDK error: {0}")]
    AwsListSecretError(#[from] SdkError<ListSecretsError, HttpResponse>),

    #[error("AWS Profile Error: {0}")]
    AwsProfileError(String),

    #[error("AWS SSO: {0}")]
    AwsSsoError(String),

    #[error("SSO session expired, please run 'models sso login'")]
    AwsSsoExpired,

    #[error("SSO Register Error: {0}")]
    SsoRegisterError(#[from] SsoSdkError<RegisterClientError, SsoHttpResponse>),

    #[error("SSO Start Device Authorization Error: {0}")]
    SsoStartDeviceAuthError(#[from] SsoSdkError<StartDeviceAuthorizationError, SsoHttpResponse>),

    #[error("SSO Create Token Error: {0}")]
    SsoCreateTokenError(#[from] SsoSdkError<CreateTokenError, SsoHttpResponse>),

    #[error("Chrono parse error: {0}")]
    ChronoParseError(#[from] chrono::ParseError),

    #[error("Error: {0}")]
    Error(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Could not determine home directory")]
    HomeDirError,

    #[error("HTTP header error: {0}")]
    HttpHeaderError(String),

    #[error("InfluxDB query error: {0}")]
    InfluxQueryError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Missing TaskResult for goal: {0}")]
    InsufficientState(String),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    #[error("Expected: {0}, actual: {1}")]
    InvalidGoalParams(String, String),

    #[error("Secret field missing or not a string: {0}")]
    InvalidSecret(String),

    #[error("Invalid TaskResult for goal: {0}. Expected: {1}, Actual: {2}")]
    InvalidState(String, String, String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Kubernetes Config error: {0}")]
    KubeconfigError(#[from] kube::config::KubeconfigError),

    #[error("Kube Context Error: {0}")]
    KubeContextError(String),

    #[error("Kubernetes error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Unable to find any pods matching service selector: {0}")]
    KubePodError(String),

    #[error("Unable to lookup Kube Service spec: {0}")]
    KubeServiceSpecError(String),

    #[error("Keyring error: {0}")]
    KeyringError(#[from] keyring::Error),

    #[error("OpenIDConnect config error: {0}")]
    OpenIdConnectConfigError(#[from] openidconnect::ConfigurationError),

    #[error("OpenIDConnect discovery error: {0}")]
    OpenIdConnectDiscoveryError(#[from] openidconnect::DiscoveryError<HttpClientError<reqwest::Error>>),

    #[error("OpenIDConnect token error: {0}")]
    OpenIdConnectTokenError(#[from] openidconnect::RequestTokenError<HttpClientError<reqwest::Error>, StandardErrorResponse<CoreErrorResponseType>>),

    #[error("HTTP request error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("System time error: {0}")]
    SystemTimeError(#[from] SystemTimeError),

    #[error("Tokio Join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Unable to parse secret as string: {0}")]
    UnparseableSecret(String),

    #[error("User input error: {0}")]
    UserInputError(String),

    #[error("URL Parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Unable to extract query param: {1}, from URL: {0}")]
    UrlQueryParamError(Url, String),

    #[error("Vault error: {0}")]
    VaultError(#[from] vaultrs::error::ClientError),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

impl ArcError {
    pub fn influx_query_error(msg: impl Into<String>) -> Self {
        ArcError::InfluxQueryError(msg.into())
    }

    pub fn insufficient_state(goal: impl Into<String>) -> Self {
        ArcError::InsufficientState(goal.into())
    }
    
    pub fn invalid_config_error(msg: impl Into<String>) -> Self {
        ArcError::InvalidConfig(msg.into())
    }

    pub fn invalid_goal_params(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        ArcError::InvalidGoalParams(expected.into(), actual.into())
    }
    pub fn invalid_secret(field: impl Into<String>) -> Self {
        ArcError::InvalidSecret(field.into())
    }

    pub fn invalid_state(goal: impl Into<String>, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        ArcError::InvalidState(goal.into(), expected.into(), actual.into())
    }

    pub fn kube_context_error(msg: impl Into<String>) -> Self {
        ArcError::KubeContextError(msg.into())
    }
}