//! Oauth2 RFC protocol definitions.

use std::collections::{BTreeMap, BTreeSet};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::SpaceSeparator;
use serde_with::{
    formats, rust::deserialize_ignore_any, serde_as, skip_serializing_none, StringWithSeparator,
};
use url::Url;
use uuid::Uuid;

/// How many seconds a device code is valid for.
pub const OAUTH2_DEVICE_CODE_EXPIRY_SECONDS: u64 = 300;
/// How often a client device can query the status of the token
pub const OAUTH2_DEVICE_CODE_INTERVAL_SECONDS: u64 = 5;
/// Token type URI for OAuth2 access tokens as per RFC8693.
pub const OAUTH2_TOKEN_TYPE_ACCESS_TOKEN: &str = "urn:ietf:params:oauth:token-type:access_token";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CodeChallengeMethod {
    // default to plain if not requested as S256. Reject the auth?
    // plain
    // BASE64URL-ENCODE(SHA256(ASCII(code_verifier)))
    S256,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PkceRequest {
    #[serde_as(as = "Base64<UrlSafe, formats::Unpadded>")]
    pub code_challenge: Vec<u8>,
    pub code_challenge_method: CodeChallengeMethod,
}

/// An OAuth2 client redirects to the authorisation server with Authorisation Request
/// parameters.
#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthorisationRequest {
    // Must be "code". (or token, see 4.2.1)
    pub response_type: ResponseType,
    /// Response mode.
    ///
    /// Optional; defaults to `query` for `response_type=code` (Auth Code), and
    /// `fragment` for `response_type=token` (Implicit Grant, which we probably
    /// won't support).
    ///
    /// Reference:
    /// [OAuth 2.0 Multiple Response Type Encoding Practices: Response Modes](https://openid.net/specs/oauth-v2-multiple-response-types-1_0.html#ResponseModes)
    pub response_mode: Option<ResponseMode>,
    pub client_id: String,
    pub state: Option<String>,
    #[serde(flatten)]
    pub pkce_request: Option<PkceRequest>,
    pub redirect_uri: Url,
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scope: BTreeSet<String>,
    // OIDC adds a nonce parameter that is optional.
    pub nonce: Option<String>,
    // OIDC also allows other optional params
    #[serde(flatten)]
    pub oidc_ext: AuthorisationRequestOidc,
    // Needs to be hoisted here due to serde flatten bug #3185
    pub max_age: Option<i64>,
    #[serde(flatten)]
    pub unknown_keys: BTreeMap<String, serde_json::value::Value>,
}

impl AuthorisationRequest {
    /// Get the `response_mode` appropriate for this request, taking into
    /// account defaults from the `response_type` parameter.
    ///
    /// Returns `None` if the selection is invalid.
    ///
    /// Reference:
    /// [OAuth 2.0 Multiple Response Type Encoding Practices: Response Modes](https://openid.net/specs/oauth-v2-multiple-response-types-1_0.html#ResponseModes)
    pub const fn get_response_mode(&self) -> Option<ResponseMode> {
        match (self.response_mode, self.response_type) {
            // https://openid.net/specs/oauth-v2-multiple-response-types-1_0.html#id_token
            // The default Response Mode for this Response Type is the fragment
            // encoding and the query encoding MUST NOT be used.
            (None, ResponseType::IdToken) => Some(ResponseMode::Fragment),
            (Some(ResponseMode::Query), ResponseType::IdToken) => None,

            // https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.2
            (None, ResponseType::Code) => Some(ResponseMode::Query),
            // https://datatracker.ietf.org/doc/html/rfc6749#section-4.2.2
            (None, ResponseType::Token) => Some(ResponseMode::Fragment),

            // https://openid.net/specs/oauth-v2-multiple-response-types-1_0.html#Security
            // In no case should a set of Authorization Response parameters
            // whose default Response Mode is the fragment encoding be encoded
            // using the query encoding.
            (Some(ResponseMode::Query), ResponseType::Token) => None,

            // Allow others.
            (Some(m), _) => Some(m),
        }
    }
}

/// An OIDC client redirects to the authorisation server with Authorisation Request
/// parameters.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AuthorisationRequestOidc {
    pub display: Option<String>,
    pub prompt: Option<String>,
    pub ui_locales: Option<()>,
    pub claims_locales: Option<()>,
    pub id_token_hint: Option<String>,
    pub login_hint: Option<String>,
    pub acr: Option<String>,
}

/// In response to an Authorisation request, the user may be prompted to consent to the
/// scopes requested by the OAuth2 client. If they have previously consented, they will
/// immediately proceed.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthorisationResponse {
    ConsentRequested {
        // A pretty-name of the client
        client_name: String,
        // A list of scopes requested / to be issued.
        scopes: BTreeSet<String>,
        // Extra PII that may be requested
        pii_scopes: BTreeSet<String>,
        // The users displayname (?)
        // pub display_name: String,
        // The token we need to be given back to allow this to proceed
        consent_token: String,
    },
    Permitted,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "grant_type", rename_all = "snake_case")]
pub enum GrantTypeReq {
    AuthorizationCode {
        // As sent by the authorisationCode
        code: String,
        // Must be the same as the original redirect uri.
        redirect_uri: Url,
        code_verifier: Option<String>,
    },
    ClientCredentials {
        #[serde_as(as = "Option<StringWithSeparator::<SpaceSeparator, String>>")]
        scope: Option<BTreeSet<String>>,
    },
    RefreshToken {
        refresh_token: String,
        #[serde_as(as = "Option<StringWithSeparator::<SpaceSeparator, String>>")]
        scope: Option<BTreeSet<String>>,
    },
    #[serde(rename = "urn:ietf:params:oauth:grant-type:token-exchange")]
    TokenExchange {
        subject_token: String,
        subject_token_type: String,
        requested_token_type: Option<String>,
        audience: Option<String>,
        resource: Option<String>,
        actor_token: Option<String>,
        actor_token_type: Option<String>,
        #[serde_as(as = "Option<StringWithSeparator::<SpaceSeparator, String>>")]
        scope: Option<BTreeSet<String>>,
    },
    /// ref <https://www.rfc-editor.org/rfc/rfc8628#section-3.4>
    #[serde(rename = "urn:ietf:params:oauth:grant-type:device_code")]
    DeviceCode {
        device_code: String,
        // #[serde_as(as = "Option<StringWithSeparator::<SpaceSeparator, String>>")]
        scope: Option<BTreeSet<String>>,
    },
}

/// An Access Token request. This requires a set of grant-type parameters to satisfy the request.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessTokenRequest {
    #[serde(flatten)]
    pub grant_type: GrantTypeReq,
    // REQUIRED, if the client is not authenticating with the
    //  authorization server as described in Section 3.2.1.
    #[serde(flatten)]
    pub client_post_auth: ClientPostAuth,
}

impl From<GrantTypeReq> for AccessTokenRequest {
    fn from(req: GrantTypeReq) -> AccessTokenRequest {
        AccessTokenRequest {
            grant_type: req,
            client_post_auth: ClientPostAuth::default(),
        }
    }
}

#[derive(Serialize, Debug, Clone, Deserialize)]
#[skip_serializing_none]
pub struct OAuth2RFC9068Token<V>
where
    V: Clone,
{
    /// The issuer of this token
    pub iss: String,
    /// Unique id of the subject
    pub sub: Uuid,
    /// client_id of the oauth2 rp
    pub aud: String,
    /// Expiry in UTC epoch seconds
    pub exp: i64,
    /// Not valid before.
    pub nbf: i64,
    /// Issued at time.
    pub iat: i64,
    /// JWT ID <https://www.rfc-editor.org/rfc/rfc7519#section-4.1.7> - we set it to the session ID
    pub jti: Uuid,
    pub client_id: String,
    #[serde(flatten)]
    pub extensions: V,
}

