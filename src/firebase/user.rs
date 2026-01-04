use crate::firebase::get_firebase_instance;

pub async fn get_user_skin(osu_user_id: &String) -> Option<String> {
    get_firebase_instance().at("users").at(osu_user_id).at("skin").get::<Option<String>>().await.unwrap()
}

pub async fn save_skin(osu_user_id: &String, skin: &String) {
    get_firebase_instance().at("users").at(osu_user_id).set_with_key("skin", skin).await.unwrap();
}