//! The login flows a homeserver offers: `GET /_matrix/client/v3/login` (FR-012).

use serde_json::Value;

/// An identity provider of the SSO flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityProvider {
    /// Goes into a URL path: `[A-Za-z0-9._~-]+`, not `.` nor `..`.
    pub id: String,
    /// The name to show.
    pub name: String,
}

/// The `m.login.sso` flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsoFlow {
    /// In the order of the server; may be empty.
    pub identity_providers: Vec<IdentityProvider>,
}

/// The ways to log in that this app supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFlows {
    /// SSO, when offered.
    pub sso: Option<SsoFlow>,
    /// Whether `m.login.password` is offered.
    pub password: bool,
}

/// Why an answer of `/login` is not usable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoginFlowsError {
    /// The body is not JSON.
    #[error("the login answer is not JSON")]
    NotJson,
    /// The body has no list of flows.
    #[error("the login answer has no list of flows")]
    Malformed,
    /// No SSO and no password among the flows.
    #[error("the server offers no login this app supports")]
    NoSupportedLogin,
}

fn has_type(flow: &Value, kind: &str) -> bool {
    flow.get("type").and_then(Value::as_str) == Some(kind)
}

fn is_url_safe(id: &str) -> bool {
    !matches!(id, "" | "." | "..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '~' | '-'))
}

fn provider(value: &Value) -> Option<IdentityProvider> {
    let id = value.get("id")?.as_str()?;
    let name = value.get("name")?.as_str()?;
    is_url_safe(id).then(|| IdentityProvider {
        id: id.to_owned(),
        name: name.to_owned(),
    })
}

fn sso_flow(flow: &Value) -> SsoFlow {
    let providers = flow.get("identity_providers").and_then(Value::as_array);
    SsoFlow {
        identity_providers: providers
            .into_iter()
            .flatten()
            .filter_map(provider)
            .collect(),
    }
}

impl LoginFlows {
    /// Reads the body of `GET /_matrix/client/v3/login`; other flows are ignored.
    pub fn from_json(body: &str) -> Result<Self, LoginFlowsError> {
        let json: Value = serde_json::from_str(body).map_err(|_| LoginFlowsError::NotJson)?;
        let flows = json
            .get("flows")
            .and_then(Value::as_array)
            .ok_or(LoginFlowsError::Malformed)?;
        let flows = Self {
            sso: flows
                .iter()
                .find(|f| has_type(f, "m.login.sso"))
                .map(sso_flow),
            password: flows.iter().any(|f| has_type(f, "m.login.password")),
        };
        if flows.sso.is_none() && !flows.password {
            return Err(LoginFlowsError::NoSupportedLogin);
        }
        Ok(flows)
    }
}