/// Extensions for RFC 9068 Access Token
#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OAuth2RFC9068TokenExtensions {
    pub auth_time: Option<i64>,
    pub acr: Option<String>,
    pub amr: Option<Vec<String>>,

    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scope: BTreeSet<String>,

    pub nonce: Option<String>,

    pub session_id: Uuid,
    pub parent_session_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum IssuedTokenType {
    AccessToken,
    RefreshToken,
    IdToken,
    Saml1,
    Saml2,
}

/// The response for an access token
#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: AccessTokenType,
    /// Optional RFC8693 issued_token_type.
    pub issued_token_type: Option<IssuedTokenType>,
    /// Expiration relative to `now` in seconds.
    pub expires_in: u32,
    pub refresh_token: Option<String>,
    /// Space separated list of scopes that were approved, if this differs from the
    /// original request.
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scope: BTreeSet<String>,
    /// If the `openid` scope was requested, an `id_token` may be present in the response.
    pub id_token: Option<String>,
}

/// Access token types, per [IANA Registry - OAuth Access Token Types](https://www.iana.org/assignments/oauth-parameters/oauth-parameters.xhtml#token-types)
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(try_from = "&str")]
pub enum AccessTokenType {
    Bearer,
    PoP,
    #[serde(rename = "N_A")]
    NA,
    DPoP,
}

impl TryFrom<&str> for AccessTokenType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "bearer" => Ok(AccessTokenType::Bearer),
            "pop" => Ok(AccessTokenType::PoP),
            "n_a" => Ok(AccessTokenType::NA),
            "dpop" => Ok(AccessTokenType::DPoP),
            _ => Err(format!("Unknown AccessTokenType: {s}")),
        }
    }
}

/// Request revocation of an Access or Refresh token. On success the response is OK 200
/// with no body.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct TokenRevokeRequest {
    pub token: String,
    /// Not required for Kanidm.
    /// <https://datatracker.ietf.org/doc/html/rfc7009#section-4.1.2>
    pub token_type_hint: Option<String>,

    #[serde(flatten)]
    pub client_post_auth: ClientPostAuth,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
