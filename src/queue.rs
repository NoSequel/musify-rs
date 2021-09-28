use std::sync::Arc;

use serenity::{
    async_trait,
    client::Context,
    model::{guild::Guild, id::ChannelId},
};
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
    Call, Event, EventContext, EventHandler, TrackEvent,
};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Queue {
    pub sources: Vec<String>,
    channel_id: ChannelId,
    currently_playing: bool,
    voice: Option<Arc<serenity::prelude::Mutex<Call>>>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            channel_id: ChannelId(0),
            currently_playing: false,
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

    pub async fn add_to_queue(&mut self, context: &Context, path: String) {
        self.sources.push(path);

        if !self.currently_playing {
            self.play_newest_in_queue(context).await;
        }

        println!("{:?}", self.sources);
    }

    pub async fn play_newest_in_queue(&mut self, context: &Context) {
        self.play_song(context, (&self.sources.clone()[0]).to_owned())
            .await;
    }

    pub async fn play_song(&mut self, context: &Context, source: String) {
        match &mut self.voice {
            Some(voice) => {
                self.sources.remove(
                    self.sources
                        .iter()
                        .position(|string| string == &source)
                        .unwrap(),
                );

                let source = Compressed::new(
                    input::ffmpeg(format!("./{}", &source))
                        .await
                        .expect("Link may be dead."),
                    Bitrate::Auto,
                )
                .expect("These parameters are well-defined");

                let song = voice.lock().await.play_only_source(source.into());

                self.currently_playing = true;

                song.add_event(
                    Event::Track(TrackEvent::End),
                    QueueEventWrapper {
                        context: Arc::new(context.clone()),
                    },
                )
                .expect("The fuck");
            }
            None => {}
        }
    }
}

struct QueueEventWrapper {
    context: Arc<Context>,
}

#[async_trait]
impl EventHandler for QueueEventWrapper {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let queue_lock = {
            let data = self.context.data.read().await;

            data.get::<QueueData>()
                .expect("Expected QueueData in TypeMap.")
                .clone()
        };

        let mut queue = queue_lock.write().await;

        if !queue.sources.is_empty() {
            queue.play_newest_in_queue(&self.context).await;
        }

        None
    }
}

pub struct QueueData;

impl serenity::prelude::TypeMapKey for QueueData {
    type Value = Arc<RwLock<Queue>>;
}
