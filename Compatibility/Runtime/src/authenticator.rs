//! Authentication functionality.
use xal_new::{Error, SignaturePolicyCache};

use xal_new::extensions::{
    CorrelationVectorReqwestBuilder, JsonExDeserializeMiddleware, LoggingReqwestRequestHandler,
    LoggingReqwestResponseHandler, SigningReqwestBuilder,
};
use xal_new::{RequestSigner, XalAppParameters, XalClientParameters, request, response};

use xal_new::request::{XADPropertiesRPS, XSTSProperties, XTokenRequest};

use xal_new::cvlib;

/// Authentication related constants
pub struct Constants;

impl Constants {
    /// Xbox Sisu authorization endpoint
    pub const XBOX_SISU_AUTHORIZE_URL: &'static str = "https://sisu.xboxlive.com/authorize";

    /// Xbox Device Authentication endpoint (XASD token)
    pub const XBOX_DEVICE_AUTH_URL: &'static str =
        "https://device.auth.xboxlive.com/device/authenticate";
    /// Xbox Service Authorization endpoint (XSTS token)
    pub const XBOX_XSTS_AUTH_URL: &'static str = "https://xsts.auth.xboxlive.com/xsts/authorize";

    /// Relying Party Auth Xbox Live
    pub const RELYING_PARTY_AUTH_XBOXLIVE: &'static str = "http://auth.xboxlive.com";
}

/// XAL Authenticator
#[derive(Debug)]
pub struct XalAuthenticator {
    /// Random device id
    device_id: uuid::Uuid,
    /// Application parameters
    ///
    /// See constants in [`crate::models::app_params]
    app_params: XalAppParameters,
    /// Client parameters
    ///
    /// See constants in [`crate::models::client_params]
    client_params: XalClientParameters,
    /// Correlation vector
    ms_cv: cvlib::CorrelationVector,
    /// HTTP client instance
    client: reqwest::Client,
    /// HTTP request signer
    request_signer: RequestSigner,
    /// Xbox Live Sandbox Id, "RETAIL" is commonly used
    sandbox_id: String,
}

impl Default for XalAuthenticator {
    fn default() -> Self {
        Self::new(
            reqwest::Client::new(),
            XalAppParameters::default(),
            XalClientParameters::default(),
            "RETAIL".to_string(),
            SignaturePolicyCache::default(),
        )
    }
}

/// OAuth2 request functionality
impl XalAuthenticator {
    /// Create a new instance of the XAL Authenticator
    ///
    /// This method initializes an instance of the XAL Authenticator with the specified
    /// `app_params`, `client_params`, and `sandbox_id`.
    ///
    /// See constants in [`crate::models::app_params`] for [`crate::XalAppParameters`] and
    /// [`crate::models::client_params`] for [`crate::XalClientParameters`].
    ///
    /// # Examples
    ///
    /// Instantiate explicitly with app/client parameters
    ///
    /// ```
    /// use xal::{XalAuthenticator, app_params, client_params};
    /// let authenticator = XalAuthenticator::new(
    ///     app_params::APP_GAMEPASS_BETA(),
    ///     client_params::CLIENT_ANDROID(),
    ///     "RETAIL".into(),
    /// );
    /// ```
    ///
    /// # Notes
    ///
    /// If you don't have specific needs for client parameters, use [`crate::XalAuthenticator::default`]
    pub fn new(
        client: reqwest::Client,
        app_params: XalAppParameters,
        client_params: XalClientParameters,
        sandbox_id: String,
        signature_policy_cache: SignaturePolicyCache,
    ) -> Self {
        let mut signer = RequestSigner::default();
        signer.signature_policy_cache = signature_policy_cache;
        Self {
            app_params,
            client_params,
            device_id: uuid::Uuid::new_v4(),
            ms_cv: cvlib::CorrelationVector::new(),
            client: client,
            request_signer: signer,
            sandbox_id: sandbox_id.to_owned(),
        }
    }

    /// Create a new instance of the XAL Authenticator with explicit Device Id.
    ///
    /// See `new()` method.
    ///
    /// # Examples
    ///
    /// Instantiate explicitly with app/client parameters and device id
    ///
    /// ```
    /// use xal::{XalAuthenticator, app_params, client_params};
    /// let authenticator = XalAuthenticator::with_device_id(
    ///     app_params::APP_GAMEPASS_BETA(),
    ///     client_params::CLIENT_ANDROID(),
    ///     "RETAIL".into(),
    ///     uuid::uuid!("dc1183d0-a9f8-4c3f-a2a9-83706023791e")
    /// );
    /// ```
    ///
    /// # Notes
    ///
    /// If you don't have specific needs for client parameters, use [`crate::XalAuthenticator::default`]
    pub fn with_device_id(
        app_params: XalAppParameters,
        client_params: XalClientParameters,
        sandbox_id: String,
        device_id: uuid::Uuid,
    ) -> Self {
        Self {
            app_params,
            client_params,
            device_id,
            sandbox_id,
            ..Default::default()
        }
    }

    /// Get Device Id
    pub fn device_id(&self) -> uuid::Uuid {
        self.device_id
    }

    /// Get configured sandbox id
    pub fn sandbox_id(&self) -> String {
        self.sandbox_id.clone()
    }

    /// Get active app parameters
    pub fn app_params(&self) -> XalAppParameters {
        self.app_params.clone()
    }

    /// Get active client parameters
    pub fn client_params(&self) -> XalClientParameters {
        self.client_params.clone()
    }

