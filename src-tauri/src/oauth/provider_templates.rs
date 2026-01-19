/// OAuth Provider Templates
///
/// Pre-configured OAuth settings for popular providers that don't support
/// RFC 8414/9728 metadata discovery.
///
/// This allows users to quickly configure OAuth for:
/// - GitHub
/// - Google
/// - Microsoft
/// - GitLab
/// - And other common providers

use serde::{Deserialize, Serialize};

/// OAuth provider template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderTemplate {
    /// Provider ID (e.g., "github", "google")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Provider description
    pub description: String,

    /// Authorization endpoint URL
    pub authorization_endpoint: String,

    /// Token endpoint URL
    pub token_endpoint: String,

    /// Token revocation endpoint (optional)
    pub revocation_endpoint: Option<String>,

    /// Default scopes (space-separated)
    pub default_scopes: String,

    /// Supports PKCE
    pub supports_pkce: bool,

    /// Supports DPoP
    pub supports_dpop: bool,

    /// Documentation URL
    pub docs_url: String,

    /// Registration URL for creating OAuth apps
    pub registration_url: String,
}

/// Get all available OAuth provider templates
pub fn get_provider_templates() -> Vec<OAuthProviderTemplate> {
    vec![
        // GitHub
        OAuthProviderTemplate {
            id: "github".to_string(),
            name: "GitHub".to_string(),
            description: "GitHub OAuth Apps for repository access and user authentication".to_string(),
            authorization_endpoint: "https://github.com/login/oauth/authorize".to_string(),
            token_endpoint: "https://github.com/login/oauth/access_token".to_string(),
            revocation_endpoint: None,
            default_scopes: "user repo".to_string(),
            supports_pkce: false,
            supports_dpop: false,
            docs_url: "https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps".to_string(),
            registration_url: "https://github.com/settings/developers".to_string(),
        },

        // Google
        OAuthProviderTemplate {
            id: "google".to_string(),
            name: "Google".to_string(),
            description: "Google OAuth 2.0 for Google Cloud APIs and services".to_string(),
            authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            revocation_endpoint: Some("https://oauth2.googleapis.com/revoke".to_string()),
            default_scopes: "openid profile email".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://developers.google.com/identity/protocols/oauth2".to_string(),
            registration_url: "https://console.cloud.google.com/apis/credentials".to_string(),
        },

        // Microsoft
        OAuthProviderTemplate {
            id: "microsoft".to_string(),
            name: "Microsoft".to_string(),
            description: "Microsoft Entra ID (Azure AD) OAuth for Microsoft 365 and Azure".to_string(),
            authorization_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
            token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
            revocation_endpoint: None,
            default_scopes: "openid profile email".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow".to_string(),
            registration_url: "https://portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade".to_string(),
        },

        // GitLab
        OAuthProviderTemplate {
            id: "gitlab".to_string(),
            name: "GitLab".to_string(),
            description: "GitLab OAuth for repository access and CI/CD integration".to_string(),
            authorization_endpoint: "https://gitlab.com/oauth/authorize".to_string(),
            token_endpoint: "https://gitlab.com/oauth/token".to_string(),
            revocation_endpoint: Some("https://gitlab.com/oauth/revoke".to_string()),
            default_scopes: "read_user api".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://docs.gitlab.com/ee/api/oauth2.html".to_string(),
            registration_url: "https://gitlab.com/-/user_settings/applications".to_string(),
        },

        // Auth0
        OAuthProviderTemplate {
            id: "auth0".to_string(),
            name: "Auth0".to_string(),
            description: "Auth0 identity platform (requires tenant URL)".to_string(),
            authorization_endpoint: "https://{YOUR_DOMAIN}.auth0.com/authorize".to_string(),
            token_endpoint: "https://{YOUR_DOMAIN}.auth0.com/oauth/token".to_string(),
            revocation_endpoint: Some("https://{YOUR_DOMAIN}.auth0.com/oauth/revoke".to_string()),
            default_scopes: "openid profile email".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://auth0.com/docs/get-started/authentication-and-authorization-flow/authorization-code-flow".to_string(),
            registration_url: "https://manage.auth0.com/".to_string(),
        },

        // Okta
        OAuthProviderTemplate {
            id: "okta".to_string(),
            name: "Okta".to_string(),
            description: "Okta identity and access management (requires org URL)".to_string(),
            authorization_endpoint: "https://{YOUR_DOMAIN}.okta.com/oauth2/v1/authorize".to_string(),
            token_endpoint: "https://{YOUR_DOMAIN}.okta.com/oauth2/v1/token".to_string(),
            revocation_endpoint: Some("https://{YOUR_DOMAIN}.okta.com/oauth2/v1/revoke".to_string()),
            default_scopes: "openid profile email".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://developer.okta.com/docs/guides/implement-grant-type/authcode/main/".to_string(),
            registration_url: "https://developer.okta.com/".to_string(),
        },

        // Keycloak
        OAuthProviderTemplate {
            id: "keycloak".to_string(),
            name: "Keycloak".to_string(),
            description: "Keycloak open-source identity and access management (requires realm)".to_string(),
            authorization_endpoint: "https://{YOUR_DOMAIN}/realms/{REALM}/protocol/openid-connect/auth".to_string(),
            token_endpoint: "https://{YOUR_DOMAIN}/realms/{REALM}/protocol/openid-connect/token".to_string(),
            revocation_endpoint: Some("https://{YOUR_DOMAIN}/realms/{REALM}/protocol/openid-connect/revoke".to_string()),
            default_scopes: "openid profile email".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://www.keycloak.org/docs/latest/securing_apps/#_oidc".to_string(),
            registration_url: "https://www.keycloak.org/".to_string(),
        },

        // Generic OAuth 2.0
        OAuthProviderTemplate {
            id: "generic".to_string(),
            name: "Generic OAuth 2.0".to_string(),
            description: "Manual configuration for any OAuth 2.0 provider".to_string(),
            authorization_endpoint: "".to_string(),
            token_endpoint: "".to_string(),
            revocation_endpoint: None,
            default_scopes: "".to_string(),
            supports_pkce: true,
            supports_dpop: false,
            docs_url: "https://oauth.net/2/".to_string(),
            registration_url: "".to_string(),
        },
    ]
}

