use serde::Serialize;

mod subscriber;

pub use subscriber::BitrueSubscriber;

fn serialize_to_json<S: Serialize>(value: &S) -> String {
    serde_json::to_string(value).unwrap()
}
