//! Integration and unit tests for FoodShare Search API client.

use foodshare_api_client::FoodshareClient;
use foodshare_api_client::config::ClientConfig;
use foodshare_api_client::endpoints::VectorSearchRequest;

#[tokio::test]
async fn test_search_request_serialization() {
    let req = VectorSearchRequest {
        query: "fresh sourdough bread".to_string(),
        match_threshold: Some(0.7),
        match_count: Some(5),
        latitude: Some(37.7749),
        longitude: Some(-122.4194),
        radius_km: Some(10.0),
        category: Some("Food".to_string()),
    };

    let serialized = serde_json::to_string(&req).expect("valid json");
    assert!(serialized.contains("fresh sourdough bread"));
    assert!(serialized.contains("37.7749"));
    assert!(serialized.contains("Food"));
}

#[tokio::test]
async fn test_search_client_instantiation() {
    let config = ClientConfig::development();
    let client = FoodshareClient::with_config(config).expect("valid client");
    let search = client.search();
    assert_eq!(
        search.client.config().base_url,
        "http://localhost:54321/functions/v1"
    );
}
