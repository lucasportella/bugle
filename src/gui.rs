use std::rc::Rc;

use fltk::app;
use fltk::button::{Button, CheckButton, LightButton, ReturnButton};
use fltk::dialog as fltk_dialog;
use fltk::enums::Event;
use fltk::frame::Frame;
use fltk::input::{Input, SecretInput};
use fltk::menu::MenuButton;
use fltk::misc::InputChoice;
use fltk_float::button::{ButtonElement, MenuButtonElement};
use fltk_float::frame::FrameElement;
use fltk_float::input::InputElement;
use fltk_float::misc::InputChoiceElement;
use fltk_float::WrapperFactory;

mod data;
mod dialog;
pub mod glyph;
mod home;
mod launcher;
mod main_menu;
mod mod_manager;
mod mod_update;
mod prelude;
mod server_browser;
mod single_player;
pub mod theme;
mod widgets;

pub use self::dialog::Dialog;
pub use self::home::{HomeAction, HomeUpdate};
pub use self::launcher::LauncherWindow;
pub use self::mod_manager::{ModManagerAction, ModManagerUpdate};
pub use self::mod_update::{ModUpdateProgressDialog, ModUpdateSelectionDialog};
pub use self::server_browser::{ServerBrowserAction, ServerBrowserUpdate};
pub use self::single_player::{SinglePlayerAction, SinglePlayerUpdate};

pub enum Action {
    HomeAction(HomeAction),
    ServerBrowser(ServerBrowserAction),
    SinglePlayer(SinglePlayerAction),
    ModManager(ModManagerAction),
}

pub enum Update {
    HomeUpdate(HomeUpdate),
    ServerBrowser(ServerBrowserUpdate),
    SinglePlayer(SinglePlayerUpdate),
    ModManager(ModManagerUpdate),
}

impl Update {
    pub fn try_consolidate(self, other: Self) -> Result<Update, (Update, Update)> {
        match (self, other) {
            (Self::ServerBrowser(this), Self::ServerBrowser(other)) => {
                Self::consolidation_result(this.try_consolidate(other))
            }
            (this, other) => Err((this, other)),
        }
    }

    fn consolidation_result<U: Into<Update>>(
        result: Result<U, (U, U)>,
    ) -> Result<Update, (Update, Update)> {
        match result {
            Ok(consolidated) => Ok(consolidated.into()),
            Err((this, other)) => Err((this.into(), other.into())),
        }
    }
}

pub trait Handler<A>: Fn(A) -> anyhow::Result<()> {}
impl<A, F: Fn(A) -> anyhow::Result<()>> Handler<A> for F {}

type CleanupFn = Box<dyn FnMut()>;

pub fn alert_error(message: &str, err: &anyhow::Error) {
    fltk_dialog::alert_default(&format!("{}\n{}", message, err));
}

pub fn prompt_confirm(prompt: &str) -> bool {
    fltk_dialog::choice2_default(prompt, "No", "Yes", "")
        .map(|choice| choice == 1)
        .unwrap_or_default()
}

thread_local! {
    static WRAPPER_FACTORY: Rc<WrapperFactory> = {
        use self::widgets::{ReadOnlyText, ReadOnlyTextElement};
        let mut factory = WrapperFactory::new();
        factory.set_wrapper::<Button, ButtonElement<Button>>();
        factory.set_wrapper::<CheckButton, ButtonElement<CheckButton>>();
        factory.set_wrapper::<Frame, FrameElement>();
        factory.set_wrapper::<Input, InputElement<Input>>();
        factory.set_wrapper::<InputChoice, InputChoiceElement>();
        factory.set_wrapper::<LightButton, ButtonElement<LightButton>>();
        factory.set_wrapper::<MenuButton, MenuButtonElement>();
        factory.set_wrapper::<ReadOnlyText, ReadOnlyTextElement>();
        factory.set_wrapper::<ReturnButton, ButtonElement<ReturnButton>>();
        factory.set_wrapper::<SecretInput, InputElement<SecretInput>>();
        Rc::new(factory)
    }
}

fn wrapper_factory() -> Rc<WrapperFactory> {
    WRAPPER_FACTORY.with(|factory| Rc::clone(factory))
}

fn is_table_nav_event() -> bool {
    match app::event() {
        Event::KeyDown => true,
        Event::Released => app::event_is_click(),
        _ => false,
    }
}