/// <https://datatracker.ietf.org/doc/html/rfc6749#section-2.3.1>
pub struct ClientPostAuth {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl From<(String, Option<String>)> for ClientPostAuth {
    fn from((client_id, client_secret): (String, Option<String>)) -> Self {
        ClientPostAuth {
            client_id: Some(client_id),
            client_secret,
        }
    }
}

impl From<(&str, Option<&str>)> for ClientPostAuth {
    fn from((client_id, client_secret): (&str, Option<&str>)) -> Self {
        ClientPostAuth {
            client_id: Some(client_id.to_string()),
            client_secret: client_secret.map(|s| s.to_string()),
        }
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
/// <https://datatracker.ietf.org/doc/html/rfc6749#section-2.3.1>
pub struct ClientAuth {
    pub client_id: String,
    pub client_secret: Option<String>,
}

impl From<(&str, Option<&str>)> for ClientAuth {
    fn from((client_id, client_secret): (&str, Option<&str>)) -> Self {
        ClientAuth {
            client_id: client_id.to_string(),
            client_secret: client_secret.map(|s| s.to_string()),
        }
    }
}

/// Request to introspect the identity of the account associated to a token.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessTokenIntrospectRequest {
    pub token: String,
    /// Not required for Kanidm.
    /// <https://datatracker.ietf.org/doc/html/rfc7009#section-4.1.2>
    pub token_type_hint: Option<String>,

    // For when they want to use POST auth
    // https://datatracker.ietf.org/doc/html/rfc6749#section-2.3.1
    #[serde(flatten)]
    pub client_post_auth: ClientPostAuth,
}

/// Response to an introspection request. If the token is inactive or revoked, only
/// `active` will be set to the value of `false`.
#[serde_as]
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessTokenIntrospectResponse {
    pub active: bool,
    #[serde_as(as = "StringWithSeparator::<SpaceSeparator, String>")]
    pub scope: BTreeSet<String>,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub token_type: Option<AccessTokenType>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub nbf: Option<i64>,
    pub sub: Option<String>,
    pub aud: Option<String>,
    pub iss: Option<String>,
    // JWT ID <https://www.rfc-editor.org/rfc/rfc7519#section-4.1.7> set to session ID
    pub jti: Uuid,
}

impl AccessTokenIntrospectResponse {
    pub fn inactive(session_id: Uuid) -> Self {
        AccessTokenIntrospectResponse {
            active: false,
            scope: BTreeSet::default(),
            client_id: None,
            username: None,
            token_type: None,
            exp: None,
            iat: None,
            nbf: None,
            sub: None,
            aud: None,
            iss: None,
            jti: session_id,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    // Auth Code flow
    // https://datatracker.ietf.org/doc/html/rfc6749#section-4.1.1
    Code,
    // Implicit Grant flow
    // https://datatracker.ietf.org/doc/html/rfc6749#section-4.2.1
    Token,
    // https://openid.net/specs/oauth-v2-multiple-response-types-1_0.html#id_token
    IdToken,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    Query,
    Fragment,
    FormPost,
    #[serde(other, deserialize_with = "deserialize_ignore_any")]
    Invalid,
}

fn response_modes_supported_default() -> Vec<ResponseMode> {
    vec![ResponseMode::Query, ResponseMode::Fragment]
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    #[serde(rename = "authorization_code")]
    AuthorisationCode,
    Implicit,
    #[serde(rename = "urn:ietf:params:oauth:grant-type:token-exchange")]
    TokenExchange,
}

fn grant_types_supported_default() -> Vec<GrantType> {
    vec![
        GrantType::AuthorisationCode,
        GrantType::Implicit,
        GrantType::TokenExchange,
    ]
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubjectType {
    Pairwise,
    Public,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum PkceAlg {
    S256,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum IdTokenSignAlg {
    ES256,
    RS256,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenEndpointAuthMethod {
    ClientSecretPost,
    ClientSecretBasic,
    ClientSecretJwt,
    PrivateKeyJwt,
}

fn token_endpoint_auth_methods_supported_default() -> Vec<TokenEndpointAuthMethod> {
    vec![TokenEndpointAuthMethod::ClientSecretBasic]
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayValue {
    Page,
    Popup,
    Touch,
    Wap,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    Normal,
    Aggregated,
    Distributed,
}

fn claim_types_supported_default() -> Vec<ClaimType> {
    vec![ClaimType::Normal]
}

fn claims_parameter_supported_default() -> bool {
    false
}

fn request_parameter_supported_default() -> bool {
    false
}

fn request_uri_parameter_supported_default() -> bool {
    false
}

fn require_request_uri_parameter_supported_default() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OidcWebfingerRel {
    pub rel: String,
    pub href: String,
}

/// The response to an Webfinger request. Only a subset of the body is defined here.
/// <https://datatracker.ietf.org/doc/html/rfc7033#section-4.4>
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct OidcWebfingerResponse {
    pub subject: String,
    pub links: Vec<OidcWebfingerRel>,
}

/// The response to an OpenID connect discovery request
/// <https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderMetadata>
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OidcDiscoveryResponse {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
    pub userinfo_endpoint: Option<Url>,
    pub jwks_uri: Url,
    pub registration_endpoint: Option<Url>,
    pub scopes_supported: Option<Vec<String>>,
    // https://datatracker.ietf.org/doc/html/rfc6749#section-3.1.1
    pub response_types_supported: Vec<ResponseType>,
    // https://openid.net/specs/oauth-v2-multiple-response-types-1_0.html#ResponseModes
    #[serde(default = "response_modes_supported_default")]
    pub response_modes_supported: Vec<ResponseMode>,
    // Need to fill in as authorization_code only else a default is assumed.
    #[serde(default = "grant_types_supported_default")]
    pub grant_types_supported: Vec<GrantType>,
    pub acr_values_supported: Option<Vec<String>>,
    // https://openid.net/specs/openid-connect-core-1_0.html#PairwiseAlg
    pub subject_types_supported: Vec<SubjectType>,
    pub id_token_signing_alg_values_supported: Vec<IdTokenSignAlg>,
    pub id_token_encryption_alg_values_supported: Option<Vec<String>>,
    pub id_token_encryption_enc_values_supported: Option<Vec<String>>,
    pub userinfo_signing_alg_values_supported: Option<Vec<String>>,
    pub userinfo_encryption_alg_values_supported: Option<Vec<String>>,
    pub userinfo_encryption_enc_values_supported: Option<Vec<String>>,
    pub request_object_signing_alg_values_supported: Option<Vec<String>>,
    pub request_object_encryption_alg_values_supported: Option<Vec<String>>,
    pub request_object_encryption_enc_values_supported: Option<Vec<String>>,
    // Defaults to client_secret_basic
    #[serde(default = "token_endpoint_auth_methods_supported_default")]
    pub token_endpoint_auth_methods_supported: Vec<TokenEndpointAuthMethod>,
    pub token_endpoint_auth_signing_alg_values_supported: Option<Vec<String>>,
    // https://openid.net/specs/openid-connect-core-1_0.html#AuthRequest
    pub display_values_supported: Option<Vec<DisplayValue>>,
    // Default to normal.
    #[serde(default = "claim_types_supported_default")]
    pub claim_types_supported: Vec<ClaimType>,
    pub claims_supported: Option<Vec<String>>,
    pub service_documentation: Option<Url>,
    pub claims_locales_supported: Option<Vec<String>>,
    pub ui_locales_supported: Option<Vec<String>>,
    // Default false.
    #[serde(default = "claims_parameter_supported_default")]
    pub claims_parameter_supported: bool,

    pub op_policy_uri: Option<Url>,
    pub op_tos_uri: Option<Url>,

    // these are related to RFC9101 JWT-Secured Authorization Request support
    #[serde(default = "request_parameter_supported_default")]
    pub request_parameter_supported: bool,
    #[serde(default = "request_uri_parameter_supported_default")]
    pub request_uri_parameter_supported: bool,
    #[serde(default = "require_request_uri_parameter_supported_default")]
    pub require_request_uri_registration: bool,

    pub code_challenge_methods_supported: Vec<PkceAlg>,

    // https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderConfigurationResponse
    // "content type that contains a set of Claims as its members that are a subset of the Metadata
    //  values defined in Section 3. Other Claims MAY also be returned. "
    //
    // In addition, we also return the following claims in kanidm

    // rfc7009
    pub revocation_endpoint: Option<Url>,
    pub revocation_endpoint_auth_methods_supported: Vec<TokenEndpointAuthMethod>,

    // rfc7662
    pub introspection_endpoint: Option<Url>,
    pub introspection_endpoint_auth_methods_supported: Vec<TokenEndpointAuthMethod>,
    pub introspection_endpoint_auth_signing_alg_values_supported: Option<Vec<IdTokenSignAlg>>,

    /// Ref <https://www.rfc-editor.org/rfc/rfc8628#section-4>
    pub device_authorization_endpoint: Option<Url>,
}

/// The response to an OAuth2 rfc8414 metadata request
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
pub struct Oauth2Rfc8414MetadataResponse {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,

    pub jwks_uri: Option<Url>,

    // rfc7591 reg endpoint.
    pub registration_endpoint: Option<Url>,

    pub scopes_supported: Option<Vec<String>>,

    // For Oauth2 should be Code, Token.
    pub response_types_supported: Vec<ResponseType>,
    #[serde(default = "response_modes_supported_default")]
    pub response_modes_supported: Vec<ResponseMode>,
    #[serde(default = "grant_types_supported_default")]
    pub grant_types_supported: Vec<GrantType>,

    #[serde(default = "token_endpoint_auth_methods_supported_default")]
    pub token_endpoint_auth_methods_supported: Vec<TokenEndpointAuthMethod>,

    pub token_endpoint_auth_signing_alg_values_supported: Option<Vec<IdTokenSignAlg>>,

    pub service_documentation: Option<Url>,
    pub ui_locales_supported: Option<Vec<String>>,

    pub op_policy_uri: Option<Url>,
    pub op_tos_uri: Option<Url>,

    // rfc7009
    pub revocation_endpoint: Option<Url>,
    pub revocation_endpoint_auth_methods_supported: Vec<TokenEndpointAuthMethod>,

    // rfc7662
    pub introspection_endpoint: Option<Url>,
    pub introspection_endpoint_auth_methods_supported: Vec<TokenEndpointAuthMethod>,
    pub introspection_endpoint_auth_signing_alg_values_supported: Option<Vec<IdTokenSignAlg>>,

    // RFC7636
    pub code_challenge_methods_supported: Vec<PkceAlg>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<Url>,
}

#[derive(Debug, Serialize, Deserialize)]
/// Ref <https://www.rfc-editor.org/rfc/rfc8628#section-3.2>
pub struct DeviceAuthorizationResponse {
    /// Base64-encoded bundle of 16 bytes
    device_code: String,
    /// xxx-yyy-zzz where x/y/z are digits. Stored internally as a u32 because we'll drop the dashes and parse as a number.
    user_code: String,
    verification_uri: Url,
    verification_uri_complete: Url,
    expires_in: u64,
    interval: u64,
}

impl DeviceAuthorizationResponse {
    pub fn new(verification_uri: Url, device_code: [u8; 16], user_code: String) -> Self {
        let mut verification_uri_complete = verification_uri.clone();
        verification_uri_complete
            .query_pairs_mut()
            .append_pair("user_code", &user_code);

        let device_code = STANDARD.encode(device_code);

        Self {
            verification_uri_complete,
            device_code,
            user_code,
            verification_uri,
            expires_in: OAUTH2_DEVICE_CODE_EXPIRY_SECONDS,
            interval: OAUTH2_DEVICE_CODE_INTERVAL_SECONDS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use url::Url;

    #[test]
    fn test_oauth2_access_token_req() {
        let atr: AccessTokenRequest = GrantTypeReq::AuthorizationCode {
            code: "demo code".to_string(),
            redirect_uri: Url::parse("http://[::1]").unwrap(),
            code_verifier: None,
        }
        .into();

        println!("{:?}", serde_json::to_string(&atr).expect("JSON failure"));
    }

    #[test]
    fn test_oauth2_access_token_type_serde() {
        for testcase in ["bearer", "Bearer", "BeArEr"] {
            let at: super::AccessTokenType =
                serde_json::from_str(&format!("\"{testcase}\"")).expect("Failed to parse");
            assert_eq!(at, super::AccessTokenType::Bearer);
        }

        for testcase in ["dpop", "dPoP", "DPOP", "DPoP"] {
            let at: super::AccessTokenType =
                serde_json::from_str(&format!("\"{testcase}\"")).expect("Failed to parse");
            assert_eq!(at, super::AccessTokenType::DPoP);
        }

        {
            let testcase = "cheese";
            let at = serde_json::from_str::<super::AccessTokenType>(&format!("\"{testcase}\""));
            assert!(at.is_err())
        }
    }

    #[test]
    fn test_token_exchange_grant_serialization() {
        let scopes: BTreeSet<String> = ["groups", "openid"]
            .into_iter()
            .map(str::to_string)
            .collect();

        let atr = AccessTokenRequest {
            grant_type: GrantTypeReq::TokenExchange {
                subject_token: "subject".to_string(),
                subject_token_type: OAUTH2_TOKEN_TYPE_ACCESS_TOKEN.to_string(),
                requested_token_type: None,
                audience: Some("test_resource_server".to_string()),
                resource: None,
                actor_token: None,
                actor_token_type: None,
                scope: Some(scopes.clone()),
            },
            client_post_auth: Default::default(),
        };

        let json = serde_json::to_string(&atr).expect("JSON failure");
        let de: AccessTokenRequest = serde_json::from_str(&json).expect("Roundtrip failure");

        match de.grant_type {
            GrantTypeReq::TokenExchange {
                subject_token,
                subject_token_type,
                requested_token_type,
                audience,
                actor_token,
                actor_token_type,
                scope: descope,
                ..
            } => {
                assert_eq!(subject_token, "subject");
                assert_eq!(subject_token_type, OAUTH2_TOKEN_TYPE_ACCESS_TOKEN);
                assert_eq!(requested_token_type, None);
                assert_eq!(audience.as_deref(), Some("test_resource_server"));
                assert_eq!(actor_token, None);
                assert_eq!(actor_token_type, None);
                assert_eq!(descope, Some(scopes));
            }
            _ => panic!("Wrong grant type"),
        }
    }

    #[test]
    fn test_code_challenge_method_serde() {
        let method = CodeChallengeMethod::S256;
        let json = serde_json::to_string(&method).unwrap();
        let de: CodeChallengeMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(method, de);
    }

    #[test]
    fn test_pkce_request_serde() {
        let req = PkceRequest {
            code_challenge: vec![1, 2, 3, 4, 5],
            code_challenge_method: CodeChallengeMethod::S256,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("code_challenge"));
        assert!(json.contains("S256"));
        let de: PkceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.code_challenge, vec![1, 2, 3, 4, 5]);
        assert_eq!(de.code_challenge_method, CodeChallengeMethod::S256);
    }

    #[test]
    fn test_authorisation_request_serde_full() {
        let mut scope = BTreeSet::new();
        scope.insert("openid".to_string());
        scope.insert("profile".to_string());

        let req = AuthorisationRequest {
            response_type: ResponseType::Code,
            response_mode: Some(ResponseMode::Query),
            client_id: "test_client".to_string(),
            state: Some("random_state".to_string()),
            pkce_request: Some(PkceRequest {
                code_challenge: vec![10, 20, 30],
                code_challenge_method: CodeChallengeMethod::S256,
            }),
            redirect_uri: Url::parse("https://example.com/callback").unwrap(),
            scope: scope.clone(),
            nonce: Some("nonce_val".to_string()),
            oidc_ext: AuthorisationRequestOidc {
                display: Some("page".to_string()),
                prompt: Some("login".to_string()),
                ui_locales: None,
                claims_locales: None,
                id_token_hint: None,
                login_hint: Some("user@example.com".to_string()),
                acr: None,
            },
            max_age: Some(3600),
            unknown_keys: BTreeMap::new(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"response_type\":\"code\""));
        assert!(json.contains("\"client_id\":\"test_client\""));
        assert!(json.contains("\"state\":\"random_state\""));
        assert!(json.contains("\"nonce\":\"nonce_val\""));
        assert!(json.contains("\"max_age\":3600"));
        assert!(json.contains("\"login_hint\":\"user@example.com\""));

        let de: AuthorisationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.response_type, ResponseType::Code);
        assert_eq!(de.response_mode, Some(ResponseMode::Query));
        assert_eq!(de.client_id, "test_client");
        assert_eq!(de.state, Some("random_state".to_string()));
        assert!(de.pkce_request.is_some());
        assert_eq!(de.nonce, Some("nonce_val".to_string()));
        assert_eq!(de.max_age, Some(3600));
        assert_eq!(de.oidc_ext.login_hint, Some("user@example.com".to_string()));
    }

    #[test]
    fn test_authorisation_request_serde_minimal() {
        let mut scope = BTreeSet::new();
        scope.insert("openid".to_string());

        let req = AuthorisationRequest {
            response_type: ResponseType::Code,
            response_mode: None,
            client_id: "minimal_client".to_string(),
            state: None,
            pkce_request: None,
            redirect_uri: Url::parse("https://example.com/cb").unwrap(),
            scope: scope.clone(),
            nonce: None,
            oidc_ext: AuthorisationRequestOidc::default(),
            max_age: None,
            unknown_keys: BTreeMap::new(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("\"state\""));
        assert!(!json.contains("\"nonce\""));
        assert!(!json.contains("\"max_age\""));
        assert!(!json.contains("\"response_mode\""));

        let de: AuthorisationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.response_type, ResponseType::Code);
        assert!(de.state.is_none());
        assert!(de.pkce_request.is_none());
        assert!(de.nonce.is_none());
        assert!(de.max_age.is_none());
    }

    #[test]
    fn test_authorisation_request_get_response_mode() {
        let base = AuthorisationRequest {
            response_type: ResponseType::Code,
            response_mode: None,
            client_id: "c".to_string(),
            state: None,
            pkce_request: None,
            redirect_uri: Url::parse("https://example.com/cb").unwrap(),
            scope: BTreeSet::new(),
            nonce: None,
            oidc_ext: AuthorisationRequestOidc::default(),
            max_age: None,
            unknown_keys: BTreeMap::new(),
        };

        assert_eq!(base.get_response_mode(), Some(ResponseMode::Query));

        let mut req = base.clone();
        req.response_type = ResponseType::Token;
        assert_eq!(req.get_response_mode(), Some(ResponseMode::Fragment));

        req.response_mode = Some(ResponseMode::Query);
        req.response_type = ResponseType::Token;
        assert_eq!(req.get_response_mode(), None);

        let mut req2 = base.clone();
        req2.response_type = ResponseType::IdToken;
        assert_eq!(req2.get_response_mode(), Some(ResponseMode::Fragment));

        req2.response_mode = Some(ResponseMode::Query);
        assert_eq!(req2.get_response_mode(), None);

        let mut req3 = base;
        req3.response_mode = Some(ResponseMode::FormPost);
        assert_eq!(req3.get_response_mode(), Some(ResponseMode::FormPost));
    }

    #[test]
    fn test_authorisation_request_oidc_serde() {
        let oidc = AuthorisationRequestOidc {
            display: Some("popup".to_string()),
            prompt: Some("consent".to_string()),
            ui_locales: None,
            claims_locales: None,
            id_token_hint: Some("hint_token".to_string()),
            login_hint: Some("admin@example.com".to_string()),
            acr: Some("urn:mace:incommon:iap:silver".to_string()),
        };

        let json = serde_json::to_string(&oidc).unwrap();
        assert!(json.contains("\"display\":\"popup\""));
        assert!(json.contains("\"prompt\":\"consent\""));
        assert!(json.contains("\"id_token_hint\":\"hint_token\""));
        assert!(json.contains("\"login_hint\":\"admin@example.com\""));
        assert!(json.contains("\"acr\":\"urn:mace:incommon:iap:silver\""));

        let de: AuthorisationRequestOidc = serde_json::from_str(&json).unwrap();
        assert_eq!(de.display, Some("popup".to_string()));
        assert_eq!(de.prompt, Some("consent".to_string()));
        assert_eq!(de.id_token_hint, Some("hint_token".to_string()));
        assert_eq!(de.login_hint, Some("admin@example.com".to_string()));
        assert_eq!(de.acr, Some("urn:mace:incommon:iap:silver".to_string()));
    }

    #[test]
    fn test_authorisation_request_oidc_serde_default() {
        let oidc = AuthorisationRequestOidc::default();
        let json = serde_json::to_string(&oidc).unwrap();
        assert_eq!(json, "{}");
        let de: AuthorisationRequestOidc = serde_json::from_str(&json).unwrap();
        assert!(de.display.is_none());
        assert!(de.prompt.is_none());
        assert!(de.id_token_hint.is_none());
        assert!(de.login_hint.is_none());
        assert!(de.acr.is_none());
    }

    #[test]
    fn test_authorisation_response_consent_requested_serde() {
        let mut scopes = BTreeSet::new();
        scopes.insert("openid".to_string());
        let mut pii = BTreeSet::new();
        pii.insert("email".to_string());

        let resp = AuthorisationResponse::ConsentRequested {
            client_name: "My App".to_string(),
            scopes: scopes.clone(),
            pii_scopes: pii.clone(),
            consent_token: "token_abc".to_string(),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"ConsentRequested\""));
        assert!(json.contains("\"client_name\":\"My App\""));
        assert!(json.contains("\"consent_token\":\"token_abc\""));

        let de: AuthorisationResponse = serde_json::from_str(&json).unwrap();
        match de {
            AuthorisationResponse::ConsentRequested {
                client_name,
                scopes: s,
                pii_scopes: p,
                consent_token,
            } => {
                assert_eq!(client_name, "My App");
                assert_eq!(s, scopes);
                assert_eq!(p, pii);
                assert_eq!(consent_token, "token_abc");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_authorisation_response_permitted_serde() {
        let resp = AuthorisationResponse::Permitted;
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"Permitted\""));
        let de: AuthorisationResponse = serde_json::from_str(&json).unwrap();
        assert!(matches!(de, AuthorisationResponse::Permitted));
    }

    #[test]
    fn test_grant_type_req_authorization_code_serde() {
        let req = GrantTypeReq::AuthorizationCode {
            code: "auth_code_123".to_string(),
            redirect_uri: Url::parse("https://example.com/cb").unwrap(),
            code_verifier: Some("verifier_xyz".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"grant_type\":\"authorization_code\""));
        assert!(json.contains("\"code\":\"auth_code_123\""));
        assert!(json.contains("\"code_verifier\":\"verifier_xyz\""));

        let de: GrantTypeReq = serde_json::from_str(&json).unwrap();
        match de {
            GrantTypeReq::AuthorizationCode {
                code,
                redirect_uri,
                code_verifier,
            } => {
                assert_eq!(code, "auth_code_123");
                assert_eq!(redirect_uri, Url::parse("https://example.com/cb").unwrap());
                assert_eq!(code_verifier, Some("verifier_xyz".to_string()));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_grant_type_req_client_credentials_serde() {
        let req = GrantTypeReq::ClientCredentials { scope: None };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"grant_type\":\"client_credentials\""));
        let de: GrantTypeReq = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            de,
            GrantTypeReq::ClientCredentials { scope: None }
        ));
    }

    #[test]
    fn test_grant_type_req_refresh_token_serde() {
        let mut scopes = BTreeSet::new();
        scopes.insert("openid".to_string());

        let req = GrantTypeReq::RefreshToken {
            refresh_token: "refresh_abc".to_string(),
            scope: Some(scopes.clone()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"grant_type\":\"refresh_token\""));
        assert!(json.contains("\"refresh_token\":\"refresh_abc\""));

        let de: GrantTypeReq = serde_json::from_str(&json).unwrap();
        match de {
            GrantTypeReq::RefreshToken {
                refresh_token,
                scope,
            } => {
                assert_eq!(refresh_token, "refresh_abc");
                assert_eq!(scope, Some(scopes));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_grant_type_req_device_code_serde() {
        let req = GrantTypeReq::DeviceCode {
            device_code: "dev_code_123".to_string(),
            scope: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"grant_type\":\"urn:ietf:params:oauth:grant-type:device_code\""));
        assert!(json.contains("\"device_code\":\"dev_code_123\""));

        let de: GrantTypeReq = serde_json::from_str(&json).unwrap();
        match de {
            GrantTypeReq::DeviceCode { device_code, scope } => {
                assert_eq!(device_code, "dev_code_123");
                assert!(scope.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_access_token_request_serde() {
        let atr = AccessTokenRequest {
            grant_type: GrantTypeReq::RefreshToken {
                refresh_token: "rt_123".to_string(),
                scope: None,
            },
            client_post_auth: ClientPostAuth {
                client_id: Some("myclient".to_string()),
                client_secret: Some("secret123".to_string()),
            },
        };

        let json = serde_json::to_string(&atr).unwrap();
        assert!(json.contains("\"grant_type\":\"refresh_token\""));
        assert!(json.contains("\"client_id\":\"myclient\""));
        assert!(json.contains("\"client_secret\":\"secret123\""));

        let de: AccessTokenRequest = serde_json::from_str(&json).unwrap();
        match &de.grant_type {
            GrantTypeReq::RefreshToken { refresh_token, .. } => {
                assert_eq!(refresh_token, "rt_123");
            }
            _ => panic!("Wrong variant"),
        }
        assert_eq!(de.client_post_auth.client_id, Some("myclient".to_string()));
    }

    #[test]
    fn test_access_token_response_serde() {
        let mut scope = BTreeSet::new();
        scope.insert("openid".to_string());
        scope.insert("groups".to_string());

        let resp = AccessTokenResponse {
            access_token: "at_12345".to_string(),
            token_type: AccessTokenType::Bearer,
            issued_token_type: Some(IssuedTokenType::AccessToken),
            expires_in: 3600,
            refresh_token: Some("rt_67890".to_string()),
            scope: scope.clone(),
            id_token: Some("idt_jwt".to_string()),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"access_token\":\"at_12345\""));
        assert!(json.contains("\"token_type\":\"Bearer\""));
        assert!(json.contains("\"expires_in\":3600"));
        assert!(json.contains("\"refresh_token\":\"rt_67890\""));
        assert!(json.contains("\"id_token\":\"idt_jwt\""));

        let de: AccessTokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.access_token, "at_12345");
        assert_eq!(de.token_type, AccessTokenType::Bearer);
        assert_eq!(de.issued_token_type, Some(IssuedTokenType::AccessToken));
        assert_eq!(de.expires_in, 3600);
        assert_eq!(de.refresh_token, Some("rt_67890".to_string()));
        assert_eq!(de.scope, scope);
        assert_eq!(de.id_token, Some("idt_jwt".to_string()));
    }

    #[test]
    fn test_access_token_response_minimal_serde() {
        let resp = AccessTokenResponse {
            access_token: "at_min".to_string(),
            token_type: AccessTokenType::Bearer,
            issued_token_type: None,
            expires_in: 600,
            refresh_token: None,
            scope: BTreeSet::new(),
            id_token: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"refresh_token\""));
        assert!(!json.contains("\"id_token\""));
        assert!(!json.contains("\"issued_token_type\""));

        let de: AccessTokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.access_token, "at_min");
        assert!(de.refresh_token.is_none());
        assert!(de.id_token.is_none());
        assert!(de.issued_token_type.is_none());
    }

    #[test]
    fn test_access_token_type_bearer_serde() {
        let tt = AccessTokenType::Bearer;
        let json = serde_json::to_string(&tt).unwrap();
        let de: AccessTokenType = serde_json::from_str(&json).unwrap();
        assert_eq!(tt, de);
    }

    #[test]
    fn test_access_token_type_pop_serde() {
        let tt = AccessTokenType::PoP;
        let json = serde_json::to_string(&tt).unwrap();
        let de: AccessTokenType = serde_json::from_str(&json).unwrap();
        assert_eq!(tt, de);
    }

    #[test]
    fn test_access_token_type_na_serde() {
        let tt = AccessTokenType::NA;
        let json = serde_json::to_string(&tt).unwrap();
        assert!(json.contains("N_A"));
        let de: AccessTokenType = serde_json::from_str(&json).unwrap();
        assert_eq!(tt, de);
    }

    #[test]
    fn test_access_token_type_dpop_serde() {
        let tt = AccessTokenType::DPoP;
        let json = serde_json::to_string(&tt).unwrap();
        let de: AccessTokenType = serde_json::from_str(&json).unwrap();
        assert_eq!(tt, de);
    }

    #[test]
    fn test_token_revoke_request_serde() {
        let req = TokenRevokeRequest {
            token: "tok_to_revoke".to_string(),
            token_type_hint: Some("access_token".to_string()),
            client_post_auth: ClientPostAuth::default(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"token\":\"tok_to_revoke\""));
        assert!(json.contains("\"token_type_hint\":\"access_token\""));

        let de: TokenRevokeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.token, "tok_to_revoke");
        assert_eq!(de.token_type_hint, Some("access_token".to_string()));
    }

    #[test]
    fn test_token_revoke_request_minimal_serde() {
        let req = TokenRevokeRequest {
            token: "tok_min".to_string(),
            token_type_hint: None,
            client_post_auth: ClientPostAuth::default(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("token_type_hint"));

        let de: TokenRevokeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.token, "tok_min");
        assert!(de.token_type_hint.is_none());
    }

    #[test]
    fn test_client_post_auth_serde() {
        let auth = ClientPostAuth {
            client_id: Some("id123".to_string()),
            client_secret: Some("sec456".to_string()),
        };

        let json = serde_json::to_string(&auth).unwrap();
        let de: ClientPostAuth = serde_json::from_str(&json).unwrap();
        assert_eq!(de.client_id, Some("id123".to_string()));
        assert_eq!(de.client_secret, Some("sec456".to_string()));
    }

    #[test]
    fn test_client_post_auth_default_serde() {
        let auth = ClientPostAuth::default();
        let json = serde_json::to_string(&auth).unwrap();
        assert_eq!(json, "{}");
        let de: ClientPostAuth = serde_json::from_str(&json).unwrap();
        assert!(de.client_id.is_none());
        assert!(de.client_secret.is_none());
    }

    #[test]
    fn test_client_post_auth_from_tuple() {
        let auth: ClientPostAuth = ("myclient", Some("mysecret")).into();
        assert_eq!(auth.client_id, Some("myclient".to_string()));
        assert_eq!(auth.client_secret, Some("mysecret".to_string()));

        let auth2: ClientPostAuth = ("client2", None).into();
        assert_eq!(auth2.client_id, Some("client2".to_string()));
        assert_eq!(auth2.client_secret, None);
    }

    #[test]
    fn test_client_auth_serde() {
        let auth = ClientAuth {
            client_id: "cid".to_string(),
            client_secret: Some("csecret".to_string()),
        };

        let json = serde_json::to_string(&auth).unwrap();
        let de: ClientAuth = serde_json::from_str(&json).unwrap();
        assert_eq!(de.client_id, "cid");
        assert_eq!(de.client_secret, Some("csecret".to_string()));
    }

    #[test]
    fn test_client_auth_no_secret_serde() {
        let auth = ClientAuth {
            client_id: "cid2".to_string(),
            client_secret: None,
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(!json.contains("client_secret"));
        let de: ClientAuth = serde_json::from_str(&json).unwrap();
        assert_eq!(de.client_id, "cid2");
        assert!(de.client_secret.is_none());
    }

    #[test]
    fn test_client_auth_from_tuple() {
        let auth: ClientAuth = ("myclient", Some("mysecret")).into();
        assert_eq!(auth.client_id, "myclient");
        assert_eq!(auth.client_secret, Some("mysecret".to_string()));
    }

    #[test]
    fn test_access_token_introspect_request_serde() {
        let req = AccessTokenIntrospectRequest {
            token: "introspect_me".to_string(),
            token_type_hint: Some("refresh_token".to_string()),
            client_post_auth: ClientPostAuth {
                client_id: Some("cid".to_string()),
                client_secret: Some("csec".to_string()),
            },
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"token\":\"introspect_me\""));

        let de: AccessTokenIntrospectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.token, "introspect_me");
        assert_eq!(de.token_type_hint, Some("refresh_token".to_string()));
    }

    #[test]
    fn test_access_token_introspect_response_active_serde() {
        let mut scope = BTreeSet::new();
        scope.insert("openid".to_string());

        let resp = AccessTokenIntrospectResponse {
            active: true,
            scope: scope.clone(),
            client_id: Some("client1".to_string()),
            username: Some("user1".to_string()),
            token_type: Some(AccessTokenType::Bearer),
            exp: Some(1234567890),
            iat: Some(1234560000),
            nbf: Some(1234560000),
            sub: Some("user1".to_string()),
            aud: Some("client1".to_string()),
            iss: Some("https://id.example.com".to_string()),
            jti: Uuid::new_v4(),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"active\":true"));
        assert!(json.contains("\"client_id\":\"client1\""));

        let de: AccessTokenIntrospectResponse = serde_json::from_str(&json).unwrap();
        assert!(de.active);
        assert_eq!(de.scope, scope);
        assert_eq!(de.client_id, Some("client1".to_string()));
        assert_eq!(de.username, Some("user1".to_string()));
        assert_eq!(de.token_type, Some(AccessTokenType::Bearer));
        assert_eq!(de.exp, Some(1234567890));
    }

    #[test]
    fn test_access_token_introspect_response_inactive() {
        let session_id = Uuid::new_v4();
        let resp = AccessTokenIntrospectResponse::inactive(session_id);
        assert!(!resp.active);
        assert!(resp.scope.is_empty());
        assert!(resp.client_id.is_none());
        assert!(resp.username.is_none());
        assert_eq!(resp.jti, session_id);
    }

    #[test]
    fn test_response_type_serde() {
        let variants = [
            (ResponseType::Code, "code"),
            (ResponseType::Token, "token"),
            (ResponseType::IdToken, "id_token"),
        ];
        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: ResponseType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, de);
        }
    }

    #[test]
    fn test_response_mode_serde() {
        assert_eq!(
            serde_json::to_string(&ResponseMode::Query).unwrap(),
            "\"query\""
        );
        assert_eq!(
            serde_json::to_string(&ResponseMode::Fragment).unwrap(),
            "\"fragment\""
        );
        assert_eq!(
            serde_json::to_string(&ResponseMode::FormPost).unwrap(),
            "\"form_post\""
        );

        let de: ResponseMode = serde_json::from_str("\"query\"").unwrap();
        assert_eq!(de, ResponseMode::Query);
        let de: ResponseMode = serde_json::from_str("\"fragment\"").unwrap();
        assert_eq!(de, ResponseMode::Fragment);
        let de: ResponseMode = serde_json::from_str("\"form_post\"").unwrap();
        assert_eq!(de, ResponseMode::FormPost);
    }

    #[test]
    fn test_response_mode_invalid_is_catchall() {
        assert_eq!(ResponseMode::Invalid, ResponseMode::Invalid);
    }

    #[test]
    fn test_grant_type_serde() {
        let json = serde_json::to_string(&GrantType::AuthorisationCode).unwrap();
        assert_eq!(json, "\"authorization_code\"");
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::AuthorisationCode);

        let json = serde_json::to_string(&GrantType::Implicit).unwrap();
        assert_eq!(json, "\"implicit\"");
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::Implicit);

        let json = serde_json::to_string(&GrantType::TokenExchange).unwrap();
        assert_eq!(json, "\"urn:ietf:params:oauth:grant-type:token-exchange\"");
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::TokenExchange);
    }

