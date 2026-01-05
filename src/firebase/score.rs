use crate::firebase::get_firebase_instance;

pub async fn score_already_saved(identifier: &String) -> bool {
    match get_firebase_instance().at("checked_scores").at(identifier).get::<bool>().await {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub async fn insert_score(identifier: &String) {
    get_firebase_instance().at("checked_scores").set_with_key(identifier, &true).await.unwrap();
}