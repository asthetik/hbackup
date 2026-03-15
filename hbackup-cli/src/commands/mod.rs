use crate::Result;

pub mod add;

pub trait ProcessCommand {
    async fn run(self) -> Result<()>;
}
