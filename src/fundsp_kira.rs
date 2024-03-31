//! Stuff to connect fundsp and kira
pub mod spatial;

use bevy::prelude::*;
use fundsp::prelude::*;
use generic_array::{
    functional::FunctionalSequence as _, sequence::GenericSequence, typenum::U512, ArrayLength,
    GenericArray,
};
use kira::{
    manager::{
        backend::{Backend, DefaultBackend},
        AudioManager, AudioManagerSettings, Capacities,
    },
    sound::{Sound, SoundData},
    track::{TrackBuilder, TrackHandle},
};
use std::{
    any::Any,
    collections::HashMap,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

// TODO Make tweenable volume per-track
// TODO Allow for emitter tracks (tracks that are positioned at a location)
// TODO Make dynamic tracks (optional)

pub struct FundspAudioPlugin;

#[derive(Resource)]
pub struct DefaultTrack;

pub type DefaultBufferLength = U512;
pub type MainTrack = Track<DefaultTrack, DefaultBufferLength>;

impl Plugin for FundspAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<FundspAudioOutput>()
            .init_asset::<Machine>()
            .add_track::<DefaultTrack, DefaultBufferLength>(None);
    }
}

pub trait FundspAudioApp {
    fn add_track<T: Resource, N: ArrayLength>(
        &mut self,
        subchannel_id: Option<usize>,
    ) -> &mut Self {
        self.add_track_with_sample_rate::<T, N>(DEFAULT_SR, subchannel_id)
    }

    fn add_track_with_sample_rate<T: Resource, N: ArrayLength>(
        &mut self,
        sample_rate: f64,
        subtrack_id: Option<usize>,
    ) -> &mut Self;
}
/// Labels for audio systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum FundspAudioSystemSet {
    /// Label for systems in [`CoreStage::PreUpdate`] that clean up tracked audio instances
    // InstanceCleanup,
    /// Label for system in [`CoreStage::PostUpdate`] that processes audio commands for dynamic channels
    // PlayDynamicChannels,
    /// Label for systems in [`CoreStage::PostUpdate`] that process audio commands for typed channels
    PlayTypedChannels,
}

impl FundspAudioApp for App {
    fn add_track_with_sample_rate<T: Resource, N: ArrayLength>(
        &mut self,
        sample_rate: f64,
        subchannel_id: Option<usize>,
    ) -> &mut Self {
        if let Some(subtrack_id) = subchannel_id {
            self.add_systems(
                Startup,
                move |mut commands: Commands, mut output: NonSendMut<FundspAudioOutput>| {
                    if let Some(handle) = output.sub_channels.get(&subtrack_id) {
                        commands.insert_resource(Track::<T, N> {
                            sample_rate,
                            output: kira::OutputDestination::Track(handle.into()),
                            ..default()
                        });
                    } else if let Some(manager) = output.manager.as_mut() {
                        // TODO Expose TrackRoutes::parent and TrackRouters::with_routes
                        // Maybe to go ham and like use the whole kira
                        let new_track = match manager.add_sub_track(TrackBuilder::default()) {
                            Ok(handle) => {
                                debug!("Created new subtrack for subtrack id {subtrack_id}");
                                let new_track = Track::<T, N> {
                                    sample_rate,
                                    output: kira::OutputDestination::Track(handle.id()),
                                    ..default()
                                };
                                output.sub_channels.insert(subtrack_id, handle);
                                new_track
                            }
                            Err(e) => {
                                warn!("Failed to create subtrack: {e}, routing to main");
                                Track::<T, N> {
                                    sample_rate,
                                    ..default()
                                }
                            }
                        };
                        commands.insert_resource(new_track);
                    } else {
                        // Our manager doesn't exist, so we *can't* create tracks
                        debug!("No manager, so tracks can't exist")
                    }
                },
            );
        } else {
            self.add_systems(Startup, move |mut commands: Commands| {
                commands.insert_resource(Track::<T, N> {
                    sample_rate,
                    ..default()
                })
            });
        }
        self.add_systems(
            PostUpdate,
            load_next_machine::<T, N>.in_set(FundspAudioSystemSet::PlayTypedChannels),
        )
    }
}

fn load_next_machine<T: Resource, N: ArrayLength>(
    mut output: NonSendMut<FundspAudioOutput>,
    mut track: ResMut<Track<T, N>>,
    machines: Res<Assets<Machine>>,
) {
    if let Some(machine) = track.next_machine.take().and_then(|hnd| machines.get(hnd)) {
        output.play(machine, &mut *track);
    }
}

