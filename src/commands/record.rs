use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use log::{error, info, warn};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{async_trait, builder::*};
use songbird::{
    CoreEvent, Event, EventContext, EventHandler, TrackEvent,
    model::{
        id::UserId,
        payload::{ClientDisconnect, Speaking},
    },
    packet::Packet,
};

use super::TrackErrorNotifier;
use crate::{COLOR_ERROR, COLOR_OK, UserData};

const WAV_SPEC: hound::WavSpec = hound::WavSpec {
    channels: 2,
    sample_rate: 48000,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
};
const SAMPLES_20MS: usize = (0.02 * 48_000.0) as usize;

pub fn register() -> CreateCommand {
    CreateCommand::new("record").description("Record bot audio to a multi-track flac file.")
}

#[derive(Clone)]
struct Receiver {
    inner: Arc<InnerReceiver>,
}

struct InnerReceiver {
    record: AtomicBool,
    last_tick_was_empty: AtomicBool,
    known_ssrcs: DashMap<u32, UserId>,
    writers: DashMap<u32, hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
    start_time: DateTime<Utc>,
}

impl Receiver {
    pub fn new() -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        Self {
            inner: Arc::new(InnerReceiver {
                record: AtomicBool::new(true),
                last_tick_was_empty: AtomicBool::default(),
                known_ssrcs: DashMap::new(),
                writers: DashMap::new(),
                start_time: chrono::offset::Utc::now(),
            }),
        }
    }

    pub fn start_time(&self) -> DateTime<Utc> {
        self.inner.start_time
    }

    pub fn start_time_formatted(&self) -> String {
        self.inner.start_time.format("%Y-%m-%d_%H-%M").to_string()
    }

    pub fn set_record(&self, v: bool) {
        self.inner.record.store(v, Ordering::Relaxed);
    }

    pub fn get_record(&self) -> bool {
        self.inner.record.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl EventHandler for Receiver {
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if !self.inner.record.load(Ordering::Relaxed) {
            return None;
        }
        use EventContext as Ctx;
        match ctx {
            Ctx::SpeakingStateUpdate(Speaking {
                speaking,
                ssrc,
                user_id,
                ..
            }) => {
                // Discord voice calls use RTP, where every sender uses a randomly allocated
                // *Synchronisation Source* (SSRC) to allow receivers to tell which audio
                // stream a received packet belongs to. As this number is not derived from
                // the sender's user_id, only Discord Voice Gateway messages like this one
                // inform us about which random SSRC a user has been allocated. Future voice
                // packets will contain *only* the SSRC.
                //
                // You can implement logic here so that you can differentiate users'
                // SSRCs and map the SSRC to the User ID and maintain this state.
                // Using this map, you can map the `ssrc` in `voice_packet`
                // to the user ID and handle their audio packets separately.
                info!(
                    "Speaking state update: user {:?} has SSRC {:?}, using {:?}",
                    user_id, ssrc, speaking,
                );

                if let Some(user) = user_id {
                    self.inner.known_ssrcs.insert(*ssrc, *user);
                }
            }
            Ctx::VoiceTick(tick) => {
                let speaking = tick.speaking.len();
                let total_participants = speaking + tick.silent.len();
                let last_tick_was_empty = self.inner.last_tick_was_empty.load(Ordering::SeqCst);

                if speaking == 0 && !last_tick_was_empty {
                    info!("No speakers");

                    self.inner.last_tick_was_empty.store(true, Ordering::SeqCst);
                } else if speaking != 0 {
                    self.inner
                        .last_tick_was_empty
                        .store(false, Ordering::SeqCst);

                    info!("Voice tick ({speaking}/{total_participants} live):");

                    // You can also examine tick.silent to see users who are present
                    // but haven't spoken in this tick.
                    for (ssrc, data) in &tick.speaking {
                        let user_id_str = if let Some(id) = self.inner.known_ssrcs.get(ssrc) {
                            format!("{:?}", *id)
                        } else {
                            "?".into()
                        };

                        // This field should *always* exist under DecodeMode::Decode.
                        // The `else` allows you to see how the other modes are affected.
                        if let Some(decoded_voice) = data.decoded_voice.as_ref() {
                            let voice_len = decoded_voice.len();
                            let audio_str = format!(
                                "first samples from {}: {:?}",
                                voice_len,
                                &decoded_voice[..voice_len.min(5)]
                            );

                            if let Some(packet) = &data.packet {
                                let rtp = packet.rtp();
                                info!(
                                    "\t{ssrc}/{user_id_str}: packet seq {} ts {} -- {audio_str}",
                                    rtp.get_sequence().0,
                                    rtp.get_timestamp().0
                                );
                            } else {
                                info!("\t{ssrc}/{user_id_str}: Missed packet -- {audio_str}");
                            }

                            // 20ms of 16-bit little endian 48khz 2 channel PCM
                            if let Some(mut writer) = self.inner.writers.get_mut(ssrc) {
                                for s in decoded_voice {
                                    writer.write_sample(*s).unwrap()
                                }
                            } else {
                                let t = self.start_time_formatted();
                                let p = format!("recordings/{t}/{ssrc}.wav");
                                info!("{}", p);
                                std::fs::create_dir_all(t).unwrap();
                                let mut writer = hound::WavWriter::create(p, WAV_SPEC).unwrap();
                                for s in decoded_voice {
                                    writer.write_sample(*s).unwrap()
                                }
                                self.inner.writers.insert(*ssrc, writer);
                            }
                        } else {
                            info!("\t{ssrc}/{user_id_str}: Decode disabled.");
                        }
                    }
                }
                for ssrc in &tick.silent {
                    if let Some(mut writer) = self.inner.writers.get_mut(ssrc) {
                        for _ in 0..SAMPLES_20MS {
                            writer.write_sample(0).unwrap();
                        }
                    } else {
                        let t = self.start_time_formatted();
                        let p = format!("recordings/{t}/{ssrc}.wav");
                        info!("{}", p);
                        std::fs::create_dir_all(t).unwrap();
                        let mut writer = hound::WavWriter::create(p, WAV_SPEC).unwrap();
                        for _ in 0..SAMPLES_20MS {
                            writer.write_sample(0).unwrap();
                        }
                        self.inner.writers.insert(*ssrc, writer);
                    }
                }
            }
            Ctx::RtpPacket(packet) => {
                // An event which fires for every received audio packet,
                // containing the decoded data.
                let rtp = packet.rtp();
                info!(
                    "Received voice packet from SSRC {}, sequence {}, timestamp {} -- {}B long",
                    rtp.get_ssrc(),
                    rtp.get_sequence().0,
                    rtp.get_timestamp().0,
                    rtp.payload().len()
                );
            }
            Ctx::RtcpPacket(data) => {
                // An event which fires for every received rtcp packet,
                // containing the call statistics and reporting information.
                info!("RTCP packet received: {:?}", data.packet);
            }
            Ctx::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                // You can implement your own logic here to handle a user who has left the
                // voice channel e.g., finalise processing of statistics etc.
                // You will typically need to map the User ID to their SSRC; observed when
                // first speaking.

                info!("Client disconnected: user {:?}", user_id);
            }
            _ => {
                // We won't be registering this struct for any more event classes.
                unimplemented!()
            }
        }

        None
    }
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    if !interaction
        .member
        .clone()
        .unwrap()
        .permissions
        .unwrap()
        .administrator()
    {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .color(Colour::new(COLOR_ERROR))
                                .description("You're not big")
                                .title("Error")
                                .timestamp(Timestamp::now()),
                        )
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    // let guild_id = interaction.guild_id.unwrap();
    // let mut typemap = ctx.data.write().await;
    // let data = typemap.get_mut::<UserData>().unwrap();
    // let track = data.track_handles.get_mut(&guild_id).unwrap();

    // let _ = track.stop();
    // let _ = data.songbird.remove(guild_id).await;

    let (guild_id, channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let user = interaction.user.id;

        if let Some(g) = ctx.cache.guild(guild_id) {
            let vs = g.voice_states.get(&user).unwrap();

            (guild_id, vs.channel_id.unwrap())
        } else {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .color(Colour::new(COLOR_ERROR))
                                .description("Could not get current guild information")
                                .title("Error")
                                .timestamp(Timestamp::now()),
                        ),
                    ),
                )
                .await?;

            // TODO: guild err
            error!("guild none");
            return Ok(());
        }
    };

    let mut typemap = ctx.data.write().await;
    let data = typemap.get_mut::<UserData>().unwrap();
    let manager = &data.songbird;

    let call = manager.join(guild_id, channel_id).await;
    if let Ok(handler_lock) = call {
        let mut handler = handler_lock.lock().await;
        handler.remove_all_global_events();

        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        let _ = handler.deafen(false).await;

        let evt_receiver = Receiver::new();
        if data.is_recording {
            evt_receiver.set_record(false);
            data.is_recording = false;
        } else {
            evt_receiver.set_record(true);
            data.is_recording = true;
        }

        handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::RtpPacket.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::RtcpPacket.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::ClientDisconnect.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::VoiceTick.into(), evt_receiver);
    } else if let Err(e) = call {
        warn!("{}", e);
    }

    // if let Ok(handler_lock) = manager.join(guild_id, channel_id).await {
    if manager.join(guild_id, channel_id).await.is_ok() {
        // check_msg(
        //     msg.channel_id
        //         .say(&ctx.http, &format!("Joined {}", channel_id.mention()))
        //         .await,
        // );
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_OK))
                            .title(format!(
                                "Recording {}",
                                if data.is_recording { "On" } else { "Off" }
                            ))
                            .timestamp(Timestamp::now()),
                    ),
                ),
            )
            .await?;
    } else {
        // Although we failed to join, we need to clear out existing event handlers on the call.
        _ = manager.remove(guild_id).await;

        // check_msg(
        //     msg.channel_id
        //         .say(&ctx.http, "Error joining the channel")
        //         .await,
        // );
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_ERROR))
                            .title("Error joining")
                            .timestamp(Timestamp::now()),
                    ),
                ),
            )
            .await?;
    }

    // interaction
    //     .create_response(
    //         ctx,
    //         CreateInteractionResponse::Message(
    //             CreateInteractionResponseMessage::new().embed(
    //                 CreateEmbed::new()
    //                     .color(Colour::new(COLOR_OK))
    //                     .title("a")
    //                     .timestamp(Timestamp::now()),
    //             ),
    //         ),
    //     )
    //     .await?;

    Ok(())
}
