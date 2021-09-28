mod queue;
use std::sync::Arc;

use queue::{Queue, QueueData};
use songbird::SerenityInit;

// Import the `Context` to handle commands.
use serenity::client::Context;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            CommandResult,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    Result as SerenityResult,
};
use tokio::sync::RwLock;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(play, stop)]
struct General;

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = "ODkxODUyNjA5Mjg2MTkzMTgy.YVEYdw.P9sY3Ui55JNueu9IyjTDMwU9SdY";

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    client
        .data
        .write()
        .await
        .insert::<QueueData>(Arc::new(RwLock::new(Queue::new())));

    let _ = client
        .start()
        .await
        .map_err(|why| println!("Client ended: {:?}", why));
}

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message) -> CommandResult {
    let queue_lock = {
        let data = ctx.data.read().await;

        data.get::<QueueData>()
            .expect("Expected QueueData in TypeMap.")
            .clone()
    };

    let arguments = msg.content.split(" ").collect::<Vec<&str>>();

    let guild = msg.guild(&ctx.cache).await.unwrap();

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let url = arguments[1];

    check_msg(
        msg.reply(ctx, format!("Downloading video with search term `{}`", url))
            .await,
    );

    let video = match download_video(url).await {
        Some(str) => str,
        None => {
            check_msg(
                msg.reply(ctx, "Failed to download the provided term.")
                    .await,
            );
            return Ok(());
        }
    };

    check_msg(
        msg.reply(
            ctx,
            "Finished downloading video, attempting to play source.",
        )
        .await,
    );

    let mut queue = queue_lock.write().await;

    queue.join_channel(ctx, guild, connect_to).await;
    queue.add_to_queue(ctx, video).await;

    Ok(())
}

#[command]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let _ = manager.leave(guild_id).await;

    check_msg(
        msg.reply(ctx, format!("I have stopped playing `{}`", "song"))
            .await,
    );

    Ok(())
}

async fn download_video(term: &str) -> Option<String> {
    if term.contains("https://") {
        return match rustube::download_best_quality(term).await {
            Ok(path) => Some(path.into_os_string().into_string().unwrap()),
            Err(_) => None,
        };
    } else {
    }

    return None;
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