/// This resource is used to configure the audio backend at creation
///
/// It needs to be inserted before adding the [`FundspAudioPlugin`] and will be
/// consumed by it. Settings cannot be changed at run-time!
#[derive(Resource)]
pub struct FundspBackendSettings {
    /// The number of commands that can be sent to the audio backend at a time.
    ///
    /// Each action you take, like playing or pausing a sound
    /// queues up one command.
    ///
    /// Note that configuring a channel will cause one command per sound in the channel!
    // NOTE We only allow 1 "sound" per channel.
    command_capacity: usize,
    /// The maximum number of sounds that can be playing at a time.
    // TODO Check if this ends up being equal to the max number of channels
    sound_capacity: usize,
}

impl Default for FundspBackendSettings {
    fn default() -> Self {
        Self {
            command_capacity: 128,
            sound_capacity: 128,
        }
    }
}

impl From<FundspBackendSettings> for AudioManagerSettings<DefaultBackend> {
    fn from(settings: FundspBackendSettings) -> Self {
        AudioManagerSettings {
            capacities: Capacities {
                command_capacity: settings.command_capacity,
                sound_capacity: settings.sound_capacity,
                ..default()
            },
            ..default()
        }
    }
}

pub struct FundspAudioOutput<B: Backend = DefaultBackend> {
    manager: Option<AudioManager<B>>,
    sub_channels: HashMap<usize, TrackHandle>,
}

impl FromWorld for FundspAudioOutput {
    fn from_world(world: &mut World) -> Self {
        let settings = world
            .remove_resource::<FundspBackendSettings>()
            .unwrap_or_default();
        let manager = AudioManager::new(settings.into());
        if let Err(ref setup_err) = manager {
            warn!("failed to setup up fundsp audio: {setup_err:?}")
        }

        Self {
            manager: manager.ok(),
            sub_channels: HashMap::new(),
        }
    }
}

// TODO Take bevy_kira_audio's ideas of channels and run with it
// A `Machined` with continue forever until:
// - the output it produces (on both channels!) is below the noise floor
// (I'm using a RMS < 0.001 to mean "below the noise floor")
// - the channel it is associated with gets a new `Machined` to play.
//
// this means that yes, a channel will only be able to play 1 `Machine` at a time.
// But you can create as many channels as you want.
impl FundspAudioOutput {
    // TODO Make this called from some systems, and provide a way to
    // *queue* these commands, so that external systems aren't touching
    // the *non-Send* Self (FundspAudioOutput)
    fn play<T: Resource, N: ArrayLength>(&mut self, machine: &Machine, track: &mut Track<T, N>) {
        if let Some(manager) = self.manager.as_mut() {
            // Stop the previous sound
            if let Some(handle) = track.active_handle.take() {
                handle.unload();
            }
            // Play the new sound
            // TODO Should this be queued?
            let machined = Machined::<T, N>::new(
                machine.machine.clone(),
                MachinedSettings {
                    output: track.output,
                    noise_floor: machine.noise_floor,
                },
                track.sample_rate,
                track.trackable.clone(),
            );
            track.active_handle = manager
                .play(machined)
                .inspect_err(|err| error!("{err}"))
                .ok();
        }
    }
}

#[derive(Debug, Clone)]
struct ChannelDetails<N: ArrayLength> {
    rms: GenericArray<Arc<AtomicU32>, N>,
    samples: GenericArray<Arc<AtomicU32>, N>,
}

impl<N: ArrayLength> ChannelDetails<N> {
    fn store_sample(&self, sample: f32) {
        for i in 0..(N::to_usize() - 1) {
            self.samples[i].swap(
                self.samples[i + 1].load(Ordering::Acquire),
                Ordering::AcqRel,
            );
        }
        self.samples[N::to_usize() - 1].store(sample.to_bits(), Ordering::Release);
    }

    fn store_rms(&self, rms: f32) {
        for i in 0..(N::to_usize() - 1) {
            self.rms[i].swap(self.rms[i + 1].load(Ordering::Acquire), Ordering::AcqRel);
        }
        self.rms[N::to_usize() - 1].store(rms.to_bits(), Ordering::Release);
    }

    fn samples(&self) -> GenericArray<f32, N> {
        <GenericArray<_, N>>::clone(&self.samples)
            .map(|bits| f32::from_bits(bits.load(Ordering::Acquire)))
    }

    fn rms(&self) -> GenericArray<f32, N> {
        <GenericArray<_, N>>::clone(&self.rms)
            .map(|bits| f32::from_bits(bits.load(Ordering::Acquire)))
    }

    fn empty_atomics() -> GenericArray<Arc<AtomicU32>, N> {
        GenericArray::generate(|_| Arc::new(AtomicU32::new(0.0f32.to_bits())))
    }
}

impl<N: ArrayLength> Default for ChannelDetails<N> {
    fn default() -> Self {
        Self {
            rms: Self::empty_atomics(),
            samples: Self::empty_atomics(),
        }
    }
}

#[derive(Debug)]
struct Trackable<T, N: ArrayLength> {
    left_channel: ChannelDetails<N>,
    right_channel: ChannelDetails<N>,
    _marker: PhantomData<T>,
}

