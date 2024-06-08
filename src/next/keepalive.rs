use std::{future::pending, time::Duration};

use futures_lite::{stream, Stream};

use crate::ConnectionCommand;

#[derive(Clone)]
pub(super) struct KeepAliveSettings {
    /// How often to send a keep alive ping
    pub(super) interval: Option<Duration>,

    /// How many pings can be sent without receiving a reply before
    /// the connection is considered dropped
    pub(super) retries: usize,
}

impl Default for KeepAliveSettings {
    fn default() -> Self {
        Self {
            interval: None,
            retries: 3,
        }
    }
}

enum KeepAliveState {
    Running,
    StartedKeepAlive,
    TimingOut { failure_count: usize },
}

impl KeepAliveSettings {
    pub(super) fn run(&self) -> impl Stream<Item = ConnectionCommand> + 'static {
        let settings = self.clone();

        stream::unfold(KeepAliveState::Running, move |mut state| async move {
            match settings.interval {
                Some(duration) => futures_timer::Delay::new(duration).await,
                None => pending::<()>().await,
            }

            match state {
                KeepAliveState::Running => {
                    state = KeepAliveState::StartedKeepAlive;
                }
                KeepAliveState::StartedKeepAlive => {
                    state = KeepAliveState::TimingOut { failure_count: 0 };
                }
                KeepAliveState::TimingOut { failure_count } => {
                    state = KeepAliveState::TimingOut {
                        failure_count: failure_count + 1,
                    };
                }
            }

            if state.failure_count() > settings.retries {
                // returning None aborts
                return None;
            }

            Some((ConnectionCommand::Ping, state))
        })
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