    #[test]
    fn test_subject_type_serde() {
        assert_eq!(
            serde_json::to_string(&SubjectType::Pairwise).unwrap(),
            "\"pairwise\""
        );
        assert_eq!(
            serde_json::to_string(&SubjectType::Public).unwrap(),
            "\"public\""
        );
        let de: SubjectType = serde_json::from_str("\"pairwise\"").unwrap();
        assert_eq!(de, SubjectType::Pairwise);
        let de: SubjectType = serde_json::from_str("\"public\"").unwrap();
        assert_eq!(de, SubjectType::Public);
    }

    #[test]
    fn test_id_token_sign_alg_serde() {
        assert_eq!(
            serde_json::to_string(&IdTokenSignAlg::ES256).unwrap(),
            "\"ES256\""
        );
        assert_eq!(
            serde_json::to_string(&IdTokenSignAlg::RS256).unwrap(),
            "\"RS256\""
        );
        let de: IdTokenSignAlg = serde_json::from_str("\"ES256\"").unwrap();
        assert_eq!(de, IdTokenSignAlg::ES256);
        let de: IdTokenSignAlg = serde_json::from_str("\"RS256\"").unwrap();
        assert_eq!(de, IdTokenSignAlg::RS256);
    }

    #[test]
    fn test_token_endpoint_auth_method_serde() {
        let variants = [
            (
                TokenEndpointAuthMethod::ClientSecretPost,
                "client_secret_post",
            ),
            (
                TokenEndpointAuthMethod::ClientSecretBasic,
                "client_secret_basic",
            ),
            (
                TokenEndpointAuthMethod::ClientSecretJwt,
                "client_secret_jwt",
            ),
            (TokenEndpointAuthMethod::PrivateKeyJwt, "private_key_jwt"),
        ];
        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: TokenEndpointAuthMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, de);
        }
    }