impl<T, N: ArrayLength> Trackable<T, N> {
    fn buffer_length(&self) -> usize {
        N::to_usize()
    }
}

impl<T, N: ArrayLength> Clone for Trackable<T, N>
where
    T: Resource,
{
    fn clone(&self) -> Self {
        Self {
            left_channel: self.left_channel.clone(),
            right_channel: self.right_channel.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, N: ArrayLength> Default for Trackable<T, N> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            left_channel: default(),
            right_channel: default(),
        }
    }
}

#[derive(Resource)]
pub struct Track<T, N: ArrayLength> {
    trackable: Trackable<T, N>,
    output: kira::OutputDestination,
    sample_rate: f64,
    active_handle: Option<MachinedHandle>,
    next_machine: Option<Handle<Machine>>,
}

impl<T, N: ArrayLength> Default for Track<T, N> {
    fn default() -> Self {
        Self {
            trackable: Trackable::default(),
            output: kira::OutputDestination::MAIN_TRACK,
            sample_rate: DEFAULT_SR,
            active_handle: None,
            next_machine: None,
        }
    }
}

impl<T, N: ArrayLength> Track<T, N> {
    /// Get this track's sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Get the most recent sample values
    pub fn samples(&self) -> GenericArray<(f32, f32), N> {
        let left_samples = self.trackable.left_channel.samples();
        let right_samples = self.trackable.right_channel.samples();
        left_samples.zip(right_samples, |l, r| (l, r))
    }

    /// Get the most recent RMS values
    pub fn rms(&self) -> GenericArray<(f32, f32), N> {
        let left_rms = self.trackable.left_channel.rms();
        let right_rms = self.trackable.right_channel.rms();
        left_rms.zip(right_rms, |l, r| (l, r))
    }

    pub fn play(&mut self, machine: Handle<Machine>) {
        self.next_machine = Some(machine);
    }

    pub fn buffer_length(&self) -> usize {
        self.trackable.buffer_length()
    }
}

#[derive(Asset, Clone, TypePath)]
pub struct Machine {
    /// The fundsp audio program
    pub machine: Box<dyn AudioUnit32>,
    /// The desired noise floor (RMS)
    pub noise_floor: f32,
    /// Userdata (generally how the Machine was created)
    // generally immutable, after all, the Machine has been made
    userdata: Option<Arc<dyn Any + Send + Sync>>,
}

impl Machine {
    pub const DEFAULT_NOISE_FLOOR: f32 = 0.001;

    pub fn new<T: AudioUnit32 + 'static>(machine: T) -> Self {
        Self::with_noise_floor(machine, Self::DEFAULT_NOISE_FLOOR)
    }

    pub fn with_noise_floor<T: AudioUnit32 + 'static>(machine: T, noise_floor: f32) -> Self {
        Self {
            machine: Box::new(machine),
            noise_floor,
            userdata: None,
        }
    }

    pub fn with_userdata<U: Any + Send + Sync>(self, userdata: U) -> Self {
        Self {
            userdata: Some(Arc::new(userdata)),
            ..self
        }
    }

    pub fn userdata(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.userdata.as_ref()
    }
}

// TODO This should be associated with a track n stuff.
pub struct Machined<T, N: ArrayLength> {
    node: Box<dyn AudioUnit32>,
    sample_rate: f64,
    settings: MachinedSettings,
    handle: MachinedHandle,
    trackable: Trackable<T, N>,
}

// TODO This should be determined by the track
#[derive(Clone, Debug, Default)]
struct MachinedSettings {
    output: kira::OutputDestination,
    noise_floor: f32,
}

#[derive(Clone, Debug)]
pub struct MachinedHandle {
    should_unload: Arc<AtomicBool>,
}

