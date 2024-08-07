use async_trait::async_trait;
use hmac::{Hmac, Mac};
use rand::random;
use reqwest::Client;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use xb_types::{ExchangeOrderExecutor, PendingOrder};

const BASE_URL: &str = "https://www.lbkex.net";

pub struct LBankClient {
    api_key: String,
    secret_key: String,
    client: Client,
}

impl LBankClient {
    pub fn new(api_key: String, secret_key: String) -> LBankClient {
        LBankClient {
            api_key,
            secret_key,
            client: Client::new(),
        }
    }

    async fn post_request(&self, path: &str, mut params: BTreeMap<&'static str, String>) {
        params.insert("api_key", self.api_key.clone());
        params.insert("echostr", generate_echostr());
        params.insert("signature_method", "HmacSHA256".to_string());
        params.insert(
            "timestamp",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string(),
        );

        let mut query = String::new();
        for (key, value) in params {
            push_query_param(&mut query, key, value.as_str());
        }

        let sig = self.get_signature(&query);
        push_query_param(&mut query, "sign", &sig);

        let response = self
            .client
            .post(format!("{BASE_URL}{path}?{query}"))
            .header("contentType", "application/x-www-form-urlencoded")
            .send()
            .await
            .unwrap();

        info!("LBank: Response: {response:?}");

        let content = response.text().await.unwrap();

        info!("LBank: Response content: {content}");
    }

    fn get_signature(&self, query: &str) -> String {
        info!("Signing: {query}");
        let md5_digest = md5::compute(query.as_bytes());
        let input = format!("{md5_digest:X}");

        let mut hmac: Hmac<Sha256> = Hmac::new_from_slice(self.secret_key.as_bytes()).unwrap();
        hmac.update(input.as_bytes());
        let sig_bytes = hmac.finalize().into_bytes();
        hex::encode(sig_bytes)
    }
}

#[async_trait]
impl ExchangeOrderExecutor for LBankClient {
    async fn submit_order(&self, order: PendingOrder) -> Result<String, String> {
        let mut params = BTreeMap::new();
        params.insert("symbol", "chat_usdt".to_string());
        params.insert("amount", order.amount().to_string());

        match order {
            PendingOrder::Limit(o) => {
                params.insert(
                    "type",
                    (if o.direction.is_buy() {
                        "buy_maker"
                    } else {
                        "sell_maker"
                    })
                    .to_string(),
                );
                params.insert("price", o.price.to_string());
            }
            PendingOrder::Market(o) => {
                params.insert(
                    "type",
                    (if o.direction.is_buy() {
                        "buy_market"
                    } else {
                        "sell_market"
                    })
                    .to_string(),
                );
            }
        }

        self.post_request("/v2/supplement/create_order.do", params)
            .await;

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

fn generate_echostr() -> String {
    let bytes: [u8; 16] = random();
    hex::encode(bytes)
}
