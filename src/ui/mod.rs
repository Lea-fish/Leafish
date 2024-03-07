use std::{fmt::Debug, sync::Arc};

use leafish_protocol::format::Component;

use crate::{ecs, Game};

mod glow;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum UiImpl {
    Wgpu,
    Glow,
}

pub fn ui_queue(ui: UiImpl) -> Box<dyn UiQueue> {
    match ui {
        UiImpl::Wgpu => todo!(),
        UiImpl::Glow => glow::queue(),
    }
}

pub fn start_ui(game: &Arc<Game>, ui: UiImpl) -> anyhow::Result<()> {
    match ui {
        UiImpl::Wgpu => {
            todo!()
        }
        UiImpl::Glow => {
            glow::run(game)
        }
    }
}

pub fn init_systems(ui: UiImpl, manager: &mut ecs::Manager) -> anyhow::Result<()> {
    match ui {
        UiImpl::Wgpu => {
            todo!()
        }
        UiImpl::Glow => {
            glow::init_systems(manager)
        }
    }
}

#[derive(Clone, Debug)]
pub enum InterUiMessage {
    Disconnected {
        reason: Option<Component>,
    },
    OpenChat,
    CloseChat,
}

pub(crate) type UiQueueSender = Box<dyn Fn(InterUiMessage) + Send + Sync + 'static>;

pub trait UiQueue: Send + Sync + 'static {

    fn send(&self, msg: InterUiMessage);

}

impl<U: Fn(InterUiMessage) + Send + Sync + 'static> UiQueue for U {
    fn send(&self, msg: InterUiMessage) {
        self(msg)
    }
}

impl UiQueue for dyn Fn(InterUiMessage) + Send + Sync + 'static {
    fn send(&self, msg: InterUiMessage) {
        self(msg)
    }
}
