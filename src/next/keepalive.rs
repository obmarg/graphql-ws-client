use std::{
    future::pending,
    time::{Duration, Instant},
};

use futures::{future::FusedFuture, Future, FutureExt};

use crate::ConnectionCommand;

pub(super) struct KeepAliveSettings {
    /// How often to send a keep alive ping
    pub(super) interval: Option<Duration>,

    /// How many pings must be sent without a pong
    /// before the connection is considered dropped
    pub(super) allowed_failures: usize,
}

pub(super) struct KeepAlive {
    settings: KeepAliveSettings,

    starting_time: Instant,
    state: KeepAliveState,
}

impl KeepAlive {
    pub fn new(settings: KeepAliveSettings) -> Self {
        KeepAlive {
            settings,
            starting_time: Instant::now(),
            state: KeepAliveState::Running,
        }
    }

    pub fn interval(&self) -> Option<Duration> {
        self.settings.interval
    }
}

enum KeepAliveState {
    Running,
    StartedKeepAlive,
    TimingOut { failure_count: usize },
}

impl KeepAlive {
    /// Notifies the keep alive actor of other traffic.
    pub fn kick(&mut self) {
        self.starting_time = Instant::now();
        self.state = KeepAliveState::Running;
    }

    pub fn received_pong(&mut self) {
        self.starting_time = Instant::now();
        self.state = KeepAliveState::Running;
    }

    pub(super) fn run(
        &mut self,
    ) -> impl FusedFuture + Future<Output = Option<ConnectionCommand>> + '_ {
        async move {
            match self.settings.interval {
                Some(duration) => futures_timer::Delay::new(duration).await,
                None => pending::<()>().await,
            }

            match self.state {
                KeepAliveState::Running => {
                    self.state = KeepAliveState::StartedKeepAlive;
                }
                KeepAliveState::StartedKeepAlive => {
                    self.state = KeepAliveState::TimingOut { failure_count: 1 };
                }
                KeepAliveState::TimingOut { failure_count } => {
                    self.state = KeepAliveState::TimingOut {
                        failure_count: failure_count + 1,
                    };
                }
            }

            if self.state.failure_count() > self.settings.allowed_failures {
                // returning None aborts
                return None;
            }

            return Some(ConnectionCommand::Ping);
        }
        .fuse()
    }
}

impl KeepAliveState {
    pub fn failure_count(&self) -> usize {
        match self {
            KeepAliveState::Running | KeepAliveState::StartedKeepAlive => 0,
            KeepAliveState::TimingOut { failure_count } => *failure_count,
        }
    }
}
