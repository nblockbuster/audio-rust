use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler};

pub mod r#loop;
// pub mod pause;
pub mod disconnect;
pub mod pause;
pub mod play;
pub mod record;
pub mod stop;
pub mod volume;

struct TrackErrorNotifier;

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}