/// Get a specific provider template by ID
pub fn get_provider_template(id: &str) -> Option<OAuthProviderTemplate> {
    get_provider_templates().into_iter().find(|t| t.id == id)
}

/// Validate manual OAuth configuration
pub fn validate_manual_config(
    authorization_endpoint: &str,
    token_endpoint: &str,
) -> Result<(), String> {
    // Validate authorization endpoint
    if authorization_endpoint.is_empty() {
        return Err("Authorization endpoint is required".to_string());
    }

    if let Err(e) = url::Url::parse(authorization_endpoint) {
        return Err(format!("Invalid authorization endpoint URL: {}", e));
    }

    // Validate token endpoint
    if token_endpoint.is_empty() {
        return Err("Token endpoint is required".to_string());
    }

    if let Err(e) = url::Url::parse(token_endpoint) {
        return Err(format!("Invalid token endpoint URL: {}", e));
    }

    // Validate HTTPS (except localhost for testing)
    let auth_url = url::Url::parse(authorization_endpoint).unwrap();
    let token_url = url::Url::parse(token_endpoint).unwrap();

    if auth_url.scheme() != "https"
        && auth_url.host_str() != Some("localhost")
        && auth_url.host_str() != Some("127.0.0.1")
    {
        return Err("Authorization endpoint must use HTTPS (except localhost)".to_string());
    }

    if token_url.scheme() != "https"
        && token_url.host_str() != Some("localhost")
        && token_url.host_str() != Some("127.0.0.1")
    {
        return Err("Token endpoint must use HTTPS (except localhost)".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_github_template() {
        let template = get_provider_template("github").unwrap();
        assert_eq!(template.name, "GitHub");
        assert!(template
            .authorization_endpoint
            .contains("github.com/login/oauth"));
    }

    #[test]
    fn test_validate_manual_config() {
        assert!(validate_manual_config(
            "https://example.com/authorize",
            "https://example.com/token"
        )
        .is_ok());

        assert!(validate_manual_config(
            "http://localhost:8080/authorize",
            "http://localhost:8080/token"
        )
        .is_ok());

        assert!(validate_manual_config("", "https://example.com/token").is_err());

        assert!(validate_manual_config(
            "http://example.com/authorize",
            "https://example.com/token"
        )
        .is_err());
    }
}
