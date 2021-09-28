use std::sync::Arc;

use serenity::{
    client::Context,
    model::{guild::Guild, id::ChannelId},
};
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
    Call,
};
use tokio::sync::RwLock;

pub struct Queue {
    sources: Vec<String>,
    channel_id: ChannelId,
    voice: Option<Arc<serenity::prelude::Mutex<Call>>>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            channel_id: ChannelId(0),
            voice: None,
        }
    }

    pub async fn join_channel(&mut self, context: &Context, guild: Guild, channel_id: ChannelId) {
        let manager = songbird::get(context)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        let handler_lock = manager.join(guild.id, channel_id).await.0;

        self.channel_id = channel_id;
        self.voice = Some(handler_lock);
    }

    pub async fn add_to_queue(&mut self, path: String) {
        if self.sources.is_empty() {
            self.play_song(path.clone()).await;
        }

        self.sources.push(path);

        println!("{:?}", self.sources);
    }

    pub async fn play_song(&mut self, source: String) {
        match &mut self.voice {
            Some(voice) => {
                let source = Compressed::new(
                    input::ffmpeg(format!("./{}", &source))
                        .await
                        .expect("Link may be dead."),
                    Bitrate::Auto,
                )
                .expect("These parameters are well-defined");

                let song = voice.lock().await.play_only_source(source.into());
            }
            None => {}
        }
    }
}

pub struct QueueData;

impl serenity::prelude::TypeMapKey for QueueData {
    type Value = Arc<RwLock<Queue>>;
}
