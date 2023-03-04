pub async fn get_mong(connstring: &str) -> Result<mongodb::Client, mongodb::error::Error> {
    let mong_options = mongodb::options::ClientOptions::parse(connstring).await?;
    mongodb::Client::with_options(mong_options)
}