    #[test]
    fn test_oidc_discovery_response_serde() {
        let resp = OidcDiscoveryResponse {
            issuer: Url::parse("https://id.example.com").unwrap(),
            authorization_endpoint: Url::parse("https://id.example.com/oauth2/authorise").unwrap(),
            token_endpoint: Url::parse("https://id.example.com/oauth2/token").unwrap(),
            userinfo_endpoint: Some(Url::parse("https://id.example.com/oauth2/userinfo").unwrap()),
            jwks_uri: Url::parse("https://id.example.com/oauth2/openid/jwks").unwrap(),
            registration_endpoint: None,
            scopes_supported: Some(vec!["openid".to_string(), "profile".to_string()]),
            response_types_supported: vec![ResponseType::Code],
            response_modes_supported: vec![ResponseMode::Query, ResponseMode::Fragment],
            grant_types_supported: vec![GrantType::AuthorisationCode],
            acr_values_supported: None,
            subject_types_supported: vec![SubjectType::Public],
            id_token_signing_alg_values_supported: vec![IdTokenSignAlg::ES256],
            id_token_encryption_alg_values_supported: None,
            id_token_encryption_enc_values_supported: None,
            userinfo_signing_alg_values_supported: None,
            userinfo_encryption_alg_values_supported: None,
            userinfo_encryption_enc_values_supported: None,
            request_object_signing_alg_values_supported: None,
            request_object_encryption_alg_values_supported: None,
            request_object_encryption_enc_values_supported: None,
            token_endpoint_auth_methods_supported: vec![TokenEndpointAuthMethod::ClientSecretBasic],
            token_endpoint_auth_signing_alg_values_supported: None,
            display_values_supported: None,
            claim_types_supported: vec![ClaimType::Normal],
            claims_supported: Some(vec!["sub".to_string()]),
            service_documentation: None,
            claims_locales_supported: None,
            ui_locales_supported: None,
            claims_parameter_supported: false,
            op_policy_uri: None,
            op_tos_uri: None,
            request_parameter_supported: false,
            request_uri_parameter_supported: false,
            require_request_uri_registration: false,
            code_challenge_methods_supported: vec![PkceAlg::S256],
            revocation_endpoint: Some(Url::parse("https://id.example.com/oauth2/revoke").unwrap()),
            revocation_endpoint_auth_methods_supported: vec![
                TokenEndpointAuthMethod::ClientSecretBasic,
            ],
            introspection_endpoint: Some(
                Url::parse("https://id.example.com/oauth2/introspect").unwrap(),
            ),
            introspection_endpoint_auth_methods_supported: vec![
                TokenEndpointAuthMethod::ClientSecretBasic,
            ],
            introspection_endpoint_auth_signing_alg_values_supported: None,
            device_authorization_endpoint: Some(
                Url::parse("https://id.example.com/oauth2/device").unwrap(),
            ),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"response_types_supported\":[\"code\"]"));
        assert!(json.contains("\"subject_types_supported\":[\"public\"]"));
        assert!(json.contains("\"id_token_signing_alg_values_supported\":[\"ES256\"]"));
        assert!(json.contains("\"code_challenge_methods_supported\":[\"S256\"]"));
        assert!(json.contains("\"claims_parameter_supported\":false"));

