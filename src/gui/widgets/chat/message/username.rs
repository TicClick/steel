use eframe::egui::{self, Color32, Widget};
use steel_core::{
    chat::Message,
    ipc::client::CoreClient,
    settings::Settings,
    string_utils::{UsernameKey, UsernameString},
    TextStyle,
};

use crate::gui::{
    context_menu::{
        chat_user::{
            menu_item_copy_message, menu_item_copy_username, menu_item_ignore_user,
            menu_item_open_chat, menu_item_open_chat_user_profile, menu_item_report_to_moderators,
            menu_item_translate_message, menu_item_unignore_user,
        },
        shared::menu_item_open_chat_log,
    },
    widgets::selectable_button::SelectableButton,
    DecoratedText,
};

pub fn choose_colour(
    key: &UsernameKey,
    own_username: &Option<String>,
    settings: &Settings,
) -> Color32 {
    let is_own_message = own_username
        .as_ref()
        .is_some_and(|u| u.as_username_key() == *key);
    let colour = match is_own_message {
        true => &settings.ui.colours().own,
        false => settings.ui.colours().username_colour(key),
    };
    colour.clone().into()
}

pub struct Username<'msg, 'app> {
    styles: Option<&'msg Vec<TextStyle>>,

    chat_name: &'msg str,

    message: &'msg Message,
    core_client: &'app CoreClient,
    settings: &'app Settings,

    #[cfg(feature = "glass")]
    glass: &'app glass::Glass,
}

impl<'msg, 'app> Username<'msg, 'app> {
    pub fn new(
        message: &'msg Message,
        chat_name: &'msg str,
        styles: Option<&'msg Vec<TextStyle>>,
        core_client: &'app CoreClient,
        settings: &'app Settings,

        #[cfg(feature = "glass")] glass: &'app glass::Glass,
    ) -> Self {
        Self {
            message,
            chat_name,
            styles,
            core_client,
            settings,

            #[cfg(feature = "glass")]
            glass,
        }
    }

    fn show_context_menu(&self, ui: &mut egui::Ui) {
        menu_item_open_chat(ui, self.core_client, true, &self.message.username_display);
        menu_item_open_chat_user_profile(ui, true, &self.message.username_display);
        menu_item_translate_message(ui, true, &self.message.text);
        menu_item_open_chat_log(ui, self.core_client, true, &self.message.username_display);
        menu_item_report_to_moderators(
            ui,
            self.core_client,
            self.settings,
            true,
            self.chat_name,
            self.message,
        );

        let is_ignored = self
            .settings
            .chat
            .ignored_users
            .iter()
            .any(|u| u == self.message.username.as_str());
        if is_ignored {
            menu_item_unignore_user(ui, self.core_client, true, self.message.username.as_str());
        } else {
            menu_item_ignore_user(ui, self.core_client, true, self.message.username.as_str());
        }

        ui.separator();

        menu_item_copy_message(ui, false, self.message);
        menu_item_copy_username(ui, false, self.message);

        #[cfg(feature = "glass")]
        self.glass.show_user_context_menu(
            ui,
            self.core_client,
            self.settings,
            self.chat_name,
            self.message,
        );
    }
}

impl Widget for Username<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let username_text =
            egui::RichText::new(&self.message.username_display).with_styles(self.styles);
        let invisible_text = format!(" <{}>", self.message.username_display);

        #[allow(unused_mut)] // glass
        let mut resp = ui.add(SelectableButton::new(username_text, invisible_text));

        #[cfg(feature = "glass")]
        if let Some(tt) = self.glass.show_user_tooltip(self.chat_name, self.message) {
            resp = resp.on_hover_text_at_pointer(tt);
        }

        if resp.clicked() {
            self.core_client
                .insert_user_mention(&self.message.username_display);
        }

        resp.context_menu(|ui| self.show_context_menu(ui));
        resp
    }
}