    /// Get request signer instance
    pub fn request_signer(&self) -> RequestSigner {
        self.request_signer.clone()
    }
}

/// Xbox Live token functionality
impl XalAuthenticator {
    /// Authorize via SISU flow after completing OAuth2 Authentication
    /// Unlike the normal sisu, this windows device sisu needs RST2 tokens
    /// and works for titles that have blocked the mobile xal flow
    pub async fn sisu_authorize_rps(
        &mut self,
        access_token: &str,
        device_token: &str,
        sisu_session_id: Option<&str>,
    ) -> Result<response::SisuRPSAuthorizationResponse, Error> {
        let json_body = request::SisuAuthorizationRequest {
            access_token: access_token,
            app_id: &self.app_params.client_id,
            device_token: device_token,
            sandbox: &self.sandbox_id.clone(),
            site_name: "user.auth.xboxlive.com",
            session_id: sisu_session_id.map(|a| a.to_string()),
            proof_key: self.request_signer.get_proof_key(),
        };

        self.client
            .post(Constants::XBOX_SISU_AUTHORIZE_URL)
            .add_cv(&mut self.ms_cv)?
            .json(&json_body)
            .sign(&mut self.request_signer, None)
            .await?
            .send()
            .await?
            .json_ex::<response::SisuRPSAuthorizationResponse>()
            .await
    }

    /// Requests a Xbox Live Device Token from the Xbox Live authentication service.
    /// This method takes an RPS Device ticket from windows system auth
    pub async fn get_device_token_rps(
        &mut self,
        rps: String,
    ) -> Result<response::DeviceToken, Error> {
        let json_body = XTokenRequest::<XADPropertiesRPS> {
            relying_party: Constants::RELYING_PARTY_AUTH_XBOXLIVE,
            token_type: "JWT",
            properties: XADPropertiesRPS {
                auth_method: "RPS",
                rps_ticket: &rps,
                site_name: "user.auth.xboxlive.com",
                version: &self.client_params.client_version,
                proof_key: self.request_signer.get_proof_key(),
            },
        };

        self.client
            .post(Constants::XBOX_DEVICE_AUTH_URL)
            .header("x-xbl-contract-version", "1")
            .add_cv(&mut self.ms_cv)?
            .json(&json_body)
            .sign(&mut self.request_signer, None)
            .await?
            .send()
            .await?
            .json_ex::<response::DeviceToken>()
            .await
    }

    /// Authenticates with the Xbox Live service and retrieves an XSTS token.
    ///
    /// This method sends a POST request to the Xbox Live XSTS Authentication URL, using the provided `relying_party`
    /// and optionally `device_token`, `title_token`, and `user_token`.
    ///
    /// The resulting XSTS token can be used to authenticate with various Xbox Live services.
    ///
    /// # Arguments
    ///
    /// * `device_token` - (Optional) The Xbox Live device token.
    /// * `title_token` - (Optional) The Xbox Live title token.
    /// * `user_token` - (Optional) The Xbox Live user token.
    /// * `relying_party` - The relying party of the application.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` if the POST request fails or the JSON response cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// use xal::{XalAuthenticator, Flows, Error, AccessTokenPrefix, CliCallbackHandler};
    /// use xal::response::WindowsLiveTokens;
    ///
    /// # async fn example() -> Result<(), Error> {
    /// let mut authenticator = XalAuthenticator::new(
    ///     xal::app_params::MC_BEDROCK_SWITCH(),
    ///     xal::client_params::CLIENT_NINTENDO(),
    ///     "RETAIL".into()
    /// );
    ///
    /// let token_store = Flows::ms_device_code_flow(
    ///     &mut authenticator,
    ///     CliCallbackHandler,
    ///     tokio::time::sleep
    /// )
    /// .await?;
    ///
    /// let device_token = authenticator.get_device_token()
    ///     .await?;
    ///
    /// let title_token = authenticator.get_title_token(
    ///     &token_store.live_token,
    ///     &device_token,
    /// )
    /// .await?;
    ///
    /// let user_token = authenticator.get_user_token(
    ///     &token_store.live_token,
    ///     AccessTokenPrefix::None,
    /// )
    /// .await?;
    ///
    /// let xsts_token = authenticator.get_xsts_token(
    ///     Some(&device_token),
    ///     Some(&title_token),
    ///     Some(&user_token),
    ///     "rp://api.minecraftservices.com/",
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_xsts_token(
        &mut self,
        device_token: Option<&response::DeviceToken>,
        title_token: Option<&response::TitleToken>,
        user_token: Option<&response::UserToken>,
        relying_party: &str,
    ) -> Result<response::XSTSToken, Error> {
        let dtoken = device_token.map(|t| t.token.clone());
        let ttoken = title_token.map(|t| t.token.clone());

        let json_body = XTokenRequest::<XSTSProperties> {
            relying_party,
            token_type: "JWT",
            properties: XSTSProperties {
                sandbox_id: &self.sandbox_id,
                device_token: dtoken.as_deref(),
                title_token: ttoken.as_deref(),
                user_tokens: if let Some(token) = user_token {
                    vec![&token.token]
                } else {
                    vec![]
                },
            },
        };

        self.client
            .post(Constants::XBOX_XSTS_AUTH_URL)
            .header("x-xbl-contract-version", "1")
            .add_cv(&mut self.ms_cv)?
            .json(&json_body)
            .sign(&mut self.request_signer, None)
            .await?
            .log()
            .await?
            .send()
            .await?
            .log()
            .await?
            .json_ex::<response::XSTSToken>()
            .await
    }
}