        let de: OidcDiscoveryResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.issuer, resp.issuer);
        assert_eq!(de.authorization_endpoint, resp.authorization_endpoint);
        assert_eq!(de.token_endpoint, resp.token_endpoint);
        assert_eq!(de.response_types_supported, resp.response_types_supported);
        assert_eq!(de.subject_types_supported, resp.subject_types_supported);
        assert_eq!(
            de.id_token_signing_alg_values_supported,
            resp.id_token_signing_alg_values_supported
        );
        assert_eq!(
            de.code_challenge_methods_supported,
            resp.code_challenge_methods_supported
        );
    }

    #[test]
    fn test_oidc_discovery_response_defaults() {
        let json = r#"{
            "issuer": "https://id.example.com",
            "authorization_endpoint": "https://id.example.com/oauth2/authorise",
            "token_endpoint": "https://id.example.com/oauth2/token",
            "jwks_uri": "https://id.example.com/oauth2/openid/jwks",
            "response_types_supported": ["code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"],
            "code_challenge_methods_supported": ["S256"],
            "revocation_endpoint_auth_methods_supported": ["client_secret_basic"],
            "introspection_endpoint_auth_methods_supported": ["client_secret_basic"]
        }"#;

        let de: OidcDiscoveryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            de.response_modes_supported,
            vec![ResponseMode::Query, ResponseMode::Fragment]
        );
        assert_eq!(
            de.grant_types_supported,
            vec![
                GrantType::AuthorisationCode,
                GrantType::Implicit,
                GrantType::TokenExchange
            ]
        );
        assert_eq!(
            de.token_endpoint_auth_methods_supported,
            vec![TokenEndpointAuthMethod::ClientSecretBasic]
        );
        assert_eq!(de.claim_types_supported, vec![ClaimType::Normal]);
        assert!(!de.claims_parameter_supported);
        assert!(!de.request_parameter_supported);
        assert!(!de.request_uri_parameter_supported);
        assert!(!de.require_request_uri_registration);
    }

    #[test]
    fn test_device_authorization_response_serde() {
        let verification_uri = Url::parse("https://example.com/device").unwrap();
        let user_code = "123-456".to_string();
        let device_code_bytes: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let resp = DeviceAuthorizationResponse::new(
            verification_uri,
            device_code_bytes,
            user_code.clone(),
        );

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"verification_uri\":\"https://example.com/device\""));
        assert!(json.contains("\"user_code\":\"123-456\""));
        assert!(json.contains(&format!(
            "\"expires_in\":{OAUTH2_DEVICE_CODE_EXPIRY_SECONDS}"
        )));
        assert!(json.contains(&format!(
            "\"interval\":{OAUTH2_DEVICE_CODE_INTERVAL_SECONDS}"
        )));
        assert!(json.contains("verification_uri_complete"));

        let json2 = serde_json::to_string(
            &serde_json::from_str::<DeviceAuthorizationResponse>(&json).unwrap(),
        )
        .unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_error_response_serde() {
        let err = ErrorResponse {
            error: "invalid_request".to_string(),
            error_description: Some("Missing parameter".to_string()),
            error_uri: Some(Url::parse("https://example.com/errors/invalid").unwrap()),
        };

        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"error\":\"invalid_request\""));
        assert!(json.contains("\"error_description\":\"Missing parameter\""));
        assert!(json.contains("\"error_uri\""));

        let de: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.error, "invalid_request");
        assert_eq!(de.error_description, Some("Missing parameter".to_string()));
        assert!(de.error_uri.is_some());
    }

    #[test]
    fn test_error_response_minimal_serde() {
        let err = ErrorResponse {
            error: "unauthorized_client".to_string(),
            error_description: None,
            error_uri: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("error_description"));
        assert!(!json.contains("error_uri"));

        let de: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.error, "unauthorized_client");
        assert!(de.error_description.is_none());
        assert!(de.error_uri.is_none());
    }

    #[test]
    fn test_pkce_alg_serde() {
        assert_eq!(serde_json::to_string(&PkceAlg::S256).unwrap(), "\"S256\"");
        let de: PkceAlg = serde_json::from_str("\"S256\"").unwrap();
        assert_eq!(de, PkceAlg::S256);
    }

    #[test]
    fn test_display_value_serde() {
        assert_eq!(
            serde_json::to_string(&DisplayValue::Page).unwrap(),
            "\"page\""
        );
        assert_eq!(
            serde_json::to_string(&DisplayValue::Popup).unwrap(),
            "\"popup\""
        );
        assert_eq!(
            serde_json::to_string(&DisplayValue::Touch).unwrap(),
            "\"touch\""
        );
        assert_eq!(
            serde_json::to_string(&DisplayValue::Wap).unwrap(),
            "\"wap\""
        );
        let de: DisplayValue = serde_json::from_str("\"page\"").unwrap();
        assert_eq!(de, DisplayValue::Page);
    }

    #[test]
    fn test_claim_type_serde() {
        assert_eq!(
            serde_json::to_string(&ClaimType::Normal).unwrap(),
            "\"normal\""
        );
        assert_eq!(
            serde_json::to_string(&ClaimType::Aggregated).unwrap(),
            "\"aggregated\""
        );
        assert_eq!(
            serde_json::to_string(&ClaimType::Distributed).unwrap(),
            "\"distributed\""
        );
        let de: ClaimType = serde_json::from_str("\"normal\"").unwrap();
        assert_eq!(de, ClaimType::Normal);
    }

    #[test]
    fn test_issued_token_type_serde() {
        let variants = [
            (IssuedTokenType::AccessToken, "AccessToken"),
            (IssuedTokenType::RefreshToken, "RefreshToken"),
            (IssuedTokenType::IdToken, "IdToken"),
            (IssuedTokenType::Saml1, "Saml1"),
            (IssuedTokenType::Saml2, "Saml2"),
        ];
        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let de: IssuedTokenType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, de);
        }
    }

    #[test]
    fn test_oidc_webfinger_rel_serde() {
        let rel = OidcWebfingerRel {
            rel: "http://openid.net/specs/connect/1.0/issuer".to_string(),
            href: "https://id.example.com".to_string(),
        };
        let json = serde_json::to_string(&rel).unwrap();
        let de: OidcWebfingerRel = serde_json::from_str(&json).unwrap();
        assert_eq!(de.rel, "http://openid.net/specs/connect/1.0/issuer");
        assert_eq!(de.href, "https://id.example.com");
    }

    #[test]
    fn test_oidc_webfinger_response_serde() {
        let resp = OidcWebfingerResponse {
            subject: "acct:user@example.com".to_string(),
            links: vec![OidcWebfingerRel {
                rel: "http://openid.net/specs/connect/1.0/issuer".to_string(),
                href: "https://id.example.com".to_string(),
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"subject\":\"acct:user@example.com\""));
        let de: OidcWebfingerResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.subject, "acct:user@example.com");
        assert_eq!(de.links.len(), 1);
    }

    #[test]
    fn test_oauth2_rfc8414_metadata_response_serde() {
        let resp = Oauth2Rfc8414MetadataResponse {
            issuer: Url::parse("https://id.example.com").unwrap(),
            authorization_endpoint: Url::parse("https://id.example.com/oauth2/authorise").unwrap(),
            token_endpoint: Url::parse("https://id.example.com/oauth2/token").unwrap(),
            jwks_uri: Some(Url::parse("https://id.example.com/oauth2/openid/jwks").unwrap()),
            registration_endpoint: None,
            scopes_supported: None,
            response_types_supported: vec![ResponseType::Code],
            response_modes_supported: vec![ResponseMode::Query],
            grant_types_supported: vec![GrantType::AuthorisationCode],
            token_endpoint_auth_methods_supported: vec![TokenEndpointAuthMethod::ClientSecretBasic],
            token_endpoint_auth_signing_alg_values_supported: None,
            service_documentation: None,
            ui_locales_supported: None,
            op_policy_uri: None,
            op_tos_uri: None,
            revocation_endpoint: None,
            revocation_endpoint_auth_methods_supported: vec![],
            introspection_endpoint: None,
            introspection_endpoint_auth_methods_supported: vec![],
            introspection_endpoint_auth_signing_alg_values_supported: None,
            code_challenge_methods_supported: vec![PkceAlg::S256],
        };

        let json = serde_json::to_string(&resp).unwrap();

        let de: Oauth2Rfc8414MetadataResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(de.issuer, resp.issuer);
        assert_eq!(de.authorization_endpoint, resp.authorization_endpoint);
        assert_eq!(de.response_types_supported, resp.response_types_supported);
    }

    #[test]
    fn test_oauth2_rfc9068_token_serde() {
        let extensions = OAuth2RFC9068TokenExtensions {
            auth_time: Some(1234567890),
            acr: Some("urn:mace:incommon:iap:silver".to_string()),
            amr: Some(vec!["pwd".to_string(), "mfa".to_string()]),
            scope: {
                let mut s = BTreeSet::new();
                s.insert("openid".to_string());
                s
            },
            nonce: Some("abc123".to_string()),
            session_id: Uuid::new_v4(),
            parent_session_id: Some(Uuid::new_v4()),
        };

        let token = OAuth2RFC9068Token {
            iss: "https://id.example.com".to_string(),
            sub: Uuid::new_v4(),
            aud: "client_id_123".to_string(),
            exp: 9999999999,
            nbf: 1234567890,
            iat: 1234567890,
            jti: Uuid::new_v4(),
            client_id: "client_id_123".to_string(),
            extensions: extensions.clone(),
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("\"iss\":\"https://id.example.com\""));
        assert!(json.contains("\"aud\":\"client_id_123\""));
        assert!(json.contains("\"client_id\":\"client_id_123\""));

        let de: OAuth2RFC9068Token<OAuth2RFC9068TokenExtensions> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(de.iss, "https://id.example.com");
        assert_eq!(de.aud, "client_id_123");
        assert_eq!(de.client_id, "client_id_123");
        assert_eq!(de.exp, 9999999999);
        assert_eq!(de.extensions.auth_time, extensions.auth_time);
        assert_eq!(de.extensions.acr, extensions.acr);
        assert_eq!(de.extensions.nonce, extensions.nonce);
        assert_eq!(de.extensions.scope, extensions.scope);
    }

    #[test]
    fn test_device_code_constants() {
        assert_eq!(OAUTH2_DEVICE_CODE_EXPIRY_SECONDS, 300);
        assert_eq!(OAUTH2_DEVICE_CODE_INTERVAL_SECONDS, 5);
    }

    #[test]
    fn test_token_type_access_token_constant() {
        assert_eq!(
            OAUTH2_TOKEN_TYPE_ACCESS_TOKEN,
            "urn:ietf:params:oauth:token-type:access_token"
        );
    }
}
