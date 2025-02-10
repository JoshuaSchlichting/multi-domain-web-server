use anyhow::{bail, Context};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::any,
    Router,
};
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
};
use std::collections::HashMap;

pub async fn acme_challenge_response() -> Result<(), anyhow::Error> {
    let acct = create_account().await?;

    let challenges = create_order("domain.com", acct).await?;
    let router = acme_router(challenges);
    let address = "0.0.0.0:5002";
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    // Start the Axum server as a background task, so it's running while we complete the challenge
    // in the next steps.
    tokio::task::spawn(async move { axum::serve(listener, router).await.unwrap() });

    tracing::info!("serving HTTP-01 challenge server at: {}", address);
    Ok(())
}

async fn create_account<'a>() -> Result<Account, anyhow::Error> {
    let account = NewAccount {
        // Optionally add a list of contact URIs (like mailto:info@your-domain.com).
        contact: &[],
        terms_of_service_agreed: true,
        only_return_existing: false,
    };

    // We'll use methods on the returned account struct for future calls to the ACME server.
    let (account, _credentials) = Account::create(&account, &LetsEncrypt::Staging.url(), None)
        .await
        .context("failed to create acme account")?;
    Ok(account)
}

type ChallengeMap = HashMap<String, String>;
async fn create_order<'a>(domain: &str, account: Account) -> Result<ChallengeMap, anyhow::Error> {
    // Using the account we created earlier, create an order for our domain.
    let mut order = account
        .new_order(&NewOrder {
            identifiers: &[Identifier::Dns(domain.to_string())],
        })
        .await
        .context("failed to order certificate")?;

    // Request authorizations for our order from the ACME server.
    let authorizations = order
        .authorizations()
        .await
        .context("failed to retrieve order authorizations")?;

    // There should only be 1 authorization as we only provided 1 domain above.
    let authorization = authorizations
        .first()
        .context("there should be one authorization")?;

    if !matches!(authorization.status, AuthorizationStatus::Pending) {
        bail!("order should be pending");
    }

    // We want to complete an HTTP-01 challenge for this example, so we
    // extract it from the authorization. It holds the token we need to
    // complete the challenge.
    let challenge = authorization
        .challenges
        .iter()
        .find(|c| c.r#type == ChallengeType::Http01)
        .ok_or_else(|| anyhow::anyhow!("no http01 challenge found"))?;
    let challenges = HashMap::from([(
        challenge.token.clone(),
        order.key_authorization(challenge).as_str().to_string(),
    )]);

    Ok(challenges)
}

/// Set up a simple acme server to respond to http01 challenges.
pub fn acme_router(challenges: HashMap<String, String>) -> Router {
    Router::new()
        .route(
            "/.well-known/acme-challenge/{*token}",
            any(http01_challenge),
        )
        .with_state(challenges)
}

/// Respond to HTTP-01 challenges by extracting the token from the path of the request, and then
/// using the token to look up the matching key authorization in our internal state.
pub async fn http01_challenge(
    State(challenges): State<HashMap<String, String>>,
    Path(token): Path<String>,
) -> Result<String, StatusCode> {
    tracing::info!(%token, "received HTTP-01 ACME challenge");

    if let Some(key_auth) = challenges.get(&token) {
        Ok({
            tracing::info!(%key_auth, "responding to ACME challenge");
            key_auth.clone()
        })
    } else {
        tracing::warn!(%token, "didn't find acme challenge");
        Err(StatusCode::NOT_FOUND)
    }
}
