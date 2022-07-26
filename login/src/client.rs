use crate::{
    db::Db, handler::Handler, login::queries, net::connection::Connection, shutdown::Shutdown,
    Shared,
};
use oxide_core::Result;
use std::sync::Arc;

pub struct Client {
    pub session_id: usize,
    pub db: Db,
    pub connection: Connection,
    pub shutdown: Shutdown,
    pub shared: Arc<Shared>,
    pub login_attempts: u8,
    pub pin_attempts: u8,
    pub pic_attempts: u8,
    pub pin: Option<String>,
    pub pic: Option<String>,
    pub id: Option<i32>,
    pub world_id: Option<i32>,
    pub channel_id: Option<i32>,
}

impl Client {
    pub async fn connect(&mut self) -> Result<()> {
        log::info!("Client connected to server (session: {})", self.session_id);

        while !self.shutdown.is_shutdown() {
            let maybe_packet = tokio::select! {
                res = self.connection.read_packet() => res?,
                // shutdown signal was receieved, return and terminate the task
                _ = self.shutdown.recv() => {
                    return Ok(())
                }
            };

            // None => client disconnected
            let packet = match maybe_packet {
                Some(packet) => packet,
                None => return Ok(()),
            };

            log::debug!("Received packet: {}", packet);

            let handler = match Handler::get(packet) {
                Some(handler) => handler,
                None => return Ok(()),
            };

            handler.handle(self).await?;
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        log::info!("Client disconnecting (session: {})", self.session_id);
        self.connection.disconnect().await?;
        Ok(())
    }

    pub async fn on_disconnect(&mut self) -> Result<()> {
        if let Some(id) = self.id {
            queries::update_login_state(id, 0, &self.db).await?;
        }

        Ok(())
    }
}
