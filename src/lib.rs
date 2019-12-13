#[tarpc::service]
pub trait Service {
    async fn write(word: String) -> String;
}