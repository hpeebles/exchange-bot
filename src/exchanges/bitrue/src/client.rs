use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use xb_types::{ExchangeOrderExecutor, PendingOrder};

const BASE_URL: &str = "https://openapi.bitrue.com";

pub struct BitrueClient {
    api_key: String,
    secret_key: String,
    client: Client,
}

impl BitrueClient {
    pub fn new(api_key: String, secret_key: String) -> BitrueClient {
        BitrueClient {
            api_key,
            secret_key,
            client: Client::new(),
        }
    }

    async fn post_request(&self, path: &str, params: BTreeMap<&'static str, String>) {
        let mut query = String::new();
        for (key, value) in params {
            push_query_param(&mut query, key, value.as_str());
        }

        let sig = self.get_signature(&query);
        push_query_param(&mut query, "signature", &sig);

        let response = self
            .client
            .post(format!("{BASE_URL}{path}?{query}"))
            .header("contentType", "application/x-www-form-urlencoded")
            .header("X-MBX-APIKEY", self.api_key.clone())
            .send()
            .await
            .unwrap();

        info!("Bitrue: Response: {response:?}");

        let content = response.text().await.unwrap();

        info!("Bitrue: Response content: {content}");
    }

    fn get_signature(&self, query: &str) -> String {
        info!("Signing: {query}");
        let mut hmac: Hmac<Sha256> = Hmac::new_from_slice(self.secret_key.as_bytes()).unwrap();
        hmac.update(query.as_bytes());
        let sig_bytes = hmac.finalize().into_bytes();
        hex::encode(sig_bytes)
    }
}

#[async_trait]
impl ExchangeOrderExecutor for BitrueClient {
    async fn submit_order(&self, order: PendingOrder) -> Result<String, String> {
        let mut params = BTreeMap::new();
        params.insert("symbol", "chatusdt".to_string());
        params.insert("quantity", order.amount().to_string());
        params.insert(
            "side",
            (if order.direction().is_buy() {
                "BUY"
            } else {
                "SELL"
            })
            .to_string(),
        );
        params.insert(
            "timestamp",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string(),
        );

        match order {
            PendingOrder::Limit(o) => {
                params.insert("type", "LIMIT".to_string());
                params.insert("price", o.price.to_string());
            }
            PendingOrder::Market(_) => {
                params.insert("type", "MARKET".to_string());
            }
        }

        self.post_request("/api/v1/order", params).await;

        todo!()
    }
}

fn push_query_param(q: &mut String, key: &str, value: &str) {
    if !q.is_empty() {
        q.push('&');
    }
    q.push_str(key);
    q.push('=');
    q.push_str(value);
}