impl Default for MachinedHandle {
    fn default() -> Self {
        Self {
            should_unload: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl MachinedHandle {
    /// Stops any sounds associated with this [`Trackable`]
    fn unload(&self) {
        self.should_unload.store(true, Ordering::SeqCst)
    }
}

impl<T, N: ArrayLength> Machined<T, N>
where
    T: Resource,
{
    fn new(
        mut node: Box<dyn AudioUnit32>,
        settings: MachinedSettings,
        sample_rate: f64,
        trackable: Trackable<T, N>,
    ) -> Self {
        // Set the node sample rate from the track
        node.set_sample_rate(sample_rate);

        Self {
            node,
            sample_rate,
            settings,
            handle: MachinedHandle::default(),
            trackable,
        }
    }
}

impl<Track, N> SoundData for Machined<Track, N>
where
    Track: Resource,
    N: ArrayLength,
{
    type Error = NoAudioOutputs;
    // TODO Make this an actual handle that can control the state of the FundspSound
    type Handle = MachinedHandle;

    fn into_sound(mut self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        self.node.allocate();
        Ok((
            Box::new(FundspSound::new(
                self.node,
                self.sample_rate,
                self.settings,
                self.trackable,
                self.handle.clone(),
            )?),
            self.handle,
        ))
    }
}

pub struct FundspSound<Track, N: ArrayLength> {
    node: Box<dyn AudioUnit32>,
    sample_time: Duration,
    elapsed: Duration,

    settings: MachinedSettings,
    rms_left: Shared<f32>,
    rms_right: Shared<f32>,
    monitor_left: An<Monitor<f32>>,
    monitor_right: An<Monitor<f32>>,

    trackable: Trackable<Track, N>,
    handle: MachinedHandle,

    buffer: [kira::dsp::Frame; 4],
}

#[derive(thiserror::Error, Debug)]
#[error("No audio outputs from given AudioUnit32")]
pub struct NoAudioOutputs;

impl<Track, N: ArrayLength> FundspSound<Track, N> {
    #[inline(always)]
    fn make_frame((left, right): (f32, f32)) -> kira::dsp::Frame {
        kira::dsp::Frame { left, right }
    }

    fn new(
        mut node: Box<dyn AudioUnit32>,
        sample_rate: f64,
        settings: MachinedSettings,
        trackable: Trackable<Track, N>,
        handle: MachinedHandle,
    ) -> Result<Self, NoAudioOutputs> {
        let rms_left = Shared::new(0.0);
        let rms_right = Shared::new(0.0);

        // to start a new sound, fast-forward (and throw away)
        // latency signals
        if let Some(mut latency) = node.latency() {
            while latency >= 1.0 {
                latency -= 1.0;
                _ = node.get_stereo();
            }
        } else {
            // Return an error!
            return Err(NoAudioOutputs);
        }

        Ok(FundspSound {
            sample_time: Duration::from_secs_f64(sample_rate.recip()),
            elapsed: Duration::ZERO,
            settings,
            monitor_left: monitor(&rms_left, Meter::Rms(0.1)),
            monitor_right: monitor(&rms_right, Meter::Rms(0.1)),

            rms_left,
            rms_right,

            trackable,
            handle,
            // Get the first four frames
            buffer: [
                // prev
                kira::dsp::Frame::ZERO,
                // current
                Self::make_frame(node.get_stereo()),
                // next_1
                Self::make_frame(node.get_stereo()),
                // next_2
                Self::make_frame(node.get_stereo()),
            ],
            node,
        })
    }
}

impl<Track, N> Sound for FundspSound<Track, N>
where
    Track: Send,
    N: ArrayLength,
{
    fn output_destination(&mut self) -> kira::OutputDestination {
        self.settings.output
    }

    fn process(
        &mut self,
        dt: f64,
        _clock_info_provider: &kira::clock::clock_info::ClockInfoProvider,
        _modulator_value_provider: &kira::modulator::value_provider::ModulatorValueProvider,
    ) -> kira::dsp::Frame {
        // sample rate check
        let frame = kira::dsp::interpolate_frame(
            self.buffer[0],
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
            (self.elapsed.as_secs_f32() / self.sample_time.as_secs_f32()).clamp(0.0, 1.0),
        );
        self.elapsed += Duration::from_secs_f64(dt);
        while self.elapsed > self.sample_time {
            self.elapsed -= self.sample_time;
            // push in a new frame, by shifting over everything else
            for i in 0..self.buffer.len() - 1 {
                self.buffer[i] = self.buffer[i + 1];
            }
            self.buffer[self.buffer.len() - 1] = Self::make_frame(self.node.get_stereo());
        }
        // Process samples (from frame)
        // Monitor samples
        self.monitor_left.filter_mono(frame.left);
        self.monitor_right.filter_mono(frame.right);
        // Report sample
        self.trackable.left_channel.store_sample(frame.left);
        self.trackable.right_channel.store_sample(frame.right);
        frame
    }

    fn finished(&self) -> bool {
        let left_rms = self.rms_left.value();
        let right_rms = self.rms_right.value();
        let noise_floor = self.settings.noise_floor;
        trace!("RMS: {left_rms} {right_rms} NF {noise_floor}");

        // Report RMS
        self.trackable.left_channel.store_rms(left_rms);
        self.trackable.right_channel.store_rms(right_rms);

        // If we are quieter (on both channels!!) than the noise floor, assume we
        // are finished an want to stop
        let below_noise_floor = left_rms < noise_floor && right_rms < noise_floor;
        if below_noise_floor {
            debug!("Stopping {self:p} due to being quieter than the noise floor!");
        }

        let should_unload = self.handle.should_unload.load(Ordering::Acquire);
        if should_unload {
            debug!("Stopping {self:p} due to unload!")
        }

        below_noise_floor || should_unload
    }
}
